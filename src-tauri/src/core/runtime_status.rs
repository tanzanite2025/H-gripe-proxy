use smartstring::alias::String;

#[derive(Debug, Clone, serde::Serialize)]
pub struct DnsRuntimeSnapshot {
    pub enhanced_mode: Option<String>,
    pub ipv6: Option<bool>,
    pub nameserver_count: usize,
    pub fallback_count: usize,
    pub nameserver_policy_count: usize,
    pub use_hosts: Option<bool>,
    pub use_system_hosts: Option<bool>,
    pub respect_rules: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DnsRuntimeDerivedState {
    pub routing_mode: Option<String>,
    pub domestic_dns: Vec<String>,
    pub foreign_dns: Vec<String>,
    pub default_nameserver_count: usize,
    pub prefer_h3: Option<bool>,
    pub leak_protection_level: Option<String>,
    pub leak_protection_security: Option<String>,
    pub leak_protection_safe: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DnsRuntimeStatus {
    pub enable_dns_settings: bool,
    pub dns_config_exists: bool,
    pub dns_config_valid: bool,
    pub runtime_has_dns: bool,
    pub runtime_has_hosts: bool,
    pub runtime_dns_matches_saved: bool,
    pub runtime_hosts_matches_saved: bool,
    pub runtime_matches_saved: bool,
    pub snapshot: DnsRuntimeSnapshot,
    pub derived: DnsRuntimeDerivedState,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DnsLeakServer {
    pub ip: String,
    pub hostname: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DnsLeakTestResult {
    pub has_leak: bool,
    pub observed_leak: bool,
    pub runtime_risk_detected: bool,
    pub observation_incomplete: bool,
    pub confidence: String,
    pub assessment: String,
    pub leak_type: Vec<String>,
    pub observed_leak_type: Vec<String>,
    pub runtime_risk_type: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    pub dns_servers: Vec<DnsLeakServer>,
    pub dns_location: Option<String>,
    pub ip_location: String,
    pub location_match: bool,
    pub location_comparable: bool,
    pub risk_level: String,
    pub timestamp: u64,
    pub checked_via_core_proxy: bool,
    pub observation_path: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProxyDetectionLocation {
    pub country_code: Option<String>,
    pub country: Option<String>,
    pub region: Option<String>,
    pub city: Option<String>,
    pub organization: Option<String>,
    pub asn: Option<u32>,
    pub asn_organization: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProxyDetectionResult {
    pub checked: bool,
    pub core_running: bool,
    pub direct_observed: bool,
    pub proxy_observed: bool,
    pub checked_via_core_proxy: bool,
    pub proxy_effective: bool,
    pub ip_changed: bool,
    pub location_changed: bool,
    pub observation_incomplete: bool,
    pub runtime_risk_detected: bool,
    pub confidence: String,
    pub assessment: String,
    pub runtime_risk_type: Vec<String>,
    pub warnings: Vec<String>,
    pub recommendations: Vec<String>,
    pub direct_ip: Option<String>,
    pub proxy_ip: Option<String>,
    pub direct_location: Option<ProxyDetectionLocation>,
    pub proxy_location: Option<ProxyDetectionLocation>,
    pub observation_path: String,
    pub error: Option<String>,
    pub timestamp: u64,
}
