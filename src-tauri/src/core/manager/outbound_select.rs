//! Resolve the user's currently selected node into a learn-gripe outbound.
//!
//! The kernel data plane carries every connection through a single
//! [`OutboundMode`]. This module turns the control-plane state (the generated
//! runtime clash config plus the persisted per-group selection) into that
//! outbound, so starting the core dials the node the user picked instead of
//! always going [`OutboundMode::Direct`].
//!
//! Two egress strategies are produced from the same control-plane state:
//!
//! - [`selected_outbound`] resolves a *single global egress* — the current
//!   selection of the primary `select` proxy-group (or `GLOBAL` in global
//!   mode), following nested selector groups down to a concrete node. Groups
//!   that need active measurement or balancing (`url-test`, `fallback`,
//!   `load-balance`, `relay`, …), protocols without a data plane yet, and any
//!   resolution failure fall back to `Direct` rather than risk mis-routing.
//! - [`routed_outbound`] honours per-connection rule routing in `rule` mode:
//!   it builds a [`learn_gripe::Router`] from the runtime `rules:` list so each
//!   connection takes the outbound its first matching rule selects. It falls
//!   back to [`selected_outbound`] in `direct`/`global` mode and for any config
//!   without rules that resolve to a usable outbound.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use learn_gripe::{GeoLookup, IpCidr, OutboundMode, Router, Rule, RuleMatcher};
use serde_yaml_ng::{Mapping, Value};

/// Built-in policy names that are not real `proxies:` entries.
const DIRECT: &str = "DIRECT";
const REJECT: &str = "REJECT";

/// Resolve the outbound for the selected node, falling back to
/// [`OutboundMode::Direct`] (logging why) when nothing usable is selected.
pub fn selected_outbound(config: &Mapping, selected: &HashMap<String, String>) -> OutboundMode {
    match resolve(config, selected) {
        Ok(mode) => mode,
        Err(err) => {
            clash_verge_logging::logging!(
                info,
                clash_verge_logging::Type::Core,
                "learn-gripe outbound falls back to Direct: {err:#}"
            );
            OutboundMode::Direct
        }
    }
}

/// Resolve the outbound for the mixed inbound, honouring per-connection rule
/// routing in `rule` mode.
///
/// In `rule` mode (the clash default) this builds a [`learn_gripe::Router`]
/// from the runtime `rules:` list, so each connection takes the outbound its
/// first matching rule selects instead of forcing every connection through one
/// node. `direct`/`global` mode, and any config without rules that resolve to a
/// usable outbound, fall back to the single global egress
/// ([`selected_outbound`]).
///
/// `GEOIP` / `GEOSITE` rules are only routable when the embedder supplies a
/// [`GeoLookup`] backed by *local* geo data; the kernel never reads or fetches
/// that data itself. When `geo` is `None` (no local data) those rules are
/// skipped, exactly as any other unsupported rule type.
pub fn routed_outbound(
    config: &Mapping,
    selected: &HashMap<String, String>,
    geo: Option<Arc<dyn GeoLookup>>,
) -> OutboundMode {
    match build_router(config, selected, geo) {
        Some(router) => OutboundMode::Routed(Box::new(router)),
        None => selected_outbound(config, selected),
    }
}

