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
use learn_gripe::{GeoLookup, IpCidr, LogicalOp, OutboundMode, PortRange, Router, Rule, RuleMatcher, RuleSetLookup};
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
/// `GEOIP` / `GEOSITE` / `IP-ASN` rules are only routable when the embedder
/// supplies a [`GeoLookup`] backed by *local* geo data, and `RULE-SET` rules
/// when it supplies a [`RuleSetLookup`] backed by the locally-loaded rule
/// providers; the kernel never reads or fetches that data itself. When the
/// corresponding lookup is `None` those rules are skipped, exactly as any
/// other unsupported rule type.
pub fn routed_outbound(
    config: &Mapping,
    selected: &HashMap<String, String>,
    geo: Option<Arc<dyn GeoLookup>>,
    rule_sets: Option<Arc<dyn RuleSetLookup>>,
) -> OutboundMode {
    match build_router(config, selected, geo, rule_sets) {
        Some(router) => OutboundMode::Routed(Box::new(router)),
        None => selected_outbound(config, selected),
    }
}

/// Resolve a single policy name (a `proxies:` node, a selector proxy-group
/// followed to its selected node, or a built-in `DIRECT`/`REJECT`) to the
/// concrete [`OutboundMode`] a delay probe should dial.
///
/// This is the delay-measurement counterpart of [`selected_outbound`]: the
/// control plane hands it the name the UI wants tested (matching the Mihomo
/// `/proxies/{name}/delay` semantics, where a group name tests its selected
/// node), and gets back the outbound to time.
pub fn outbound_for_proxy(config: &Mapping, selected: &HashMap<String, String>, name: &str) -> Result<OutboundMode> {
    let groups = group_index(config);
    let proxies = proxy_index(config);
    let node = resolve_node_name(name, &groups, selected)?;
    outbound_for_node(&node, &proxies)
}

/// Resolve every member of a selector proxy-group to `(member_name, outbound)`
/// pairs the control plane can probe to fill a group delay test (the in-process
/// replacement for the Mihomo `/group/{name}/delay` call).
///
/// Each listed member is keyed by the name the UI references and resolved to a
/// concrete node (sub-selectors are followed to their selected node). Members
/// that resolve to `REJECT`, to an unselectable sub-group (`url-test`,
/// `load-balance`, …), or to a protocol without a data plane are dropped — they
/// have no measurable delay — rather than failing the whole group. Errors only
/// when the group is missing or empty.
pub fn group_member_outbounds(
    config: &Mapping,
    selected: &HashMap<String, String>,
    group_name: &str,
) -> Result<Vec<(String, OutboundMode)>> {
    let groups = group_index(config);
    let proxies = proxy_index(config);
    let group = groups
        .get(group_name)
        .ok_or_else(|| anyhow!("proxy-group {group_name:?} not found"))?;
    let members = group_members(group);
    if members.is_empty() {
        bail!("proxy-group {group_name:?} has no members");
    }

    let outbounds = members
        .into_iter()
        .filter_map(|member| {
            let node = resolve_node_name(&member, &groups, selected).ok()?;
            if node == REJECT {
                return None;
            }
            let mode = outbound_for_node(&node, &proxies).ok()?;
            Some((member, mode))
        })
        .collect();
    Ok(outbounds)
}

