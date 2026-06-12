use std::net::IpAddr;

use crate::security::ingress_countermeasure::ThreatReason;

pub fn anti_probe_verify_handshake(ip: &IpAddr, token: &str) -> bool {
    let coordinator = crate::core::coordinator::get_coordinator();
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
