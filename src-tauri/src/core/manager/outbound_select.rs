//! Resolve the user's currently selected node into a learn-gripe outbound.
//!
//! The kernel data plane carries every connection through a single
//! [`OutboundMode`]. This module turns the control-plane state (the generated
//! runtime clash config plus the persisted per-group selection) into that
//! outbound, so starting the core dials the node the user picked instead of
//! always going [`OutboundMode::Direct`].
//!
//! Scope: this resolves a *single global egress* — the current selection of the
//! primary `select` proxy-group (or `GLOBAL` in global mode), following nested
//! selector groups down to a concrete node. Groups that need active measurement
//! or balancing (`url-test`, `fallback`, `load-balance`, `relay`, …), protocols
//! without a data plane yet, and any resolution failure fall back to `Direct`
//! rather than risk mis-routing. Per-connection rule-based routing
//! (`OutboundMode::Routed`) is intentionally out of scope here.

use std::collections::{HashMap, HashSet};

use anyhow::{Result, anyhow, bail};
use learn_gripe::OutboundMode;
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

    fn cfg(yaml: &str) -> Mapping {
        serde_yaml_ng::from_str(yaml).expect("valid yaml mapping")
    }

    fn sel(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs.iter().map(|(g, n)| (g.to_string(), n.to_string())).collect()
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
}