/// Build a rule [`Router`] from the runtime config, or `None` when routing does
/// not apply — non-`rule` mode, no `rules:` section, or no rule resolves to a
/// usable outbound — so the caller falls back to the single global egress.
fn build_router(
    config: &Mapping,
    selected: &HashMap<String, String>,
    geo: Option<Arc<dyn GeoLookup>>,
    rule_sets: Option<Arc<dyn RuleSetLookup>>,
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
        let Some((matcher, target)) = parse_router_rule(raw, geo.as_ref(), rule_sets.as_ref()) else {
            // Unsupported rule type (process/port/logical/…), a geo or
            // rule-set rule with no local data, or a malformed payload: skip
            // it so it never matches, falling through to a later rule or the
            // fallback rather than mis-routing.
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
/// (`SRC-IP-CIDR`, `SRC-IP-ASN`, `SRC-PORT`, `PROCESS-NAME`, `NETWORK`, …) or a
/// malformed payload, so the caller drops the rule. `DST-PORT` accepts a single
/// port (`443`) or an inclusive range (`8000-9000`); `SRC-PORT` stays
/// unsupported since the matcher only sees the destination target.
///
/// Logical rules `AND` / `OR` / `NOT` are supported: their parenthesized
/// sub-rules are parsed recursively (so they nest), e.g.
/// `AND,((DOMAIN-SUFFIX,example.com),(IP-CIDR,10.0.0.0/8)),DIRECT`. If any
/// sub-rule is malformed or uses a type the kernel cannot evaluate, the whole
/// logical rule is dropped rather than silently ignoring a condition.
///
/// `GEOIP` / `GEOSITE` / `IP-ASN` are evaluated against `geo` (a [`GeoLookup`]
/// over local geo data) and `RULE-SET` against `rule_sets` (a [`RuleSetLookup`]
/// over the locally-loaded rule providers); when the matching lookup is `None`
/// those rules are dropped, since without local data there is nothing to match
/// against.
fn parse_router_rule(
    raw: &str,
    geo: Option<&Arc<dyn GeoLookup>>,
    rule_sets: Option<&Arc<dyn RuleSetLookup>>,
) -> Option<(RuleMatcher, String)> {
    let trimmed = raw.trim();
    let rule_type = trimmed.split(',').next()?.trim().to_ascii_uppercase();

    if rule_type == "MATCH" {
        let target = trimmed.split(',').nth(1)?.trim();
        return (!target.is_empty()).then(|| (RuleMatcher::Match, target.to_string()));
    }

    // Every other rule is a matcher followed by its target (and optional
    // modifiers): parse the matcher, then take the target from the tail it
    // leaves behind.
    let (matcher, tail) = parse_matcher(trimmed, geo, rule_sets)?;
    let target = tail.trim_start_matches(',').split(',').next()?.trim();
    (!target.is_empty()).then(|| (matcher, target.to_string()))
}

/// Parse a matcher spec — a rule string without its target policy — into a
/// [`RuleMatcher`], returning the matcher and the unconsumed tail (the text
/// after the matcher's own fields, starting with the `,` separator, or empty).
/// The tail carries a top-level rule's target (and any modifier) or a logical
/// sub-rule's modifiers (e.g. `no-resolve`), which never affect matching.
/// Returns `None` for an unsupported rule type or a malformed payload.
fn parse_matcher<'a>(
    spec: &'a str,
    geo: Option<&Arc<dyn GeoLookup>>,
    rule_sets: Option<&Arc<dyn RuleSetLookup>>,
) -> Option<(RuleMatcher, &'a str)> {
    let spec = spec.trim();
    let comma = spec.find(',')?;
    let rule_type = spec[..comma].trim().to_ascii_uppercase();
    let rest = &spec[comma + 1..];

    if let Some(op) = logical_op(&rule_type) {
        // The matcher's payload is the parenthesized sub-rule group; the target
        // (if any) follows it.
        let (group, tail) = split_paren_group(rest.trim_start())?;
        let matcher = parse_logical(op, group, geo, rule_sets)?;
        return Some((matcher, tail));
    }

    let (payload, tail) = match rest.find(',') {
        Some(i) => (rest[..i].trim(), &rest[i..]),
        None => (rest.trim(), ""),
    };
    if payload.is_empty() {
        return None;
    }
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
        "IP-ASN" => RuleMatcher::Asn {
            asn: payload.parse().ok()?,
            db: Arc::clone(geo?),
        },
        "RULE-SET" => RuleMatcher::RuleSet {
            name: payload.to_string(),
            provider: Arc::clone(rule_sets?),
        },
        "DST-PORT" => RuleMatcher::DstPort(PortRange::parse(payload).ok()?),
        _ => return None,
    };
    Some((matcher, tail))
}

/// The [`LogicalOp`] a rule type names, or `None` for a non-logical type.
fn logical_op(rule_type: &str) -> Option<LogicalOp> {
    match rule_type {
        "AND" => Some(LogicalOp::And),
        "OR" => Some(LogicalOp::Or),
        "NOT" => Some(LogicalOp::Not),
        _ => None,
    }
}

