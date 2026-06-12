use serde_yaml_ng::{Mapping, Value};

use crate::config::advanced::{AdvancedConfig, SnifferConfig};

macro_rules! revise {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        $map.insert(ret_key, Value::from($val));
    };
}

/// Load advanced config from coordinator (same pattern as stable_egress.rs)
fn load_advanced_config() -> AdvancedConfig {
    crate::core::coordinator::get_coordinator().get_advanced_config()
}

/// Apply sniffer configuration from advanced.yaml to the Mihomo config mapping.
/// This is the main entry point called from the enhance pipeline.
pub fn apply_sniffer_config(config: Mapping) -> Mapping {
    let advanced = load_advanced_config();
    apply_sniffer_config_with_advanced(config, &advanced.security.sniffer)
}

/// Apply sniffer configuration with an explicit SnifferConfig reference.
/// Injects the `sniffer:` YAML section based on the user's SnifferConfig.
pub fn apply_sniffer_config_with_advanced(mut config: Mapping, sniffer_cfg: &SnifferConfig) -> Mapping {
    if !sniffer_cfg.enabled {
        // Ensure sniffer is disabled
        let sniffer_key = Value::from("sniffer");
        let mut sniffer_val = config
            .get(&sniffer_key)
            .and_then(|v| v.as_mapping().cloned())
            .unwrap_or_else(Mapping::new);
        revise!(sniffer_val, "enable", false);
        revise!(config, "sniffer", sniffer_val);
        return config;
    }

    let mut sniffer_val = Mapping::new();

    revise!(sniffer_val, "enable", true);
    revise!(sniffer_val, "override-destination", sniffer_cfg.override_dest);
    revise!(sniffer_val, "parse-pure-ip", sniffer_cfg.parse_pure_ip);
    revise!(sniffer_val, "force-dns-mapping", sniffer_cfg.force_dns_mapping);

    // force-domain
    if !sniffer_cfg.force_domain.is_empty() {
        let domains: Vec<Value> = sniffer_cfg
            .force_domain
            .iter()
            .map(|d| Value::String(d.clone()))
            .collect();
        revise!(sniffer_val, "force-domain", domains);
    }

    // skip-domain
    if !sniffer_cfg.skip_domain.is_empty() {
        let domains: Vec<Value> = sniffer_cfg
            .skip_domain
            .iter()
            .map(|d| Value::String(d.clone()))
            .collect();
        revise!(sniffer_val, "skip-domain", domains);
    }

    // sniff types — build the `sniff` map with per-type override-destination
    let mut sniff_map = Mapping::new();
    for sniff_type in &sniffer_cfg.sniffing {
        let type_key = Value::String(sniff_type.to_uppercase());
        let mut type_cfg = Mapping::new();
        revise!(type_cfg, "override-destination", sniffer_cfg.override_dest);
        sniff_map.insert(type_key, Value::Mapping(type_cfg));
    }
    if !sniff_map.is_empty() {
        revise!(sniffer_val, "sniff", sniff_map);
    }

    revise!(config, "sniffer", sniffer_val);
    config
}