/// Build a rule [`Router`] from the runtime config, or `None` when routing does
/// not apply — non-`rule` mode, no `rules:` section, or no rule resolves to a
/// usable outbound — so the caller falls back to the single global egress.
fn build_router(
    config: &Mapping,
    selected: &HashMap<String, String>,
    geo: Option<Arc<dyn GeoLookup>>,
) -> Option<Router> {
    let mode = config
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("rule")
        .to_ascii_lowercase();
    if mode != "rule" {
        return None;
    }

    let groups = group_index(config);
    let proxies = proxy_index(config);
    let raw_rules = config.get("rules").and_then(Value::as_sequence)?;

    let mut outbounds: HashMap<String, OutboundMode> = HashMap::new();
    let mut rules: Vec<Rule> = Vec::new();

    for raw in raw_rules.iter().filter_map(Value::as_str) {
        let Some((matcher, target)) = parse_router_rule(raw, geo.as_ref()) else {
            // Unsupported rule type (GEOIP/GEOSITE/process/port/…) or malformed
            // payload: skip it so it never matches, falling through to a later
            // rule or the fallback rather than mis-routing.
            continue;
        };
        match register_target(&target, &groups, &proxies, selected, &mut outbounds) {
            Ok(name) => rules.push(Rule::new(matcher, name)),
            Err(err) => {
                clash_verge_logging::logging!(
                    info,
                    clash_verge_logging::Type::Core,
                    "learn-gripe routing skips rule {raw:?}: {err:#}"
                );
            }
        }
    }

    if rules.is_empty() {
        return None;
    }

    // Fallback is `DIRECT`: real configs end with a `MATCH` rule (kept above as
    // a catch-all `RuleMatcher::Match`), so the fallback only fires for a config
    // with no `MATCH`, where clash also sends unmatched traffic direct.
    match Router::new(outbounds, rules, DIRECT) {
        Ok(router) => Some(router),
        Err(err) => {
            clash_verge_logging::logging!(
                info,
                clash_verge_logging::Type::Core,
                "learn-gripe routing disabled, falling back to single egress: {err:#}"
            );
            None
        }
    }
}

/// Parse one clash rule string into a [`RuleMatcher`] plus its target policy
/// name. Returns `None` for rule types the kernel router cannot evaluate yet
/// (`SRC-IP-CIDR`, `DST-PORT`, `PROCESS-NAME`, `RULE-SET`, logical rules, …) or
/// a malformed payload, so the caller drops the rule.
///
/// `GEOIP` / `GEOSITE` are evaluated against `geo` (a [`GeoLookup`] over local
/// geo data) when present; when `geo` is `None` they are dropped too, since
/// without local data there is nothing to match against.
fn parse_router_rule(raw: &str, geo: Option<&Arc<dyn GeoLookup>>) -> Option<(RuleMatcher, String)> {
    let mut parts = raw.split(',').map(str::trim);
    let rule_type = parts.next()?.to_ascii_uppercase();

    if rule_type == "MATCH" {
        let target = parts.next()?;
        return (!target.is_empty()).then(|| (RuleMatcher::Match, target.to_string()));
    }

    let payload = parts.next()?;
    let target = parts.next()?;
    if payload.is_empty() || target.is_empty() {
        return None;
    }
    // Any trailing modifier (e.g. `no-resolve`) does not change how the kernel
    // matches the connection target, so it is ignored.
    let matcher = match rule_type.as_str() {
        "DOMAIN" => RuleMatcher::Domain(payload.to_string()),
        "DOMAIN-SUFFIX" => RuleMatcher::DomainSuffix(payload.to_string()),
        "DOMAIN-KEYWORD" => RuleMatcher::DomainKeyword(payload.to_string()),
        "IP-CIDR" | "IP-CIDR6" => RuleMatcher::IpCidr(IpCidr::parse(payload).ok()?),
        "GEOIP" => RuleMatcher::GeoIp {
            code: payload.to_string(),
            db: Arc::clone(geo?),
        },
        "GEOSITE" => RuleMatcher::GeoSite {
            code: payload.to_string(),
            db: Arc::clone(geo?),
        },
        _ => return None,
    };
    Some((matcher, target.to_string()))
}

/// Resolve a rule's target policy (a proxy-group, node, or built-in
/// `DIRECT`/`REJECT`) to a concrete outbound, registering it under a stable
/// name the rule references. Built-ins resolve to themselves without an entry;
/// a group/node is followed to a concrete node (honouring the persisted
/// selection) and its outbound is registered once, keyed by the target name so
/// repeated references share one entry. Errors on an unresolvable/unsupported
/// target so the caller drops the rule.
fn register_target(
    target: &str,
    groups: &HashMap<String, &Mapping>,
    proxies: &HashMap<String, &Mapping>,
    selected: &HashMap<String, String>,
    outbounds: &mut HashMap<String, OutboundMode>,
) -> Result<String> {
    if target == DIRECT || target == REJECT {
        return Ok(target.to_string());
    }
    let node = resolve_node_name(target, groups, selected)?;
    if node == DIRECT || node == REJECT {
        return Ok(node);
    }
    if !outbounds.contains_key(target) {
        let mode = outbound_for_node(&node, proxies)?;
        outbounds.insert(target.to_string(), mode);
    }
    Ok(target.to_string())
}