/// Parse a logical rule's sub-rule group (`group` includes its outer parens,
/// e.g. `((DOMAIN,a.com),(IP-CIDR,10.0.0.0/8))`) into a
/// [`RuleMatcher::Logical`]. Each sub-rule is parsed recursively, so logical
/// rules nest. Returns `None` if the group is malformed, if any sub-rule is
/// malformed or uses a type the kernel cannot evaluate (so a condition is never
/// silently dropped, which would change the combined result), or — for `NOT` —
/// if there is not exactly one sub-rule.
fn parse_logical(
    op: LogicalOp,
    group: &str,
    geo: Option<&Arc<dyn GeoLookup>>,
    rule_sets: Option<&Arc<dyn RuleSetLookup>>,
) -> Option<RuleMatcher> {
    let mut subs = Vec::new();
    let mut rest = strip_outer_parens(group)?.trim();
    while !rest.is_empty() {
        let (sub_group, after) = split_paren_group(rest)?;
        let (matcher, _modifiers) = parse_matcher(strip_outer_parens(sub_group)?, geo, rule_sets)?;
        subs.push(matcher);
        rest = after.trim_start();
        match rest.strip_prefix(',') {
            Some(next) => rest = next.trim_start(),
            // Anything other than a separator between sub-rule groups (or the
            // end) is malformed.
            None if !rest.is_empty() => return None,
            None => {}
        }
    }
    if subs.is_empty() || (op == LogicalOp::Not && subs.len() != 1) {
        return None;
    }
    Some(RuleMatcher::Logical { op, subs })
}

/// Split a string that begins (after leading spaces) with `(` into its balanced
/// parenthesized prefix (parens included) and the unconsumed tail. Returns
/// `None` when it does not start with `(` or the parentheses are unbalanced.
fn split_paren_group(s: &str) -> Option<(&str, &str)> {
    let s = s.trim_start();
    if !s.starts_with('(') {
        return None;
    }
    let mut depth = 0u32;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some((&s[..=i], &s[i + ch.len_utf8()..]));
                }
            }
            _ => {}
        }
    }
    None
}

