use serde_yaml_ng::{Mapping, Value};

use crate::config::advanced::AdvancedConfig;
use crate::core::timezone_spoof::{NtpStrategy, get_fresh_observed_egress_region, select_ntp_server};

const OBSERVED_EGRESS_REGION_MAX_AGE_MS: u64 = 3 * 60 * 1000;

macro_rules! revise {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        $map.insert(ret_key, Value::from($val));
    };
}

fn load_advanced_config() -> AdvancedConfig {
    crate::feat::get_coordinator().get_advanced_config()
}

pub fn apply_timezone_spoof_config(mut config: Mapping) -> Mapping {
    let advanced = load_advanced_config();
    let cfg = &advanced.timezone_spoof;

    if !cfg.enabled {
        revise!(config, "ntp", build_ntp_disabled());
        return config;
    }

    let ntp_section = match &cfg.ntp_strategy {
        NtpStrategy::Disabled => {
            revise!(config, "ntp", build_ntp_disabled());
            return config;
        }
        NtpStrategy::Manual => {
            let server = cfg
                .manual_ntp_server
                .clone()
                .unwrap_or_else(|| "pool.ntp.org".to_string());
            build_ntp_enabled(
                &server,
                cfg.ntp_interval_min,
                cfg.write_to_system,
                cfg.dialer_proxy.as_deref(),
            )
        }
        NtpStrategy::Auto => {
            let server = detect_egress_ntp_server();
            build_ntp_enabled(
                &server,
                cfg.ntp_interval_min,
                cfg.write_to_system,
                cfg.dialer_proxy.as_deref(),
            )
        }
    };

    revise!(config, "ntp", ntp_section);
    config
}

fn build_ntp_enabled(server: &str, interval_min: u32, write_to_system: bool, dialer_proxy: Option<&str>) -> Value {
    let mut ntp = Mapping::new();
    revise!(ntp, "enable", true);
    revise!(ntp, "server", server.to_string());
    revise!(ntp, "port", 123);
    revise!(ntp, "interval", interval_min as i64);
    revise!(ntp, "write-to-system", write_to_system);
    if let Some(proxy) = dialer_proxy {
        revise!(ntp, "dialer-proxy", proxy.to_string());
    }
    Value::Mapping(ntp)
}

fn build_ntp_disabled() -> Value {
    let mut ntp = Mapping::new();
    revise!(ntp, "enable", false);
    Value::Mapping(ntp)
}

fn detect_egress_ntp_server() -> String {
    let Some(observed_region) = get_fresh_observed_egress_region(OBSERVED_EGRESS_REGION_MAX_AGE_MS) else {
        log::warn!("[TimezoneSpoof] Auto mode has no fresh observed egress region, fallback to global NTP pool");
        return "pool.ntp.org".to_string();
    };

    let server = select_ntp_server(&observed_region.country_code);
    log::info!(
        "[TimezoneSpoof] Auto mode uses observed egress region {} / {} from {}, selected NTP server {}",
        observed_region.country_code,
        observed_region.timezone,
        observed_region.source,
        server
    );
    server
}