/// Resolve the selected node to a concrete [`OutboundMode`], or return an error
/// describing why no usable node could be selected.
fn resolve(config: &Mapping, selected: &HashMap<String, String>) -> Result<OutboundMode> {
    let mode = config
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("rule")
        .to_ascii_lowercase();

    if mode == "direct" {
        return Ok(OutboundMode::Direct);
    }

    let groups = group_index(config);
    let proxies = proxy_index(config);

    let start = starting_group(&mode, config, &groups)
        .ok_or_else(|| anyhow!("no select proxy-group to resolve the egress node from"))?;

    let node = resolve_node_name(&start, &groups, selected)?;
    outbound_for_node(&node, &proxies)
}

/// Walk selector groups from `start` down to a concrete node (or a built-in
/// `DIRECT`/`REJECT` policy), honoring the persisted selection at each hop and
/// defaulting to the group's first member when nothing is selected.
fn resolve_node_name(
    start: &str,
    groups: &HashMap<String, &Mapping>,
    selected: &HashMap<String, String>,
) -> Result<String> {
    let mut name = start.to_string();
    let mut visited = HashSet::new();

    loop {
        if name == DIRECT || name == REJECT {
            return Ok(name);
        }
        // A concrete proxy (not a group) terminates the walk.
        let Some(group) = groups.get(name.as_str()) else {
            return Ok(name);
        };
        if !visited.insert(name.clone()) {
            bail!("proxy-group selection cycle at {name:?}");
        }
        if !is_selector(group) {
            let kind = group.get("type").and_then(Value::as_str).unwrap_or("unknown");
            bail!("selected group {name:?} has type {kind:?}, which has no deterministic egress");
        }

        let members = group_members(group);
        if members.is_empty() {
            bail!("select group {name:?} has no members");
        }
        name = selected
            .get(&name)
            .filter(|chosen| members.iter().any(|m| m == *chosen))
            .cloned()
            .unwrap_or_else(|| members[0].clone());
    }
}

/// Map a resolved node name to an outbound, deserializing its `proxies:` entry.
fn outbound_for_node(node: &str, proxies: &HashMap<String, &Mapping>) -> Result<OutboundMode> {
    match node {
        DIRECT => Ok(OutboundMode::Direct),
        REJECT => Ok(OutboundMode::Reject),
        _ => {
            let entry_map = proxies
                .get(node)
                .ok_or_else(|| anyhow!("selected node {node:?} is not in the proxies list"))?;
            let entry: learn_gripe::ProxyEntry = serde_yaml_ng::from_value(Value::Mapping((*entry_map).clone()))
                .map_err(|e| anyhow!("parse proxy {node:?}: {e}"))?;
            OutboundMode::from_proxy(&entry).map_err(|e| anyhow!("node {node:?}: {e:#}"))
        }
    }
}

/// The group the egress resolution starts from: `GLOBAL` in global mode (or the
/// first selector if there is no `GLOBAL`), otherwise the first selector group.
fn starting_group<'a>(mode: &str, config: &'a Mapping, groups: &HashMap<String, &'a Mapping>) -> Option<String> {
    if mode == "global"
        && let Some(global) = groups.get("GLOBAL")
        && is_selector(global)
    {
        return Some("GLOBAL".to_string());
    }
    first_selector_name(config)
}

/// Name of the first `select`/`selector` group in declaration order.
fn first_selector_name(config: &Mapping) -> Option<String> {
    config
        .get("proxy-groups")
        .and_then(Value::as_sequence)?
        .iter()
        .filter_map(Value::as_mapping)
        .find(|g| is_selector(g))
        .and_then(|g| g.get("name").and_then(Value::as_str))
        .map(str::to_string)
}