/// Strip one pair of wrapping parentheses from `s` (trimmed). Intended for a
/// group already validated by [`split_paren_group`].
fn strip_outer_parens(s: &str) -> Option<&str> {
    let s = s.trim();
    s.strip_prefix('(')?.strip_suffix(')')
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

    /// In-memory geo database for routing tests: `cn` covers `1.0.0.0/8`, the
    /// `cn` geosite category covers any host ending in `.cn`, and AS13335
    /// covers `1.0.0.0/8`.
    #[derive(Debug)]
    struct FakeGeo;

    impl GeoLookup for FakeGeo {
        fn geoip_matches(&self, code: &str, ip: std::net::IpAddr) -> bool {
            code == "cn" && matches!(ip, std::net::IpAddr::V4(v4) if v4.octets()[0] == 1)
        }

        fn geosite_matches(&self, code: &str, host: &str) -> bool {
            code == "cn" && (host == "cn" || host.ends_with(".cn"))
        }

        fn asn_matches(&self, asn: u32, ip: std::net::IpAddr) -> bool {
            asn == 13335 && matches!(ip, std::net::IpAddr::V4(v4) if v4.octets()[0] == 1)
        }
    }

    fn fake_geo() -> Option<Arc<dyn GeoLookup>> {
        Some(Arc::new(FakeGeo))
    }

    /// In-memory rule-set provider for routing tests: the `ads` set matches any
    /// host ending in `.ads.example` and any IP in `9.9.9.0/24` (a mixed set);
    /// every other set name matches nothing.
    #[derive(Debug)]
    struct FakeRuleSet;

    impl RuleSetLookup for FakeRuleSet {
        fn rule_set_matches(&self, name: &str, target: &TargetAddr) -> bool {
            if name != "ads" {
                return false;
            }
            match target {
                TargetAddr::Domain(host, _) => host == "ads.example" || host.ends_with(".ads.example"),
                TargetAddr::Ip(addr) => matches!(addr.ip(), std::net::IpAddr::V4(v4) if v4.octets()[..3] == [9, 9, 9]),
            }
        }
    }

    fn fake_rule_sets() -> Option<Arc<dyn RuleSetLookup>> {
        Some(Arc::new(FakeRuleSet))
    }

    /// No local rule-provider data available (the common case in tests):
    /// `RULE-SET` rules are skipped just like any other unsupported rule.
    fn no_rule_sets() -> Option<Arc<dyn RuleSetLookup>> {
        None
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
        let mode = routed_outbound(&cfg(ROUTED_CFG), &HashMap::new(), no_geo(), no_rule_sets());
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
            let mode = routed_outbound(&cfg(&yaml), &HashMap::new(), no_geo(), no_rule_sets());
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
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
        match mode {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "a.example"),
            other => panic!("expected single-egress trojan, got {other:?}"),
        }
    }

    #[test]
    fn missing_rules_section_falls_back_to_single_egress() {
        // `mode: rule` but no `rules:` at all -> single global egress.
        let mode = routed_outbound(&cfg(TROJAN_CFG), &sel(&[("PROXY", "node-b")]), no_geo(), no_rule_sets());
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
        let mode = routed_outbound(&cfg(yaml), &sel(&[("PROXY", "node-b")]), no_geo(), no_rule_sets());
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
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
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
        let mode = routed_outbound(&cfg(GEO_CFG), &HashMap::new(), fake_geo(), no_rule_sets());
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
        let mode = routed_outbound(&cfg(GEO_CFG), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        assert!(matches!(
            router.select(&domain("www.example.cn")),
            OutboundMode::Trojan(_)
        ));
        assert!(matches!(router.select(&ip("1.2.3.4:80")), OutboundMode::Trojan(_)));
    }

    const ASN_CFG: &str = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - IP-ASN,13335,DIRECT
  - MATCH,PROXY
"#;

    #[test]
    fn asn_rule_routes_when_local_data_present() {
        let mode = routed_outbound(&cfg(ASN_CFG), &HashMap::new(), fake_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        // IP-ASN,13335 matches an IP in AS13335 (1.0.0.0/8) -> DIRECT.
        assert_eq!(router.select(&ip("1.2.3.4:80")), &OutboundMode::Direct);
        // A foreign IP falls through to the MATCH catch-all (PROXY).
        assert!(matches!(router.select(&ip("8.8.8.8:80")), OutboundMode::Trojan(_)));
        // A domain target never matches an IP-ASN rule -> MATCH catch-all.
        assert!(matches!(
            router.select(&domain("www.example.cn")),
            OutboundMode::Trojan(_)
        ));
    }

    #[test]
    fn asn_rule_skipped_without_local_data() {
        // Same config but no local geo data: the IP-ASN rule is dropped, so an
        // AS13335 IP falls through to the MATCH catch-all (PROXY).
        let mode = routed_outbound(&cfg(ASN_CFG), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        assert!(matches!(router.select(&ip("1.2.3.4:80")), OutboundMode::Trojan(_)));
    }

    #[test]
    fn malformed_asn_rule_is_skipped() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - IP-ASN,not-a-number,DIRECT
  - MATCH,PROXY
"#;
        // The ASN payload does not parse, so the rule is dropped and every
        // target falls through to the MATCH catch-all (PROXY).
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), fake_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        assert!(matches!(router.select(&ip("1.2.3.4:80")), OutboundMode::Trojan(_)));
    }

    const RULE_SET_CFG: &str = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - RULE-SET,ads,REJECT
  - MATCH,PROXY
"#;

    #[test]
    fn rule_set_rule_routes_when_provider_data_present() {
        let mode = routed_outbound(&cfg(RULE_SET_CFG), &HashMap::new(), no_geo(), fake_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        // The `ads` set is mixed: a member domain and a member IP both reject.
        assert_eq!(router.select(&domain("x.ads.example")), &OutboundMode::Reject);
        assert_eq!(router.select(&ip("9.9.9.9:80")), &OutboundMode::Reject);
        // Non-members fall through to the MATCH catch-all (PROXY).
        assert!(matches!(
            router.select(&domain("www.example.com")),
            OutboundMode::Trojan(_)
        ));
        assert!(matches!(router.select(&ip("8.8.8.8:80")), OutboundMode::Trojan(_)));
    }

    #[test]
    fn rule_set_rule_skipped_without_provider_data() {
        // Same config but no rule-provider data: the RULE-SET rule is dropped,
        // so a set member falls through to the MATCH catch-all (PROXY).
        let mode = routed_outbound(&cfg(RULE_SET_CFG), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        assert!(matches!(
            router.select(&domain("x.ads.example")),
            OutboundMode::Trojan(_)
        ));
        assert!(matches!(router.select(&ip("9.9.9.9:80")), OutboundMode::Trojan(_)));
    }

    #[test]
    fn only_rule_set_rule_without_data_falls_back_to_single_egress() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a] }
rules:
  - RULE-SET,ads,REJECT
"#;
        // No MATCH and no provider data: every rule is dropped, routing is
        // disabled, and the single global egress is used.
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
        match mode {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "a.example"),
            other => panic!("expected single-egress trojan, got {other:?}"),
        }
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
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
        match mode {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "a.example"),
            other => panic!("expected single-egress trojan, got {other:?}"),
        }
    }

    const LOGICAL_CFG: &str = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - AND,((DOMAIN-SUFFIX,example.com),(DOMAIN-KEYWORD,ads)),REJECT
  - OR,((DOMAIN-SUFFIX,test.cn),(IP-CIDR,10.0.0.0/8)),DIRECT
  - MATCH,PROXY
