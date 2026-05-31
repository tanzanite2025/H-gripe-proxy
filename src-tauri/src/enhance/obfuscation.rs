use serde_yaml_ng::{Mapping, Value};

use crate::config::advanced::{ObfuscationLevel, AdvancedConfig};

macro_rules! revise {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        $map.insert(ret_key, Value::from($val));
    };
}

/// Load advanced config from coordinator
fn load_advanced_config() -> AdvancedConfig {
    crate::feat::get_coordinator().get_advanced_config()
}

/// Apply obfuscation configuration from advanced.yaml to the Mihomo config mapping.
///
/// Obfuscation affects:
/// - `global-client-fingerprint`: high/paranoid levels override to random/randomized
/// - `global-ua`: set a browser-like User-Agent when obfuscation is enabled
pub fn apply_obfuscation_config(config: Mapping) -> Mapping {
    let advanced = load_advanced_config();
    apply_obfuscation_config_with_advanced(config, &advanced.security)
}

/// Apply obfuscation with an explicit SecurityConfig reference.
pub fn apply_obfuscation_config_with_advanced(
    mut config: Mapping,
    security: &crate::config::advanced::SecurityConfig,
) -> Mapping {
    let obf = &security.obfuscation;

    if !obf.enabled {
        return config;
    }

    // When obfuscation is enabled at high/paranoid level, override TLS fingerprint
    // to random/randomized if the user hasn't explicitly chosen one
    match obf.level {
        ObfuscationLevel::High => {
            // Only override if no explicit fingerprint is set or it's not already random
            if security.tls_fingerprint.is_none() {
                revise!(config, "global-client-fingerprint", "random");
            }
        }
        ObfuscationLevel::Paranoid => {
            if security.tls_fingerprint.is_none() {
                revise!(config, "global-client-fingerprint", "randomized");
            }
        }
        _ => {}
    }

    // Set a browser-like global User-Agent when obfuscation is enabled
    match obf.level {
        ObfuscationLevel::Medium | ObfuscationLevel::High | ObfuscationLevel::Paranoid => {
            revise!(
                config,
                "global-ua",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
            );
        }
        _ => {}
    }

    config
}