fn is_selector(group: &Mapping) -> bool {
    matches!(
        group
            .get("type")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("select") | Some("selector")
    )
}

fn group_members(group: &Mapping) -> Vec<String> {
    group
        .get("proxies")
        .and_then(Value::as_sequence)
        .map(|seq| seq.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default()
}

fn group_index(config: &Mapping) -> HashMap<String, &Mapping> {
    named_mappings(config.get("proxy-groups"))
}

fn proxy_index(config: &Mapping) -> HashMap<String, &Mapping> {
    named_mappings(config.get("proxies"))
}

/// Index a sequence of `{ name: ..., ... }` mappings by their `name`.
fn named_mappings(value: Option<&Value>) -> HashMap<String, &Mapping> {
    let mut out = HashMap::new();
    let Some(seq) = value.and_then(Value::as_sequence) else {
        return out;
    };
    for item in seq {
        if let Some(map) = item.as_mapping()
            && let Some(name) = map.get("name").and_then(Value::as_str)
        {
            out.insert(name.to_string(), map);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use learn_gripe::TargetAddr;

    fn cfg(yaml: &str) -> Mapping {
        serde_yaml_ng::from_str(yaml).expect("valid yaml mapping")
    }

    fn sel(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(g, n)| (g.to_string(), n.to_string())).collect()
    }

    fn domain(host: &str) -> TargetAddr {
        TargetAddr::Domain(host.to_string(), 443)
    }

    fn ip(addr: &str) -> TargetAddr {
        TargetAddr::Ip(addr.parse().expect("socket addr"))
    }

    /// No local geo data available (the common case in tests): `GEOIP` /
    /// `GEOSITE` rules are skipped just like any other unsupported rule.
    fn no_geo() -> Option<Arc<dyn GeoLookup>> {
        None
    }

    /// In-memory geo database for routing tests: `cn` covers `1.0.0.0/8`, and
    /// the `cn` geosite category covers any host ending in `.cn`.
    #[derive(Debug)]
    struct FakeGeo;

    impl GeoLookup for FakeGeo {
        fn geoip_matches(&self, code: &str, ip: std::net::IpAddr) -> bool {
            code == "cn" && matches!(ip, std::net::IpAddr::V4(v4) if v4.octets()[0] == 1)
        }

        fn geosite_matches(&self, code: &str, host: &str) -> bool {
            code == "cn" && (host == "cn" || host.ends_with(".cn"))
        }
    }

    fn fake_geo() -> Option<Arc<dyn GeoLookup>> {
        Some(Arc::new(FakeGeo))
    }

    const TROJAN_CFG: &str = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
  - { name: node-b, type: trojan, server: b.example, port: 443, password: pb }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, node-b, DIRECT] }
"#;

    #[test]
    fn picks_persisted_selection() {
        let mode = selected_outbound(&cfg(TROJAN_CFG), &sel(&[("PROXY", "node-b")]));
        match mode {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "b.example"),
            other => panic!("expected trojan node-b, got {other:?}"),
        }
    }

    #[test]
    fn defaults_to_first_member_without_selection() {
        let mode = selected_outbound(&cfg(TROJAN_CFG), &HashMap::new());
        match mode {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "a.example"),
            other => panic!("expected trojan node-a, got {other:?}"),
        }
    }

    #[test]
    fn direct_policy_selection_maps_to_direct() {
        let mode = selected_outbound(&cfg(TROJAN_CFG), &sel(&[("PROXY", "DIRECT")]));
        assert_eq!(mode, OutboundMode::Direct);
    }

    #[test]
    fn direct_mode_short_circuits_to_direct() {
        let yaml = TROJAN_CFG.replace("mode: rule", "mode: direct");
        let mode = selected_outbound(&cfg(&yaml), &sel(&[("PROXY", "node-b")]));
        assert_eq!(mode, OutboundMode::Direct);
    }

    #[test]
    fn follows_nested_selector_groups() {
        let yaml = r#"
mode: rule
proxies:
  - { name: ss-node, type: ss, server: s.example, port: 8388, cipher: aes-256-gcm, password: pw }
proxy-groups:
  - { name: PROXY, type: select, proxies: [SUB] }
  - { name: SUB, type: select, proxies: [ss-node] }
"#;
        let mode = selected_outbound(&cfg(yaml), &sel(&[("PROXY", "SUB"), ("SUB", "ss-node")]));
        assert!(matches!(mode, OutboundMode::Shadowsocks(_)));
    }

    #[test]
    fn global_mode_uses_global_group() {
        let yaml = r#"
mode: global
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
  - { name: node-b, type: trojan, server: b.example, port: 443, password: pb }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a] }
  - { name: GLOBAL, type: select, proxies: [node-a, node-b] }
