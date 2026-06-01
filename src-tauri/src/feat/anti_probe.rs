use std::net::IpAddr;

use crate::anti_probe::AntiProbeConfig;

pub fn anti_probe_get_config() -> AntiProbeConfig {
    let coordinator = crate::feat::get_coordinator();
    let service = coordinator.anti_probe();
    service.get_config()
}

pub fn anti_probe_verify_handshake(ip: &IpAddr, token: &str) -> bool {
    let coordinator = crate::feat::get_coordinator();
    let service = coordinator.anti_probe();
    service.verify_handshake(ip, token)
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
