use serde_yaml_ng::{Mapping, Value};

use crate::config::advanced::AdvancedConfig;
use crate::tls_fingerprint::TlsFingerprintLibrary;

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

/// Apply TLS fingerprint configuration from advanced.yaml to the Mihomo config mapping.
/// Injects `global-client-fingerprint` into the top-level config.
pub fn apply_tls_fingerprint_config(config: Mapping) -> Mapping {
    let advanced = load_advanced_config();
    apply_tls_fingerprint_config_with_advanced(config, &advanced.security.tls_fingerprint)
}

/// Apply TLS fingerprint with an explicit value.
/// If `tls_fingerprint` is Some and valid, injects `global-client-fingerprint`.
/// If None, ensures the field is absent or empty.
pub fn apply_tls_fingerprint_config_with_advanced(mut config: Mapping, tls_fingerprint: &Option<String>) -> Mapping {
    match tls_fingerprint {
        Some(name) if TlsFingerprintLibrary::is_valid(name) => {
            revise!(config, "global-client-fingerprint", name.clone());
        }
        Some(name) => {
            log::warn!("[TLS Fingerprint] Invalid fingerprint name '{}', skipping", name);
        }
        None => {
            // Remove or clear the field
            let key = Value::from("global-client-fingerprint");
            config.remove(&key);
        }
    }
    config
}