"#;
        let mode = selected_outbound(&cfg(yaml), &sel(&[("GLOBAL", "node-b")]));
        match mode {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "b.example"),
            other => panic!("expected GLOBAL selection node-b, got {other:?}"),
        }
    }

    #[test]
    fn url_test_group_falls_back_to_direct() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: AUTO, type: url-test, proxies: [node-a] }
"#;
        // No selector group at all -> nothing to resolve -> Direct.
        assert_eq!(selected_outbound(&cfg(yaml), &HashMap::new()), OutboundMode::Direct);
    }

    #[test]
    fn selected_group_pointing_at_url_test_falls_back() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [AUTO] }
  - { name: AUTO, type: url-test, proxies: [node-a] }
"#;
        assert_eq!(
            selected_outbound(&cfg(yaml), &sel(&[("PROXY", "AUTO")])),
            OutboundMode::Direct
        );
    }

    #[test]
    fn unimplemented_protocol_falls_back_to_direct() {
        let yaml = r#"
mode: rule
proxies:
  - { name: hy, type: hysteria2, server: h.example, port: 443, password: pw }
proxy-groups:
  - { name: PROXY, type: select, proxies: [hy] }
"#;
        assert_eq!(selected_outbound(&cfg(yaml), &HashMap::new()), OutboundMode::Direct);
    }

    #[test]
    fn cycle_is_broken_with_fallback() {
        let yaml = r#"
mode: rule
proxies: []
proxy-groups:
  - { name: A, type: select, proxies: [B] }
  - { name: B, type: select, proxies: [A] }
"#;
        assert_eq!(
            selected_outbound(&cfg(yaml), &sel(&[("A", "B"), ("B", "A")])),
            OutboundMode::Direct
        );
    }

    const ROUTED_CFG: &str = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - DOMAIN-SUFFIX,ads.example,REJECT
  - DOMAIN,exact.example,DIRECT
  - DOMAIN-SUFFIX,proxied.example,PROXY
  - DOMAIN-KEYWORD,torrent,DIRECT
  - IP-CIDR,10.0.0.0/8,DIRECT
  - GEOIP,CN,DIRECT
  - MATCH,PROXY
"#;

    #[test]
    fn rule_mode_builds_router_and_routes_per_rule() {
        let mode = routed_outbound(&cfg(ROUTED_CFG), &HashMap::new(), no_geo());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        // DOMAIN-SUFFIX matches the domain itself and any subdomain.
        assert_eq!(router.select(&domain("x.ads.example")), &OutboundMode::Reject);
        // DOMAIN is an exact match only.
        assert_eq!(router.select(&domain("exact.example")), &OutboundMode::Direct);
        assert!(matches!(
            router.select(&domain("www.proxied.example")),
            OutboundMode::Trojan(_)
        ));
        assert_eq!(router.select(&domain("a-torrent-site.net")), &OutboundMode::Direct);
        assert_eq!(router.select(&ip("10.1.2.3:80")), &OutboundMode::Direct);
        // GEOIP is skipped; the MATCH catch-all sends everything else to PROXY.
        assert!(matches!(
            router.select(&domain("unmatched.net")),
            OutboundMode::Trojan(_)
        ));
    }

    #[test]
    fn non_rule_mode_uses_single_egress() {
        for mode_name in ["global", "direct"] {
            let yaml = ROUTED_CFG.replace("mode: rule", &format!("mode: {mode_name}"));
            let mode = routed_outbound(&cfg(&yaml), &HashMap::new(), no_geo());
            assert!(
                !matches!(mode, OutboundMode::Routed(_)),
                "{mode_name} mode must not route per connection, got {mode:?}"
            );
        }
    }

    #[test]
    fn no_supported_rules_falls_back_to_single_egress() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a] }
