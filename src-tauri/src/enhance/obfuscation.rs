use serde_yaml_ng::{Mapping, Value};

use crate::config::advanced::{AdvancedConfig, ObfuscationLevel};
use crate::security::ingress_countermeasure::{ThreatLevel, runtime_persona_profiles, select_persona};

macro_rules! revise {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        $map.insert(ret_key, Value::from($val));
    };
}

/// Load advanced config from coordinator
fn load_advanced_config() -> AdvancedConfig {
    crate::core::coordinator::get_coordinator().get_advanced_config()
}

/// Apply obfuscation configuration from advanced.yaml to the Mihomo config mapping.
///
/// Obfuscation affects:
/// - `global-client-fingerprint`: high/paranoid levels override to random/randomized
/// - `global-ua`: set a browser-like User-Agent when obfuscation is enabled
pub fn apply_obfuscation_config(config: Mapping) -> Mapping {
    let advanced = load_advanced_config();
    apply_obfuscation_config_for_threat(config, &advanced, ThreatLevel::Normal)
}

fn apply_obfuscation_config_for_threat(
    config: Mapping,
    advanced: &AdvancedConfig,
    threat_level: ThreatLevel,
) -> Mapping {
    let mut config = config;
    let security = &advanced.security;
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

    let personas = runtime_persona_profiles(&advanced.ingress_countermeasure.persona_profiles);
    if let Some(persona) = select_persona(threat_level, &personas) {
        if threat_level != ThreatLevel::Normal {
            revise!(config, "global-client-fingerprint", persona.tls_fingerprint.as_str());
            revise!(config, "global-ua", persona_user_agent(&persona.ua_family));
        } else if security.tls_fingerprint.is_none() && !config.contains_key("global-client-fingerprint") {
            revise!(config, "global-client-fingerprint", persona.tls_fingerprint.as_str());
        }
    }

    config
}

fn persona_user_agent(ua_family: &str) -> &'static str {
    match ua_family {
        "firefox-esr" => "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:128.0) Gecko/20100101 Firefox/128.0",
        "safari-mobile" => {
            "Mozilla/5.0 (iPhone; CPU iPhone OS 18_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.1 Mobile/15E148 Safari/604.1"
        }
        _ => {
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36"
        }
    }
}