"#;

    #[test]
    fn logical_and_or_rules_route() {
        let mode = routed_outbound(&cfg(LOGICAL_CFG), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        // AND: both the suffix and the keyword must hold.
        assert_eq!(router.select(&domain("ads.example.com")), &OutboundMode::Reject);
        // Suffix matches but no `ads` keyword -> AND misses, OR misses, MATCH.
        assert!(matches!(
            router.select(&domain("www.example.com")),
            OutboundMode::Trojan(_)
        ));
        // OR: either branch suffices (the domain branch, then the IP branch).
        assert_eq!(router.select(&domain("foo.test.cn")), &OutboundMode::Direct);
        assert_eq!(router.select(&ip("10.1.2.3:80")), &OutboundMode::Direct);
        // Neither logical rule matches -> MATCH catch-all (PROXY -> node-a).
        assert!(matches!(router.select(&ip("8.8.8.8:80")), OutboundMode::Trojan(_)));
    }

    #[test]
    fn logical_not_rule_routes() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - NOT,((DOMAIN-SUFFIX,example.com)),REJECT
  - MATCH,PROXY
"#;
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        // NOT inverts: an `example.com` host is excluded (falls to MATCH),
        // everything else is rejected.
        assert!(matches!(
            router.select(&domain("www.example.com")),
            OutboundMode::Trojan(_)
        ));
        assert_eq!(router.select(&domain("other.net")), &OutboundMode::Reject);
    }

    #[test]
    fn nested_logical_rule_routes() {
        // AND( suffix example.com, NOT( keyword safe ) )
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - AND,((DOMAIN-SUFFIX,example.com),(NOT,((DOMAIN-KEYWORD,safe)))),REJECT
  - MATCH,PROXY
"#;
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        // Under example.com and not a `safe` host -> reject.
        assert_eq!(router.select(&domain("ads.example.com")), &OutboundMode::Reject);
        // The inner NOT excludes `safe` hosts -> falls to MATCH.
        assert!(matches!(
            router.select(&domain("safe.example.com")),
            OutboundMode::Trojan(_)
        ));
        // Outside example.com the outer AND misses -> MATCH.
        assert!(matches!(
            router.select(&domain("ads.other.net")),
            OutboundMode::Trojan(_)
        ));
    }

    #[test]
    fn logical_rule_with_unsupported_sub_rule_is_dropped() {
        // `NETWORK` is not a matcher the kernel can evaluate, so the whole
        // logical rule is dropped rather than silently ignoring that condition.
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - AND,((DOMAIN-SUFFIX,example.com),(NETWORK,UDP)),REJECT
  - MATCH,PROXY
"#;
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        // A host that would have matched the suffix is not rejected: the rule
        // was dropped, so it falls through to MATCH (PROXY -> node-a).
        assert!(matches!(
            router.select(&domain("ads.example.com")),
            OutboundMode::Trojan(_)
        ));
    }

    #[test]
    fn logical_rule_with_geo_sub_rule_needs_local_data() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - OR,((GEOIP,cn),(DOMAIN-SUFFIX,example.com)),DIRECT
  - MATCH,PROXY
"#;
        // Without geo data the GEOIP sub-rule cannot be built, so the whole
        // logical rule is dropped: even the example.com branch falls to MATCH.
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        assert!(matches!(
            router.select(&domain("www.example.com")),
            OutboundMode::Trojan(_)
        ));

        // With geo data present both branches route DIRECT.
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), fake_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        assert_eq!(router.select(&domain("www.example.com")), &OutboundMode::Direct);
        assert_eq!(router.select(&ip("1.2.3.4:80")), &OutboundMode::Direct);
    }

    #[test]
    fn dst_port_rules_route_single_and_range() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - DST-PORT,80,DIRECT
  - DST-PORT,8000-9000,REJECT
  - MATCH,PROXY