rules:
  - GEOIP,CN,DIRECT
  - GEOSITE,google,PROXY
  - PROCESS-NAME,curl,DIRECT
"#;
        // Every rule is unsupported, so routing is disabled and the single
        // global egress (PROXY's first member) is used instead.
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo());
        match mode {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "a.example"),
            other => panic!("expected single-egress trojan, got {other:?}"),
        }
    }

    #[test]
    fn missing_rules_section_falls_back_to_single_egress() {
        // `mode: rule` but no `rules:` at all -> single global egress.
        let mode = routed_outbound(&cfg(TROJAN_CFG), &sel(&[("PROXY", "node-b")]), no_geo());
        match mode {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "b.example"),
            other => panic!("expected single-egress trojan, got {other:?}"),
        }
    }

    #[test]
    fn routed_rule_target_follows_persisted_selection() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
  - { name: node-b, type: trojan, server: b.example, port: 443, password: pb }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, node-b] }
rules:
  - MATCH,PROXY
"#;
        let mode = routed_outbound(&cfg(yaml), &sel(&[("PROXY", "node-b")]), no_geo());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        match router.select(&domain("anything.net")) {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "b.example"),
            other => panic!("expected node-b trojan, got {other:?}"),
        }
    }

    #[test]
    fn rule_with_unresolvable_target_is_skipped() {
        let yaml = r#"
mode: rule
proxies:
  - { name: hy, type: hysteria2, server: h.example, port: 443, password: pw }
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a] }
rules:
  - DOMAIN-SUFFIX,blocked.example,hy
  - MATCH,PROXY
"#;
        // The hysteria2 target has no data plane yet, so that rule is dropped and
        // blocked.example falls through to the MATCH catch-all (trojan PROXY).
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        assert!(matches!(
            router.select(&domain("x.blocked.example")),
            OutboundMode::Trojan(_)
        ));
    }

    const GEO_CFG: &str = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - GEOSITE,cn,DIRECT
  - GEOIP,cn,DIRECT
  - MATCH,PROXY
"#;

    #[test]
    fn geo_rules_route_when_local_data_present() {
        let mode = routed_outbound(&cfg(GEO_CFG), &HashMap::new(), fake_geo());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        // GEOSITE,cn matches a `.cn` domain -> DIRECT.
        assert_eq!(router.select(&domain("www.example.cn")), &OutboundMode::Direct);
        // GEOIP,cn matches an IP in 1.0.0.0/8 -> DIRECT.
        assert_eq!(router.select(&ip("1.2.3.4:80")), &OutboundMode::Direct);
        // A foreign domain / IP falls through to the MATCH catch-all (PROXY).
        assert!(matches!(
            router.select(&domain("www.example.com")),
            OutboundMode::Trojan(_)
        ));
        assert!(matches!(router.select(&ip("8.8.8.8:80")), OutboundMode::Trojan(_)));
    }

    #[test]
    fn geo_rules_skipped_without_local_data() {
        // Same config but no local geo data: both geo rules are dropped, so
        // everything falls through to the MATCH catch-all (PROXY).
        let mode = routed_outbound(&cfg(GEO_CFG), &HashMap::new(), no_geo());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        assert!(matches!(
            router.select(&domain("www.example.cn")),
            OutboundMode::Trojan(_)
        ));
        assert!(matches!(router.select(&ip("1.2.3.4:80")), OutboundMode::Trojan(_)));
    }

    #[test]
    fn only_geo_rules_without_data_falls_back_to_single_egress() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a] }
rules:
  - GEOIP,cn,DIRECT
  - GEOSITE,cn,PROXY
"#;
        // No MATCH and no local geo data: every rule is dropped, routing is
        // disabled, and the single global egress is used.
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo());
        match mode {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "a.example"),
            other => panic!("expected single-egress trojan, got {other:?}"),
        }
    }
}
