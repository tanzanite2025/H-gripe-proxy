use anyhow::{Result, bail};
use async_trait::async_trait;
use once_cell::sync::Lazy;
use serde::Serialize;
use smartstring::alias::String;
use std::{
    collections::{BTreeMap, BTreeSet},
    net::{TcpListener as StdTcpListener, UdpSocket as StdUdpSocket},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tauri_plugin_mihomo::models::Protocol;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener as TokioTcpListener, TcpStream, UdpSocket as TokioUdpSocket},
    sync::oneshot,
    time::{Duration, timeout},
};

use crate::{
    config::Config,
    core::{
        CoreManager,
        dns_runtime::{DnsDefaultRuntimeShadowEvidenceReport, dns_default_runtime_shadow_evidence},
        handle::Handle,
        manager::RunningMode,
        runtime_snapshot::{build_proxies_from_runtime_config, build_rules_from_runtime_config},
    },
};

const MIHOMO_RUNTIME_ID: &str = "mihomo-kernel-runtime";
const NEXT_SAFE_BATCH: &str = "rust-shadow-components";
const NEXT_SHADOW_BATCH: &str = "loopback-test-listener-opt-in";
const ISOLATED_TEST_LISTENER_HOST: &str = "127.0.0.1";
const DEFAULT_ISOLATED_TEST_LISTENER_PORT: u16 = 19090;
const DEFAULT_LOOPBACK_DNS_PREFLIGHT_PORT: u16 = 19053;
const LOOPBACK_DNS_SMOKE_QUERY: &str = "kernel-smoke.invalid";
const DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT: u16 = 19180;
const DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT: u16 = 19181;

static ISOLATED_TEST_LISTENER: Lazy<Mutex<Option<KernelIsolatedTestListenerState>>> = Lazy::new(|| Mutex::new(None));

