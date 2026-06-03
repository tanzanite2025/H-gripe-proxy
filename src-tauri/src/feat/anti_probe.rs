use std::net::IpAddr;

use crate::anti_probe::AntiProbeConfig;
use crate::security::ingress_countermeasure::ThreatReason;

pub fn anti_probe_get_config() -> AntiProbeConfig {
    let coordinator = crate::feat::get_coordinator();
    let service = coordinator.anti_probe();
    service.get_config()
}

pub fn anti_probe_verify_handshake(ip: &IpAddr, token: &str) -> bool {
    let coordinator = crate::feat::get_coordinator();
    let service = coordinator.anti_probe();
    let verified = service.verify_handshake(ip, token);

    if !verified {
        let ingress_countermeasure = coordinator.ingress_countermeasure();
        let source = ip.to_string();
        tauri::async_runtime::spawn(async move {
            ingress_countermeasure
                .record_signal(source, ThreatReason::AntiProbeFailure)
                .await;
        });
    }

    verified
}

pub fn anti_probe_generate_token() -> String {
    let coordinator = crate::feat::get_coordinator();
    let service = coordinator.anti_probe();
    service.generate_token()
}

pub fn anti_probe_cleanup() {
    let coordinator = crate::feat::get_coordinator();
    let service = coordinator.anti_probe();
    service.cleanup_expired();
}
