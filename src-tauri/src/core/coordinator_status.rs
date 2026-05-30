use crate::core::egress_identity::DnsMode;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorResolvedEgressIdentity {
    pub assignment_key: Option<String>,
    pub profile_id: String,
    pub selected_node: String,
    pub dns_mode: DnsMode,
    pub tls_fingerprint: Option<String>,
    pub matched_by: String,
    pub source_group_name: Option<String>,
    pub source_group_selected_node: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorBindingInfo {
    pub binding_type: String,
    pub key: String,
    pub node_id: String,
    pub bound_at: u64,
    pub expires_at: Option<u64>,
    pub remaining_seconds: Option<u64>,
    pub source_group_name: Option<String>,
    pub source_group_selected_node: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StableEgressBackwriteStatus {
    pub domain_pattern_assignments: Vec<CoordinatorResolvedEgressIdentity>,
    pub domain_rule_bindings: Vec<CoordinatorBindingInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorRuntimeState {
    pub egress_identity_assignments: Vec<CoordinatorResolvedEgressIdentity>,
    pub session_affinity_bindings: Vec<CoordinatorBindingInfo>,
    pub stable_egress_backwrite: StableEgressBackwriteStatus,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorStatus {
    pub initialized: bool,
    pub security_enabled: bool,
    pub security_compromised: bool,
    pub anti_probe_enabled: bool,
    pub tls_fingerprint: Option<String>,
    pub egress_identity_enabled: bool,
    pub session_affinity_enabled: bool,
    pub egress_identity_active_assignments: usize,
    pub session_affinity_active_bindings: usize,
    pub runtime_state: CoordinatorRuntimeState,
    pub multipath_enabled: bool,
    pub traffic_obfuscation_enabled: bool,
    pub honeypot_enabled: bool,
    pub self_destruct_enabled: bool,
    #[cfg(target_os = "linux")]
    pub xdp_enabled: bool,
    #[cfg(target_os = "linux")]
    pub xdp_running: bool,
}