struct KernelIsolatedTestListenerState {
    port: u16,
    started_at_epoch_ms: u64,
    accepted_connections: Arc<AtomicU64>,
    stop_tx: oneshot::Sender<()>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelReplacementBlocker {
    pub area: String,
    pub reason: String,
    pub required_next_step: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuntimeStatus {
    pub runtime_id: String,
    pub active_kernel: String,
    pub controller_transport: String,
    pub mutates_runtime: bool,
    pub mihomo_fallback: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuntimePreflightReport {
    pub runtime_id: String,
    pub artifact_id: Option<String>,
    pub mutates_runtime: bool,
    pub can_apply_with_rust_kernel: bool,
    pub mihomo_fallback: bool,
    pub facts: Vec<String>,
    pub blocked_replacement_areas: Vec<KernelReplacementBlocker>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelShadowComponent {
    pub component: String,
    pub kernel_area: String,
    pub status: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub evidence: Vec<String>,
    pub next_step: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelShadowComponentsReport {
    pub runtime_id: String,
    pub active_kernel: String,
    pub mutates_runtime: bool,
    pub components: Vec<KernelShadowComponent>,
    pub live_execution_blockers: Vec<KernelReplacementBlocker>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelDnsShadowEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub evidence: DnsDefaultRuntimeShadowEvidenceReport,
    pub blockers: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuleShadowRule {
    pub index: i32,
    pub rule_type: String,
    pub payload: String,
    pub proxy: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuleShadowSample {
    pub sample_index: usize,
    pub app_rule: Option<KernelRuleShadowRule>,
    pub mihomo_rule: Option<KernelRuleShadowRule>,
    pub matched: bool,
    pub mismatch_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelRuleShadowEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub status: String,
    pub app_rule_count: usize,
    pub mihomo_rule_count: usize,
    pub compared_sample_size: usize,
    pub matched_sample_count: usize,
    pub mismatched_sample_count: usize,
    pub samples: Vec<KernelRuleShadowSample>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelAdapterCapabilityEntry {
    pub proxy_type: String,
    pub app_count: usize,
    pub mihomo_count: usize,
    pub inventory_matched: bool,
    pub rust_shadow_supported: bool,
    pub live_execution_allowed: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelAdapterCapabilityReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub app_proxy_count: usize,
    pub mihomo_proxy_count: usize,
    pub capabilities: Vec<KernelAdapterCapabilityEntry>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelConnectionSessionSample {
    pub sample_index: usize,
    pub network: String,
    pub connection_type: String,
    pub chain_len: usize,
    pub provider_chain_len: usize,
    pub has_host: bool,
    pub has_process: bool,
    pub has_remote_destination: bool,
    pub rule: String,
    pub uploaded_bytes: u64,
    pub downloaded_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelConnectionSessionShadowReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub connection_count: usize,
    pub upload_total: u64,
    pub download_total: u64,
    pub memory: u32,
    pub network_counts: BTreeMap<String, usize>,
    pub connection_type_counts: BTreeMap<String, usize>,
    pub rule_counts: BTreeMap<String, usize>,
    pub samples: Vec<KernelConnectionSessionSample>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelIsolatedListenerPortCheck {
    pub host: String,
    pub port: u16,
    pub available: bool,
    pub conflicts_with_runtime_port: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelIsolatedListenerPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub requested_port: u16,
    pub can_start_after_opt_in: bool,
    pub port_check: KernelIsolatedListenerPortCheck,
    pub runtime_ports: BTreeMap<String, u16>,
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelIsolatedTestListenerStatus {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub running: bool,
    pub host: String,
    pub port: Option<u16>,
    pub started_at_epoch_ms: Option<u64>,
    pub accepted_connections: u64,
    pub loopback_only: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelIsolatedTestListenerSmokeEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub requested_port: u16,
    pub started_by_smoke: bool,
    pub response_status: Option<String>,
    pub accepted_connections_before: u64,
    pub accepted_connections_after: u64,
    pub status_incremented: bool,
    pub stopped_after_smoke: bool,
    pub system_proxy_unchanged: bool,
    pub tun_unchanged: bool,
    pub runtime_config_unchanged: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackDnsPortCheck {
    pub host: String,
    pub port: u16,
    pub udp_available: bool,
    pub tcp_available: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackDnsPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub requested_port: u16,
    pub can_start_after_opt_in: bool,
    pub port_check: KernelLoopbackDnsPortCheck,
    pub runtime_dns_present: bool,
    pub app_dns_settings_enabled: bool,
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackDnsSmokeEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub requested_port: u16,
    pub query_name: String,
    pub udp_bound: bool,
    pub local_response_received: bool,
    pub response_address: Option<String>,
    pub system_proxy_unchanged: bool,
    pub tun_unchanged: bool,
    pub runtime_config_unchanged: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackForwardingPortCheck {
    pub host: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub listener_available: bool,
    pub target_available: bool,
    pub target_loopback_only: bool,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackForwardingPreflightReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub can_start_after_opt_in: bool,
    pub port_check: KernelLoopbackForwardingPortCheck,
    pub system_proxy_enabled: bool,
    pub tun_enabled: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_allowed: bool,
    pub mihomo_fallback: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackForwardingSmokeEvidenceReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub requested_host: String,
    pub listener_port: u16,
    pub target_port: u16,
    pub request_path: String,
    pub listener_accepted: bool,
    pub target_received: bool,
    pub response_status: Option<String>,
    pub bytes_from_client: u64,
    pub bytes_from_target: u64,
    pub loopback_forwarded: bool,
    pub system_proxy_unchanged: bool,
    pub tun_unchanged: bool,
    pub runtime_config_unchanged: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelLoopbackForwardingRollbackDrillReport {
    pub runtime_id: String,
    pub component: String,
    pub kernel_area: String,
    pub mutates_runtime: bool,
    pub live_execution_allowed: bool,
    pub listener_port: u16,
    pub target_port: u16,
    pub smoke_passed: bool,
    pub ports_released: bool,
    pub post_preflight: KernelLoopbackForwardingPreflightReport,
    pub system_proxy_unchanged: bool,
    pub tun_unchanged: bool,
    pub runtime_config_unchanged: bool,
    pub default_route: bool,
    pub forwards_traffic: bool,
    pub outbound_adapters_used: bool,
    pub mihomo_fallback: bool,
    pub passed: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KernelReplacementReadiness {
    pub mutates_runtime: bool,
    pub active_kernel: String,
    pub controller_transport: String,
    pub rust_owned_control_plane: Vec<String>,
    pub mihomo_owned_data_plane: Vec<String>,
    pub blocked_replacement_areas: Vec<KernelReplacementBlocker>,
    pub next_safe_batch: String,
}

#[async_trait]
pub trait KernelRuntime: Send + Sync {
    fn runtime_id(&self) -> &'static str;

    async fn status(&self) -> KernelRuntimeStatus;

    async fn replacement_readiness(&self) -> KernelReplacementReadiness;

    async fn shadow_components(&self) -> KernelShadowComponentsReport;

    async fn apply_projection_preflight(&self, artifact_id: Option<String>) -> KernelRuntimePreflightReport;
}

#[derive(Debug, Default)]
pub struct MihomoKernelRuntime;

#[async_trait]
impl KernelRuntime for MihomoKernelRuntime {
    fn runtime_id(&self) -> &'static str {
        MIHOMO_RUNTIME_ID
    }

    async fn status(&self) -> KernelRuntimeStatus {
        KernelRuntimeStatus {
            runtime_id: self.runtime_id().into(),
            active_kernel: active_kernel_label(),
            controller_transport: controller_transport_label(&Handle::mihomo().await.protocol),
            mutates_runtime: false,
            mihomo_fallback: true,
        }
    }

    async fn replacement_readiness(&self) -> KernelReplacementReadiness {
        let status = self.status().await;

        KernelReplacementReadiness {
            mutates_runtime: false,
            active_kernel: status.active_kernel,
            controller_transport: status.controller_transport,
            rust_owned_control_plane: rust_owned_control_plane(),
            mihomo_owned_data_plane: mihomo_owned_data_plane(),
            blocked_replacement_areas: blocked_replacement_areas(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        }
    }

    async fn shadow_components(&self) -> KernelShadowComponentsReport {
        KernelShadowComponentsReport {
            runtime_id: self.runtime_id().into(),
            active_kernel: active_kernel_label(),
            mutates_runtime: false,
            components: shadow_components(),
            live_execution_blockers: blocked_replacement_areas(),
            next_safe_batch: NEXT_SHADOW_BATCH.into(),
        }
    }

    async fn apply_projection_preflight(&self, artifact_id: Option<String>) -> KernelRuntimePreflightReport {
        KernelRuntimePreflightReport {
            runtime_id: self.runtime_id().into(),
            artifact_id,
            mutates_runtime: false,
            can_apply_with_rust_kernel: false,
            mihomo_fallback: true,
            facts: vec![
                "MihomoKernelRuntime is the current adapter over CoreManager and tauri-plugin-mihomo".into(),
                "This preflight is read-only and does not start, stop, reload, or patch Mihomo".into(),
                "Rust-native kernel apply remains blocked until shadow runtime evidence exists".into(),
            ],
            blocked_replacement_areas: blocked_replacement_areas(),
            next_safe_batch: NEXT_SAFE_BATCH.into(),
        }
    }
}

pub async fn mihomo_kernel_replacement_readiness() -> KernelReplacementReadiness {
    MihomoKernelRuntime.replacement_readiness().await
}

pub async fn mihomo_kernel_apply_preflight(artifact_id: Option<String>) -> KernelRuntimePreflightReport {
    MihomoKernelRuntime.apply_projection_preflight(artifact_id).await
}

pub async fn mihomo_kernel_shadow_components() -> KernelShadowComponentsReport {
    MihomoKernelRuntime.shadow_components().await
}

pub async fn mihomo_kernel_dns_shadow_evidence(
    yaml: Option<String>,
    domain: Option<String>,
) -> Result<KernelDnsShadowEvidenceReport> {
    let evidence = dns_default_runtime_shadow_evidence(yaml, domain).await?;
    let mut blockers = evidence.blockers.clone();
    blockers.push("Rust kernel DNS live execution remains blocked; this command is shadow evidence only".into());

    Ok(KernelDnsShadowEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "dns-shadow-resolver".into(),
        kernel_area: "dns".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        evidence,
        blockers,
        next_safe_batch: "rule-shadow-classification-evidence".into(),
    })
}

pub async fn mihomo_kernel_rule_shadow_evidence() -> Result<KernelRuleShadowEvidenceReport> {
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
    let app_rules = build_rules_from_runtime_config(config);
    let mihomo_rules = Handle::mihomo().await.get_rules().await?;
    let sample_size = app_rules.rules.len().max(mihomo_rules.rules.len()).min(25);
    let mut samples = Vec::with_capacity(sample_size);

    for sample_index in 0..sample_size {
        let app_rule = app_rules.rules.get(sample_index).map(kernel_rule_shadow_rule);
        let mihomo_rule = mihomo_rules.rules.get(sample_index).map(kernel_rule_shadow_rule);
        let mismatch_reason = rule_shadow_mismatch_reason(app_rule.as_ref(), mihomo_rule.as_ref());
        samples.push(KernelRuleShadowSample {
            sample_index,
            app_rule,
            mihomo_rule,
            matched: mismatch_reason.is_none(),
            mismatch_reason,
        });
    }

    let matched_sample_count = samples.iter().filter(|sample| sample.matched).count();
    let mismatched_sample_count = samples.len().saturating_sub(matched_sample_count);
    let mut warnings = Vec::new();
    if app_rules.rules.len() != mihomo_rules.rules.len() {
        warnings.push(format!(
            "app rule inventory has {} rule(s), Mihomo reports {} rule(s)",
            app_rules.rules.len(),
            mihomo_rules.rules.len()
        ));
    }
    if mismatched_sample_count > 0 {
        warnings.push(format!(
            "{} sampled rule position(s) differ between app and Mihomo inventory",
            mismatched_sample_count
        ));
    }

    Ok(KernelRuleShadowEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "rule-shadow-classifier".into(),
        kernel_area: "rule-engine".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        status: if warnings.is_empty() { "matched" } else { "mismatched" }.into(),
        app_rule_count: app_rules.rules.len(),
        mihomo_rule_count: mihomo_rules.rules.len(),
        compared_sample_size: samples.len(),
        matched_sample_count,
        mismatched_sample_count,
        samples,
        blockers: vec![
            "Rust kernel rule classification is shadow-only and must not route traffic".into(),
            "Mihomo remains the only live rule decision owner for forwarding".into(),
        ],
        warnings,
        facts: vec![
            "sample compares app runtime rule projection with Mihomo controller rule inventory".into(),
            "command reads rule inventories only and does not change rule providers or mode".into(),
        ],
        next_safe_batch: "adapter-capability-report".into(),
    })
}

pub async fn mihomo_kernel_adapter_capability_report() -> Result<KernelAdapterCapabilityReport> {
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
    let app_proxies = build_proxies_from_runtime_config(config);
    let mihomo_proxies = Handle::mihomo().await.get_proxies().await?;
    let app_counts = proxy_type_counts(&app_proxies.proxies);
    let mihomo_counts = proxy_type_counts(&mihomo_proxies.proxies);
    let mut proxy_types = BTreeSet::new();
    proxy_types.extend(app_counts.keys().cloned());
    proxy_types.extend(mihomo_counts.keys().cloned());

    let capabilities = proxy_types
        .into_iter()
        .map(|proxy_type| {
            let app_count = app_counts.get(&proxy_type).copied().unwrap_or_default();
            let mihomo_count = mihomo_counts.get(&proxy_type).copied().unwrap_or_default();
            let inventory_matched = app_count == mihomo_count;
            let rust_shadow_supported = proxy_type != "Unknown";
            let mut notes = vec!["inventory-only capability; no outbound sockets opened".into()];
            if !inventory_matched {
                notes.push("app runtime projection and Mihomo inventory counts differ".into());
            }
            if !rust_shadow_supported {
                notes.push("unknown adapter type requires explicit Rust parser support before shadow execution".into());
            }
            KernelAdapterCapabilityEntry {
                proxy_type,
                app_count,
                mihomo_count,
                inventory_matched,
                rust_shadow_supported,
                live_execution_allowed: false,
                notes,
            }
        })
        .collect::<Vec<_>>();
    let warnings = capabilities
        .iter()
        .filter(|capability| !capability.inventory_matched || !capability.rust_shadow_supported)
        .map(|capability| format!("adapter capability needs review: {}", capability.proxy_type).into())
        .collect::<Vec<_>>();

    Ok(KernelAdapterCapabilityReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "adapter-capability-shadow".into(),
        kernel_area: "adapter".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        app_proxy_count: app_proxies.proxies.len(),
        mihomo_proxy_count: mihomo_proxies.proxies.len(),
        capabilities,
        blockers: vec![
            "Rust adapter capability reporting is inventory-only and must not dial proxy endpoints".into(),
            "Mihomo remains the only live adapter runtime owner".into(),
        ],
        warnings,
        facts: vec![
            "report compares app runtime proxy projection with Mihomo controller proxy inventory".into(),
            "adapter parsing evidence must precede any Rust protocol stack implementation".into(),
        ],
        next_safe_batch: "connection-session-shadow-model".into(),
    })
}

pub async fn mihomo_kernel_connection_session_shadow() -> Result<KernelConnectionSessionShadowReport> {
    let connections = Handle::mihomo().await.get_connections().await?;
    let sessions = connections.connections.unwrap_or_default();
    let mut network_counts = BTreeMap::new();
    let mut connection_type_counts = BTreeMap::new();
    let mut rule_counts = BTreeMap::new();

    for session in &sessions {
        increment_count(&mut network_counts, format!("{:?}", session.metadata.network).into());
        increment_count(
            &mut connection_type_counts,
            format!("{:?}", session.metadata.connection_type).into(),
        );
        increment_count(&mut rule_counts, session.rule.clone().into());
    }

    let samples = sessions
        .iter()
        .take(10)
        .enumerate()
        .map(connection_session_sample)
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    if sessions.is_empty() {
        warnings.push("no active Mihomo connections observed; shape model has no live samples".into());
    }

    Ok(KernelConnectionSessionShadowReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "connection-observer-shadow".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        connection_count: sessions.len(),
        upload_total: connections.upload_total,
        download_total: connections.download_total,
        memory: connections.memory,
        network_counts,
        connection_type_counts,
        rule_counts,
        samples,
        blockers: vec![
            "Rust connection observation is shape-only and must not close or migrate sessions".into(),
            "Mihomo remains the only live forwarding owner".into(),
        ],
        warnings,
        facts: vec![
            "report reads Mihomo connection inventory and strips endpoint identifiers from samples".into(),
            "session shape evidence must precede isolated test listener execution".into(),
        ],
        next_safe_batch: "isolated-test-listener-preflight".into(),
    })
}

pub async fn mihomo_kernel_isolated_listener_preflight(
    port: Option<u16>,
) -> Result<KernelIsolatedListenerPreflightReport> {
    let requested_host: String = "127.0.0.1".into();
    let requested_port = port.unwrap_or(DEFAULT_ISOLATED_TEST_LISTENER_PORT);
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
    let runtime_ports = kernel_runtime_ports(config);
    let verge = Config::verge().await.latest_arc();
    let system_proxy_enabled = verge.enable_system_proxy.unwrap_or(false);
    let tun_enabled = verge.enable_tun_mode.unwrap_or(false);
    let conflicts_with_runtime_port = runtime_ports.values().any(|port| *port == requested_port);
    let available = kernel_loopback_port_available(requested_port);
    let mut notes = vec!["loopback-only candidate; preflight does not start a listener".into()];
    if conflicts_with_runtime_port {
        notes.push("candidate port matches an existing Mihomo runtime port".into());
    }
    if !available {
        notes.push("candidate port is unavailable on 127.0.0.1".into());
    }
    let mut blockers =
        vec!["Rust isolated listener remains opt-in only; this preflight must not start forwarding".into()];
    if conflicts_with_runtime_port {
        blockers.push("choose a port that does not overlap Mihomo runtime listeners".into());
    }
    if !available {
        blockers.push("choose an unused loopback port before enabling a test listener".into());
    }
    let mut warnings = Vec::new();
    if system_proxy_enabled {
        warnings.push("system proxy is currently enabled; R3 listener must not become the default proxy".into());
    }
    if tun_enabled {
        warnings.push("TUN is currently enabled; R3 listener must not attach to transparent proxy routing".into());
    }

    Ok(KernelIsolatedListenerPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "isolated-test-listener-preflight".into(),
        kernel_area: "listener".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        requested_host,
        requested_port,
        can_start_after_opt_in: available && !conflicts_with_runtime_port,
        port_check: KernelIsolatedListenerPortCheck {
            host: "127.0.0.1".into(),
            port: requested_port,
            available,
            conflicts_with_runtime_port,
            notes,
        },
        runtime_ports,
        system_proxy_enabled,
        tun_enabled,
        blockers,
        warnings,
        facts: vec![
            "preflight reads runtime listener configuration and checks loopback port availability".into(),
            "R3 may only use a bounded loopback test path with Mihomo fallback preserved".into(),
        ],
        next_safe_batch: "loopback-test-listener-opt-in".into(),
    })
}

pub async fn mihomo_kernel_loopback_dns_preflight(port: Option<u16>) -> Result<KernelLoopbackDnsPreflightReport> {
    let requested_port = port.unwrap_or(DEFAULT_LOOPBACK_DNS_PREFLIGHT_PORT);
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
    let runtime_dns_present = config.get("dns").is_some();
    let verge = Config::verge().await.latest_arc();
    let app_dns_settings_enabled = verge.enable_dns_settings.unwrap_or(false);
    let system_proxy_enabled = verge.enable_system_proxy.unwrap_or(false);
    let tun_enabled = verge.enable_tun_mode.unwrap_or(false);
    let udp_available = kernel_loopback_udp_port_available(requested_port);
    let tcp_available = kernel_loopback_port_available(requested_port);
    let mut notes = vec!["loopback DNS candidate; preflight does not bind persistent sockets".into()];
    if !udp_available {
        notes.push("candidate UDP port is unavailable on 127.0.0.1".into());
    }
    if !tcp_available {
        notes.push("candidate TCP port is unavailable on 127.0.0.1".into());
    }

    let mut blockers = vec![
        "loopback DNS remains opt-in only and must not replace default Mihomo DNS".into(),
        "R3 DNS preflight must not patch Mihomo config, TUN, system proxy, or forwarding".into(),
    ];
    if !udp_available {
        blockers.push("choose an unused loopback UDP port before enabling loopback DNS smoke evidence".into());
    }
    if !tcp_available {
        blockers.push("choose an unused loopback TCP port before enabling loopback DNS smoke evidence".into());
    }

    let mut warnings = Vec::new();
    if app_dns_settings_enabled {
        warnings.push("app DNS settings are enabled; loopback DNS must still remain an isolated test path".into());
    }
    if system_proxy_enabled {
        warnings.push("system proxy is enabled; loopback DNS must not become a default proxy dependency".into());
    }
    if tun_enabled {
        warnings.push("TUN is enabled; loopback DNS must not attach to transparent proxy routing".into());
    }

    Ok(KernelLoopbackDnsPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-dns-preflight".into(),
        kernel_area: "dns".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        requested_port,
        can_start_after_opt_in: udp_available && tcp_available,
        port_check: KernelLoopbackDnsPortCheck {
            host: ISOLATED_TEST_LISTENER_HOST.into(),
            port: requested_port,
            udp_available,
            tcp_available,
            notes,
        },
        runtime_dns_present,
        app_dns_settings_enabled,
        system_proxy_enabled,
        tun_enabled,
        default_route: false,
        forwards_traffic: false,
        mihomo_fallback: true,
        blockers,
        warnings,
        facts: vec![
            "preflight checks loopback UDP and TCP DNS candidate ports without keeping sockets open".into(),
            "default Mihomo DNS remains production owner until a dedicated opt-in execution batch".into(),
            "loopback DNS must not mutate runtime config, system proxy, TUN, or outbound forwarding".into(),
        ],
        next_safe_batch: "loopback-dns-smoke-evidence".into(),
    })
}

pub async fn mihomo_kernel_loopback_dns_smoke_evidence(
    port: Option<u16>,
) -> Result<KernelLoopbackDnsSmokeEvidenceReport> {
    let requested_port = port.unwrap_or(DEFAULT_LOOPBACK_DNS_PREFLIGHT_PORT);
    let preflight = mihomo_kernel_loopback_dns_preflight(Some(requested_port)).await?;
    let before_runtime_config = kernel_runtime_config_snapshot().await?;
    let before_verge = Config::verge().await.latest_arc();
    let before_system_proxy = before_verge.enable_system_proxy.unwrap_or(false);
    let before_tun = before_verge.enable_tun_mode.unwrap_or(false);
    let mut warnings = preflight.warnings.clone();

    if !preflight.can_start_after_opt_in {
        return Ok(kernel_loopback_dns_smoke_report(
            requested_port,
            false,
            false,
            None,
            true,
            true,
            true,
            preflight.blockers,
            warnings,
        ));
    }

    let server = TokioUdpSocket::bind((ISOLATED_TEST_LISTENER_HOST, requested_port)).await?;
    let server_task = tokio::spawn(async move {
        let mut request = [0_u8; 512];
        let (request_len, peer) = timeout(Duration::from_secs(2), server.recv_from(&mut request)).await??;
        if let Some(response) = build_loopback_dns_smoke_response(&request[..request_len]) {
            server.send_to(&response, peer).await?;
            Ok::<bool, anyhow::Error>(true)
        } else {
            Ok(false)
        }
    });

    let client = TokioUdpSocket::bind((ISOLATED_TEST_LISTENER_HOST, 0)).await?;
    let query = build_loopback_dns_smoke_query(LOOPBACK_DNS_SMOKE_QUERY);
    client
        .send_to(&query, (ISOLATED_TEST_LISTENER_HOST, requested_port))
        .await?;
    let mut response = [0_u8; 512];
    let response_len = timeout(Duration::from_secs(2), client.recv(&mut response)).await??;
    let response_address = parse_loopback_dns_smoke_response(&response[..response_len]);
    let server_responded = server_task.await??;
    let local_response_received = server_responded && response_address.is_some();

    let after_runtime_config = kernel_runtime_config_snapshot().await?;
    let after_verge = Config::verge().await.latest_arc();
    let system_proxy_unchanged = before_system_proxy == after_verge.enable_system_proxy.unwrap_or(false);
    let tun_unchanged = before_tun == after_verge.enable_tun_mode.unwrap_or(false);
    let runtime_config_unchanged = before_runtime_config == after_runtime_config;
    let mut blockers = Vec::new();
    if response_address.as_deref() != Some("127.0.0.1") {
        blockers.push("loopback DNS smoke response did not return 127.0.0.1".into());
    }
    if !system_proxy_unchanged {
        blockers.push("system proxy setting changed during DNS smoke evidence".into());
    }
    if !tun_unchanged {
        blockers.push("TUN setting changed during DNS smoke evidence".into());
    }
    if !runtime_config_unchanged {
        blockers.push("runtime config changed during DNS smoke evidence".into());
    }
    warnings.push(
        "DNS smoke evidence uses a synthetic kernel-smoke.invalid query and must not be used as production DNS".into(),
    );

    Ok(kernel_loopback_dns_smoke_report(
        requested_port,
        true,
        local_response_received,
        response_address,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        blockers,
        warnings,
    ))
}

pub async fn mihomo_kernel_loopback_forwarding_preflight(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackForwardingPreflightReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let listener_available = kernel_loopback_port_available(listener_port);
    let target_available = kernel_loopback_port_available(target_port);
    let verge = Config::verge().await.latest_arc();
    let system_proxy_enabled = verge.enable_system_proxy.unwrap_or(false);
    let tun_enabled = verge.enable_tun_mode.unwrap_or(false);
    let mut notes = vec![
        "preflight checks only candidate loopback TCP ports and does not keep sockets open".into(),
        "future smoke target must be a synthetic local responder, not a real outbound adapter".into(),
    ];
    if listener_port == target_port {
        notes.push("listener and target ports must differ for a forwarding smoke path".into());
    }
    if !listener_available {
        notes.push("candidate listener port is unavailable on 127.0.0.1".into());
    }
    if !target_available {
        notes.push("candidate target port is unavailable on 127.0.0.1".into());
    }

    let mut blockers = vec![
        "loopback forwarding remains opt-in only and must not become a system proxy/default route".into(),
        "future smoke evidence must forward only to a synthetic loopback target, never Mihomo/outbound adapters".into(),
        "TUN, transparent proxy, protocol stack replacement, and production forwarding remain blocked".into(),
    ];
    if listener_port == target_port {
        blockers.push("choose different listener and target ports before forwarding smoke evidence".into());
    }
    if !listener_available {
        blockers.push("choose an unused loopback listener TCP port before forwarding smoke evidence".into());
    }
    if !target_available {
        blockers.push("choose an unused loopback target TCP port before forwarding smoke evidence".into());
    }

    let mut warnings = Vec::new();
    if system_proxy_enabled {
        warnings.push("system proxy is enabled; loopback forwarding smoke must not register as a proxy".into());
    }
    if tun_enabled {
        warnings.push("TUN is enabled; loopback forwarding smoke must not attach to transparent proxy routing".into());
    }

    Ok(KernelLoopbackForwardingPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-forwarding-preflight".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        listener_port,
        target_port,
        can_start_after_opt_in: listener_port != target_port && listener_available && target_available,
        port_check: KernelLoopbackForwardingPortCheck {
            host: ISOLATED_TEST_LISTENER_HOST.into(),
            listener_port,
            target_port,
            listener_available,
            target_available,
            target_loopback_only: true,
            notes,
        },
        system_proxy_enabled,
        tun_enabled,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_allowed: false,
        mihomo_fallback: true,
        blockers,
        warnings,
        facts: vec![
            "preflight only checks local port readiness and safety gates".into(),
            "forwarding smoke evidence must stay inside 127.0.0.1 listener -> 127.0.0.1 target".into(),
            "real adapter dialing, TUN, system proxy, and default route changes are still forbidden".into(),
        ],
        next_safe_batch: "loopback-forwarding-smoke-evidence".into(),
    })
}

pub async fn mihomo_kernel_loopback_forwarding_smoke_evidence(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackForwardingSmokeEvidenceReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let preflight = mihomo_kernel_loopback_forwarding_preflight(Some(listener_port), Some(target_port)).await?;
    let before_runtime_config = kernel_runtime_config_snapshot().await?;
    let before_verge = Config::verge().await.latest_arc();
    let before_system_proxy = before_verge.enable_system_proxy.unwrap_or(false);
    let before_tun = before_verge.enable_tun_mode.unwrap_or(false);
    let mut warnings = preflight.warnings.clone();

    if !preflight.can_start_after_opt_in {
        return Ok(kernel_loopback_forwarding_smoke_report(
            listener_port,
            target_port,
            false,
            false,
            None,
            0,
            0,
            true,
            true,
            true,
            preflight.blockers,
            warnings,
        ));
    }

    let target = TokioTcpListener::bind((ISOLATED_TEST_LISTENER_HOST, target_port)).await?;
    let listener = TokioTcpListener::bind((ISOLATED_TEST_LISTENER_HOST, listener_port)).await?;

    let target_task = tokio::spawn(async move {
        let (mut stream, _) = timeout(Duration::from_secs(2), target.accept()).await??;
        let mut request = [0_u8; 512];
        let request_len = timeout(Duration::from_secs(2), stream.read(&mut request)).await??;
        let received = std::str::from_utf8(&request[..request_len])
            .map(|request| request.contains("GET /kernel-forwarding-smoke"))
            .unwrap_or(false);
        stream
            .write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n")
            .await?;
        stream.shutdown().await?;
        Ok::<bool, anyhow::Error>(received)
    });

    let listener_task = tokio::spawn(async move {
        let (mut inbound, _) = timeout(Duration::from_secs(2), listener.accept()).await??;
        let mut outbound = timeout(
            Duration::from_secs(2),
            TcpStream::connect((ISOLATED_TEST_LISTENER_HOST, target_port)),
        )
        .await??;
        let mut request = [0_u8; 512];
        let request_len = timeout(Duration::from_secs(2), inbound.read(&mut request)).await??;
        outbound.write_all(&request[..request_len]).await?;
        let mut response = [0_u8; 512];
        let response_len = timeout(Duration::from_secs(2), outbound.read(&mut response)).await??;
        inbound.write_all(&response[..response_len]).await?;
        inbound.shutdown().await?;
        Ok::<(u64, u64), anyhow::Error>((request_len as u64, response_len as u64))
    });

    let mut client = timeout(
        Duration::from_secs(2),
        TcpStream::connect((ISOLATED_TEST_LISTENER_HOST, listener_port)),
    )
    .await??;
    client
        .write_all(b"GET /kernel-forwarding-smoke HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .await?;
    let mut response = [0_u8; 512];
    let response_len = timeout(Duration::from_secs(2), client.read(&mut response)).await??;
    let response = std::string::String::from_utf8_lossy(&response[..response_len]);
    let response_status = response.lines().next().map(Into::into);
    let (bytes_from_client, bytes_from_target) = listener_task.await??;
    let target_received = target_task.await??;

    let after_runtime_config = kernel_runtime_config_snapshot().await?;
    let after_verge = Config::verge().await.latest_arc();
    let system_proxy_unchanged = before_system_proxy == after_verge.enable_system_proxy.unwrap_or(false);
    let tun_unchanged = before_tun == after_verge.enable_tun_mode.unwrap_or(false);
    let runtime_config_unchanged = before_runtime_config == after_runtime_config;
    let mut blockers = Vec::new();
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("loopback forwarding smoke response did not return HTTP 204".into());
    }
    if !target_received {
        blockers.push("synthetic target did not receive the forwarding smoke request".into());
    }
    if !system_proxy_unchanged {
        blockers.push("system proxy setting changed during forwarding smoke evidence".into());
    }
    if !tun_unchanged {
        blockers.push("TUN setting changed during forwarding smoke evidence".into());
    }
    if !runtime_config_unchanged {
        blockers.push("runtime config changed during forwarding smoke evidence".into());
    }
    warnings.push(
        "forwarding smoke evidence uses only synthetic loopback endpoints and must not be connected to real adapters"
            .into(),
    );

    Ok(kernel_loopback_forwarding_smoke_report(
        listener_port,
        target_port,
        true,
        target_received,
        response_status,
        bytes_from_client,
        bytes_from_target,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        blockers,
        warnings,
    ))
}

pub async fn mihomo_kernel_loopback_forwarding_rollback_drill(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackForwardingRollbackDrillReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let before_runtime_config = kernel_runtime_config_snapshot().await?;
    let before_verge = Config::verge().await.latest_arc();
    let before_system_proxy = before_verge.enable_system_proxy.unwrap_or(false);
    let before_tun = before_verge.enable_tun_mode.unwrap_or(false);

    let smoke = mihomo_kernel_loopback_forwarding_smoke_evidence(Some(listener_port), Some(target_port)).await?;
    let post_preflight = mihomo_kernel_loopback_forwarding_preflight(Some(listener_port), Some(target_port)).await?;
    let ports_released = post_preflight.can_start_after_opt_in;
    let after_runtime_config = kernel_runtime_config_snapshot().await?;
    let after_verge = Config::verge().await.latest_arc();
    let system_proxy_unchanged = before_system_proxy == after_verge.enable_system_proxy.unwrap_or(false);
    let tun_unchanged = before_tun == after_verge.enable_tun_mode.unwrap_or(false);
    let runtime_config_unchanged = before_runtime_config == after_runtime_config;

    let mut blockers = Vec::new();
    if !smoke.passed {
        blockers.push("loopback forwarding smoke evidence did not pass before rollback drill".into());
    }
    if !ports_released {
        blockers.push("loopback forwarding smoke ports were not released after the drill".into());
    }
    if !system_proxy_unchanged {
        blockers.push("system proxy setting changed during forwarding rollback drill".into());
    }
    if !tun_unchanged {
        blockers.push("TUN setting changed during forwarding rollback drill".into());
    }
    if !runtime_config_unchanged {
        blockers.push("runtime config changed during forwarding rollback drill".into());
    }

    let passed = blockers.is_empty();
    Ok(KernelLoopbackForwardingRollbackDrillReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-forwarding-rollback-drill".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: true,
        live_execution_allowed: true,
        listener_port,
        target_port,
        smoke_passed: smoke.passed,
        ports_released,
        post_preflight,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed,
        blockers,
        warnings: vec!["rollback drill remains synthetic loopback-only and does not exercise real adapters".into()],
        facts: vec![
            "drill runs loopback forwarding smoke evidence and immediately re-runs preflight".into(),
            "post-preflight must show listener and target ports are available again".into(),
            "runtime config, system proxy, and TUN settings are compared before and after".into(),
        ],
        next_safe_batch: "loopback-forwarding-leak-check".into(),
    })
}

pub async fn mihomo_kernel_isolated_test_listener_status() -> KernelIsolatedTestListenerStatus {
    isolated_test_listener_status(Vec::new())
}

pub async fn mihomo_kernel_start_isolated_test_listener(port: Option<u16>) -> Result<KernelIsolatedTestListenerStatus> {
    if let Some(status) = isolated_test_listener_running_status() {
        return Ok(status);
    }

    let preflight = mihomo_kernel_isolated_listener_preflight(port).await?;
    if !preflight.can_start_after_opt_in {
        bail!(
            "isolated test listener preflight failed: {}",
            preflight
                .blockers
                .iter()
                .map(|blocker| blocker.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        );
    }

    let port = preflight.requested_port;
    let listener = TokioTcpListener::bind((ISOLATED_TEST_LISTENER_HOST, port)).await?;
    let accepted_connections = Arc::new(AtomicU64::new(0));
    let task_counter = accepted_connections.clone();
    let (stop_tx, mut stop_rx) = oneshot::channel();
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut stop_rx => break,
                accepted = listener.accept() => {
                    let Ok((mut stream, _)) = accepted else {
                        break;
                    };
                    task_counter.fetch_add(1, Ordering::Relaxed);
                    tauri::async_runtime::spawn(async move {
                        let _ = stream.write_all(b"HTTP/1.1 204 No Content\r\nConnection: close\r\nContent-Length: 0\r\n\r\n").await;
                        let _ = stream.shutdown().await;
                    });
                }
            }
        }
    });

    let state = KernelIsolatedTestListenerState {
        port,
        started_at_epoch_ms: current_epoch_ms(),
        accepted_connections,
        stop_tx,
    };
    let mut guard = ISOLATED_TEST_LISTENER.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_some() {
        return Ok(isolated_test_listener_status(vec![
            "isolated test listener was already running".into(),
        ]));
    }
    *guard = Some(state);
    Ok(isolated_test_listener_status(Vec::new()))
}

pub async fn mihomo_kernel_stop_isolated_test_listener() -> KernelIsolatedTestListenerStatus {
    let state = ISOLATED_TEST_LISTENER.lock().unwrap_or_else(|e| e.into_inner()).take();
    if let Some(state) = state {
        let _ = state.stop_tx.send(());
        return isolated_test_listener_status(vec!["isolated test listener stopped".into()]);
    }
    isolated_test_listener_status(vec!["isolated test listener was not running".into()])
}

pub async fn mihomo_kernel_isolated_test_listener_smoke_evidence(
    port: Option<u16>,
) -> Result<KernelIsolatedTestListenerSmokeEvidenceReport> {
    let requested_port = port.unwrap_or(DEFAULT_ISOLATED_TEST_LISTENER_PORT);
    let before_status = mihomo_kernel_isolated_test_listener_status().await;
    let before_runtime_config = kernel_runtime_config_snapshot().await?;
    let before_verge = Config::verge().await.latest_arc();
    let before_system_proxy = before_verge.enable_system_proxy.unwrap_or(false);
    let before_tun = before_verge.enable_tun_mode.unwrap_or(false);

    if before_status.running {
        return Ok(kernel_listener_smoke_report(
            requested_port,
            false,
            None,
            before_status.accepted_connections,
            before_status.accepted_connections,
            false,
            false,
            true,
            true,
            true,
            vec!["isolated test listener is already running; smoke evidence did not take lifecycle ownership".into()],
            Vec::new(),
        ));
    }

    let start_status = mihomo_kernel_start_isolated_test_listener(Some(requested_port)).await?;
    let accepted_connections_before = start_status.accepted_connections;
    let mut warnings = start_status.warnings.clone();
    let mut blockers = Vec::new();
    if !start_status.running {
        blockers.push("isolated test listener did not enter running state".into());
    }

    let response_status = if start_status.running {
        match isolated_test_listener_smoke_request(requested_port).await {
            Ok(status) => Some(status),
            Err(err) => {
                blockers.push(format!("local smoke request failed: {err}").into());
                None
            }
        }
    } else {
        None
    };

    let after_request_status = mihomo_kernel_isolated_test_listener_status().await;
    let accepted_connections_after = after_request_status.accepted_connections;
    let status_incremented = accepted_connections_after > accepted_connections_before;
    if !status_incremented {
        blockers.push("accepted connection count did not increase after local request".into());
    }
    if response_status.as_deref() != Some("HTTP/1.1 204 No Content") {
        blockers.push("local listener did not return HTTP 204 smoke response".into());
    }

    let stop_status = mihomo_kernel_stop_isolated_test_listener().await;
    warnings.extend(stop_status.warnings);
    let stopped_after_smoke = !stop_status.running;
    if !stopped_after_smoke {
        blockers.push("isolated test listener remained running after stop".into());
    }

    let after_runtime_config = kernel_runtime_config_snapshot().await?;
    let after_verge = Config::verge().await.latest_arc();
    let system_proxy_unchanged = before_system_proxy == after_verge.enable_system_proxy.unwrap_or(false);
    let tun_unchanged = before_tun == after_verge.enable_tun_mode.unwrap_or(false);
    let runtime_config_unchanged = before_runtime_config == after_runtime_config;
    if !system_proxy_unchanged {
        blockers.push("system proxy setting changed during smoke evidence".into());
    }
    if !tun_unchanged {
        blockers.push("TUN setting changed during smoke evidence".into());
    }
    if !runtime_config_unchanged {
        blockers.push("runtime config changed during smoke evidence".into());
    }

    Ok(kernel_listener_smoke_report(
        requested_port,
        true,
        response_status,
        accepted_connections_before,
        accepted_connections_after,
        status_incremented,
        stopped_after_smoke,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        blockers,
        warnings,
    ))
}

fn active_kernel_label() -> String {
    match CoreManager::global().get_running_mode().as_ref() {
        RunningMode::Service => "mihomo-service",
        RunningMode::Sidecar => "mihomo-sidecar",
        RunningMode::NotRunning => "not-running",
    }
    .into()
}

fn controller_transport_label(protocol: &Protocol) -> String {
    match protocol {
        Protocol::Http => "http".into(),
        Protocol::LocalSocket => "local-socket".into(),
        Protocol::Auto => "auto".into(),
    }
}

fn rust_owned_control_plane() -> Vec<String> {
    vec![
        "config-schema-validation".into(),
        "rule-engine".into(),
        "subscription-artifacts".into(),
        "app-runtime-plan".into(),
        "projection-artifact".into(),
        "staged-activation".into(),
        "runtime-apply-gates".into(),
        "audit-history".into(),
        "telemetry-wrappers".into(),
        "kernel-runtime-trait".into(),
    ]
}

fn mihomo_owned_data_plane() -> Vec<String> {
    vec![
        "protocol-stacks".into(),
        "tun-transparent-proxy".into(),
        "packet-forwarding".into(),
        "adapter-runtime".into(),
        "default-dns-runtime".into(),
    ]
}

fn blocked_replacement_areas() -> Vec<KernelReplacementBlocker> {
    vec![
        KernelReplacementBlocker {
            area: "tun-transparent-proxy".into(),
            reason: "requires platform rollback, leak verification, and Mihomo fallback before any live takeover"
                .into(),
            required_next_step: "rust-shadow-components".into(),
        },
        KernelReplacementBlocker {
            area: "protocol-stacks".into(),
            reason: "requires shadow adapter/protocol verification before forwarding traffic".into(),
            required_next_step: "shadow-adapter-capability-report".into(),
        },
        KernelReplacementBlocker {
            area: "default-dns-runtime".into(),
            reason: "must remain behind readiness, shadow evidence, opt-in execution, rollback drill, and hold history"
                .into(),
            required_next_step: "dns-shadow-evidence-continuation".into(),
        },
    ]
}

fn shadow_components() -> Vec<KernelShadowComponent> {
    vec![
        KernelShadowComponent {
            component: "dns-shadow-resolver".into(),
            kernel_area: "dns".into(),
            status: "evidence-command-available".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            evidence: vec![
                "compare Rust resolver answers against Mihomo/system answers before opt-in execution".into(),
                "must reuse DNS readiness, shadow evidence, rollback drill, and hold history".into(),
            ],
            next_step: "dns-shadow-resolver-evidence".into(),
        },
        KernelShadowComponent {
            component: "rule-shadow-classifier".into(),
            kernel_area: "rule-engine".into(),
            status: "evidence-command-available".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            evidence: vec![
                "compare Rust rule decisions with Mihomo rule inventory without routing traffic".into(),
                "must not create, delete, or disable live runtime rules".into(),
            ],
            next_step: "rule-shadow-classification-evidence".into(),
        },
        KernelShadowComponent {
            component: "adapter-capability-shadow".into(),
            kernel_area: "adapter".into(),
            status: "evidence-command-available".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            evidence: vec![
                "parse adapter capabilities before implementing Rust protocol stacks".into(),
                "must not open outbound sockets or forward packets".into(),
            ],
            next_step: "adapter-capability-report".into(),
        },
        KernelShadowComponent {
            component: "connection-observer-shadow".into(),
            kernel_area: "forwarding".into(),
            status: "evidence-command-available".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            evidence: vec![
                "observe connection/session shape before Rust forwarding takeover".into(),
                "must keep Mihomo as the only live forwarding owner".into(),
            ],
            next_step: "isolated-test-listener-preflight".into(),
        },
    ]
}

fn kernel_rule_shadow_rule(rule: &tauri_plugin_mihomo::models::Rule) -> KernelRuleShadowRule {
    KernelRuleShadowRule {
        index: rule.index,
        rule_type: format!("{:?}", rule.rule_type).into(),
        payload: rule.payload.clone().into(),
        proxy: rule.proxy.clone().into(),
        source: rule.source.clone().into(),
    }
}

fn rule_shadow_mismatch_reason(
    app_rule: Option<&KernelRuleShadowRule>,
    mihomo_rule: Option<&KernelRuleShadowRule>,
) -> Option<String> {
    match (app_rule, mihomo_rule) {
        (None, None) => None,
        (None, Some(_)) => Some("Mihomo has a rule where app projection has none".into()),
        (Some(_), None) => Some("app projection has a rule where Mihomo has none".into()),
        (Some(app), Some(mihomo)) => {
            if app.rule_type != mihomo.rule_type {
                Some(format!("rule type differs: app={} mihomo={}", app.rule_type, mihomo.rule_type).into())
            } else if app.payload != mihomo.payload {
                Some(format!("payload differs: app={} mihomo={}", app.payload, mihomo.payload).into())
            } else if app.proxy != mihomo.proxy {
                Some(format!("target differs: app={} mihomo={}", app.proxy, mihomo.proxy).into())
            } else {
                None
            }
        }
    }
}

fn proxy_type_counts(
    proxies: &std::collections::HashMap<std::string::String, tauri_plugin_mihomo::models::Proxy>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for proxy in proxies.values() {
        *counts.entry(proxy_type_label(proxy)).or_default() += 1;
    }
    counts
}

fn proxy_type_label(proxy: &tauri_plugin_mihomo::models::Proxy) -> String {
    format!("{:?}", proxy.proxy_type).into()
}

fn increment_count(counts: &mut BTreeMap<String, usize>, key: String) {
    *counts.entry(key).or_default() += 1;
}

fn connection_session_sample(
    (sample_index, session): (usize, &tauri_plugin_mihomo::models::Connection),
) -> KernelConnectionSessionSample {
    KernelConnectionSessionSample {
        sample_index,
        network: format!("{:?}", session.metadata.network).into(),
        connection_type: format!("{:?}", session.metadata.connection_type).into(),
        chain_len: session.chains.len(),
        provider_chain_len: session.provider_chains.as_ref().map(Vec::len).unwrap_or_default(),
        has_host: !session.metadata.host.is_empty(),
        has_process: !session.metadata.process.is_empty() || !session.metadata.process_path.is_empty(),
        has_remote_destination: !session.metadata.remote_destination.is_empty(),
        rule: session.rule.clone().into(),
        uploaded_bytes: session.upload,
        downloaded_bytes: session.download,
    }
}

fn kernel_runtime_ports(config: &serde_yaml_ng::Mapping) -> BTreeMap<String, u16> {
    let mut ports = BTreeMap::new();
    for key in ["port", "socks-port", "mixed-port", "redir-port", "tproxy-port"] {
        if let Some(port) = kernel_runtime_port(config, key) {
            ports.insert(key.into(), port);
        }
    }
    ports
}

fn kernel_runtime_port(config: &serde_yaml_ng::Mapping, key: &str) -> Option<u16> {
    config
        .get(key)
        .and_then(serde_yaml_ng::Value::as_i64)
        .and_then(|port| u16::try_from(port).ok())
        .filter(|port| *port > 0)
}

fn kernel_loopback_port_available(port: u16) -> bool {
    port > 0 && StdTcpListener::bind(("127.0.0.1", port)).is_ok()
}

fn isolated_test_listener_running_status() -> Option<KernelIsolatedTestListenerStatus> {
    let guard = ISOLATED_TEST_LISTENER.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref().map(|state| KernelIsolatedTestListenerStatus {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-test-listener-opt-in".into(),
        kernel_area: "listener".into(),
        mutates_runtime: true,
        live_execution_allowed: true,
        running: true,
        host: ISOLATED_TEST_LISTENER_HOST.into(),
        port: Some(state.port),
        started_at_epoch_ms: Some(state.started_at_epoch_ms),
        accepted_connections: state.accepted_connections.load(Ordering::Relaxed),
        loopback_only: true,
        default_route: false,
        forwards_traffic: false,
        mihomo_fallback: true,
        blockers: isolated_test_listener_blockers(),
        warnings: Vec::new(),
        facts: isolated_test_listener_facts(),
        next_safe_batch: "listener-smoke-evidence".into(),
    })
}

fn isolated_test_listener_status(warnings: Vec<String>) -> KernelIsolatedTestListenerStatus {
    isolated_test_listener_running_status().unwrap_or_else(|| KernelIsolatedTestListenerStatus {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-test-listener-opt-in".into(),
        kernel_area: "listener".into(),
        mutates_runtime: false,
        live_execution_allowed: true,
        running: false,
        host: ISOLATED_TEST_LISTENER_HOST.into(),
        port: None,
        started_at_epoch_ms: None,
        accepted_connections: 0,
        loopback_only: true,
        default_route: false,
        forwards_traffic: false,
        mihomo_fallback: true,
        blockers: isolated_test_listener_blockers(),
        warnings,
        facts: isolated_test_listener_facts(),
        next_safe_batch: "listener-smoke-evidence".into(),
    })
}

fn isolated_test_listener_blockers() -> Vec<String> {
    vec![
        "listener is loopback-only and must not be installed as the default proxy".into(),
        "listener must not attach to TUN, system proxy, DNS, or outbound forwarding".into(),
        "Mihomo remains the only production forwarding owner".into(),
    ]
}

fn isolated_test_listener_facts() -> Vec<String> {
    vec![
        "accepted connections receive an immediate local 204 response and are not proxied".into(),
        "start requires isolated listener preflight to pass for the selected port".into(),
    ]
}

fn current_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}

async fn isolated_test_listener_smoke_request(port: u16) -> Result<String> {
    let mut stream = timeout(
        Duration::from_secs(2),
        TcpStream::connect((ISOLATED_TEST_LISTENER_HOST, port)),
    )
    .await??;
    stream
        .write_all(b"GET /kernel-smoke HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n")
        .await?;
    let mut response = [0_u8; 128];
    let bytes_read = timeout(Duration::from_secs(2), stream.read(&mut response)).await??;
    let response = std::string::String::from_utf8_lossy(&response[..bytes_read]);
    Ok(response.lines().next().unwrap_or_default().into())
}

async fn kernel_runtime_config_snapshot() -> Result<Option<String>> {
    Config::runtime()
        .await
        .latest_arc()
        .config
        .as_ref()
        .map(serde_yaml_ng::to_string)
        .transpose()
        .map(|snapshot| snapshot.map(Into::into))
        .map_err(Into::into)
}

fn kernel_listener_smoke_report(
    requested_port: u16,
    started_by_smoke: bool,
    response_status: Option<String>,
    accepted_connections_before: u64,
    accepted_connections_after: u64,
    status_incremented: bool,
    stopped_after_smoke: bool,
    system_proxy_unchanged: bool,
    tun_unchanged: bool,
    runtime_config_unchanged: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
) -> KernelIsolatedTestListenerSmokeEvidenceReport {
    let passed = started_by_smoke
        && response_status.as_deref() == Some("HTTP/1.1 204 No Content")
        && status_incremented
        && stopped_after_smoke
        && system_proxy_unchanged
        && tun_unchanged
        && runtime_config_unchanged
        && blockers.is_empty();
    KernelIsolatedTestListenerSmokeEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "listener-smoke-evidence".into(),
        kernel_area: "listener".into(),
        mutates_runtime: started_by_smoke,
        live_execution_allowed: true,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        requested_port,
        started_by_smoke,
        response_status,
        accepted_connections_before,
        accepted_connections_after,
        status_incremented,
        stopped_after_smoke,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        default_route: false,
        forwards_traffic: false,
        mihomo_fallback: true,
        passed,
        blockers,
        warnings,
        facts: vec![
            "smoke evidence starts and stops only the loopback test listener".into(),
            "local smoke request must receive 204 and must not use outbound forwarding".into(),
            "runtime config, system proxy, and TUN settings are compared before and after".into(),
        ],
        next_safe_batch: "loopback-dns-or-forwarding-decision".into(),
    }
}