"#;
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        // Single port 80 -> DIRECT.
        assert_eq!(router.select(&ip("8.8.8.8:80")), &OutboundMode::Direct);
        // Inclusive range 8000-9000 -> REJECT (both bounds and inside).
        assert_eq!(router.select(&ip("8.8.8.8:8000")), &OutboundMode::Reject);
        assert_eq!(router.select(&ip("8.8.8.8:9000")), &OutboundMode::Reject);
        assert_eq!(
            router.select(&TargetAddr::Domain("h.example".to_string(), 8443)),
            &OutboundMode::Reject
        );
        // A port outside every rule -> MATCH catch-all (PROXY -> node-a).
        assert!(matches!(router.select(&ip("8.8.8.8:443")), OutboundMode::Trojan(_)));
    }

    #[test]
    fn malformed_or_src_port_rules_are_dropped() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, DIRECT] }
rules:
  - SRC-PORT,80,DIRECT
  - DST-PORT,99999,DIRECT
  - DST-PORT,abc,DIRECT
  - MATCH,PROXY
"#;
        // SRC-PORT is unsupported and the two DST-PORT payloads are malformed
        // (out of u16 range / non-numeric), so all three are dropped and every
        // connection falls through to MATCH.
        let mode = routed_outbound(&cfg(yaml), &HashMap::new(), no_geo(), no_rule_sets());
        let OutboundMode::Routed(router) = mode else {
            panic!("expected Routed, got {mode:?}");
        };
        assert!(matches!(router.select(&ip("8.8.8.8:80")), OutboundMode::Trojan(_)));
    }

    #[test]
    fn outbound_for_proxy_resolves_a_node_a_group_and_builtins() {
        let config = cfg(TROJAN_CFG);
        let selected = sel(&[("PROXY", "node-b")]);

        // A concrete node name maps straight to its outbound.
        match outbound_for_proxy(&config, &selected, "node-a").unwrap() {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "a.example"),
            other => panic!("expected node-a trojan, got {other:?}"),
        }
        // A group name follows the persisted selection to the chosen node.
        match outbound_for_proxy(&config, &selected, "PROXY").unwrap() {
            OutboundMode::Trojan(c) => assert_eq!(c.server, "b.example"),
            other => panic!("expected selected node-b trojan, got {other:?}"),
        }
        // Built-in policies resolve without a proxies entry.
        assert_eq!(
            outbound_for_proxy(&config, &HashMap::new(), "DIRECT").unwrap(),
            OutboundMode::Direct
        );
        assert_eq!(
            outbound_for_proxy(&config, &HashMap::new(), "REJECT").unwrap(),
            OutboundMode::Reject
        );
        // An unknown node name errors so the caller can report it.
        assert!(outbound_for_proxy(&config, &HashMap::new(), "ghost").is_err());
    }

    #[test]
    fn group_member_outbounds_expands_members_and_drops_unmeasurable() {
        let yaml = r#"
mode: rule
proxies:
  - { name: node-a, type: trojan, server: a.example, port: 443, password: pa }
  - { name: node-b, type: ss, server: b.example, port: 8388, cipher: aes-256-gcm, password: pb }
  - { name: hy, type: hysteria2, server: h.example, port: 443, password: pw }
proxy-groups:
  - { name: PROXY, type: select, proxies: [node-a, node-b, hy, DIRECT, REJECT] }
"#;
        let members = group_member_outbounds(&cfg(yaml), &HashMap::new(), "PROXY").unwrap();
        let by_name: HashMap<_, _> = members.into_iter().collect();

        // Supported nodes and DIRECT are measurable; hysteria2 (no data plane)
        // and REJECT (never connects) are dropped.
        assert!(matches!(by_name.get("node-a"), Some(OutboundMode::Trojan(_))));
        assert!(matches!(by_name.get("node-b"), Some(OutboundMode::Shadowsocks(_))));
        assert_eq!(by_name.get("DIRECT"), Some(&OutboundMode::Direct));
        assert!(!by_name.contains_key("hy"));
        assert!(!by_name.contains_key("REJECT"));
    }

    #[test]
    fn group_member_outbounds_errors_for_missing_group() {
        assert!(group_member_outbounds(&cfg(TROJAN_CFG), &HashMap::new(), "NOPE").is_err());
    }
}
