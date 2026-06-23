use anyhow::{Result, bail};
use async_trait::async_trait;
use once_cell::sync::Lazy;
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
        dns_runtime::dns_default_runtime_shadow_evidence,
        handle::Handle,
        manager::RunningMode,
        runtime_snapshot::{build_proxies_from_runtime_config, build_rules_from_runtime_config},
    },
};

const MIHOMO_RUNTIME_ID: &str = "mihomo-kernel-runtime";
pub(super) const RUST_RUNTIME_ID: &str = "rust-kernel-runtime";
const NEXT_SAFE_BATCH: &str = "rust-shadow-components";
const NEXT_SHADOW_BATCH: &str = "loopback-test-listener-opt-in";
const ISOLATED_TEST_LISTENER_HOST: &str = "127.0.0.1";
const DEFAULT_ISOLATED_TEST_LISTENER_PORT: u16 = 19090;
const DEFAULT_LOOPBACK_DNS_PREFLIGHT_PORT: u16 = 19053;
const LOOPBACK_DNS_SMOKE_QUERY: &str = "kernel-smoke.invalid";
const DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT: u16 = 19180;
const DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT: u16 = 19181;
const LOOPBACK_PLATFORM_MATRIX_PLATFORMS: [&str; 3] = ["windows", "macos", "linux"];
const LOOPBACK_HOLD_WINDOW_MIN_SECONDS: u64 = 300;
const FULL_RUST_RUNTIME_HARDENING_MIN_SOAK_HOURS: u32 = 72;

static ISOLATED_TEST_LISTENER: Lazy<Mutex<Option<KernelIsolatedTestListenerState>>> = Lazy::new(|| Mutex::new(None));

mod data_plane_hardening;
mod default_data_plane_closeout;
mod default_forwarding_hold_blocker;
mod dns_cutover_hold_blocker;
mod dns_default_path_blocker;
mod dns_system_resolver_leak_blocker;
mod encrypted_protocols_bundle;
mod encrypted_proxy_protocol;
mod encrypted_proxy_session;
mod fallback_retirement_execution;
mod fallback_retirement_readiness;
mod go_retirement;
mod http_connect_proxy_adapter;
mod loopback_migration;
mod migration_final_review;
mod mihomo_fallback_retirement_bundle;
mod plugin_process_supervision_blocker;
mod protocol_adapter_forwarding;
mod protocol_default_path_blocker;
mod protocol_forwarding;
mod quic_udp_profile_blocker;
mod remote_adapter_transport;
mod route_packet_capture_blocker;
mod runtime_real_canary;
mod shadowsocks_aead_adapter;
mod shadowsocks_aead_canary;
mod sidecar_independent_rollback;
mod socks_auth_execution;
mod socks_bind_execution;
mod socks_tcp_connect_execution;
mod socks_udp_associate;
mod socks_udp_fragments;
mod tun_device_lifecycle_blocker;
mod tun_packet_capture_hold_bundle;
mod tun_system_proxy;
mod tun_transparent_routing;
mod types;
mod udp_plugin_transport_bundle;
pub use self::data_plane_hardening::*;
pub use self::default_data_plane_closeout::*;
pub use self::default_forwarding_hold_blocker::*;
pub use self::dns_cutover_hold_blocker::*;
pub use self::dns_default_path_blocker::*;
pub use self::dns_system_resolver_leak_blocker::*;
pub use self::encrypted_protocols_bundle::*;
pub use self::encrypted_proxy_protocol::*;
pub use self::encrypted_proxy_session::*;
pub use self::fallback_retirement_execution::*;
pub use self::fallback_retirement_readiness::*;
pub use self::go_retirement::*;
pub use self::http_connect_proxy_adapter::*;
pub use self::loopback_migration::*;
pub use self::migration_final_review::*;
pub use self::mihomo_fallback_retirement_bundle::*;
pub use self::plugin_process_supervision_blocker::*;
pub use self::protocol_adapter_forwarding::*;
pub use self::protocol_default_path_blocker::*;
pub use self::protocol_forwarding::*;
pub use self::quic_udp_profile_blocker::*;
pub use self::remote_adapter_transport::*;
pub use self::route_packet_capture_blocker::*;
pub use self::runtime_real_canary::*;
pub use self::shadowsocks_aead_adapter::*;
pub use self::shadowsocks_aead_canary::*;
pub use self::sidecar_independent_rollback::*;
pub use self::socks_auth_execution::*;
pub use self::socks_bind_execution::*;
pub use self::socks_tcp_connect_execution::*;
pub use self::socks_udp_associate::*;
pub use self::socks_udp_fragments::*;
pub use self::tun_device_lifecycle_blocker::*;
pub use self::tun_packet_capture_hold_bundle::*;
pub use self::tun_system_proxy::*;
pub use self::tun_transparent_routing::*;
use self::types::KernelIsolatedTestListenerState;
pub use self::types::*;
pub use self::udp_plugin_transport_bundle::*;

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

#[derive(Debug, Default)]
pub struct RustKernelRuntime;

#[async_trait]
impl KernelRuntime for RustKernelRuntime {
    fn runtime_id(&self) -> &'static str {
        RUST_RUNTIME_ID
    }

    async fn status(&self) -> KernelRuntimeStatus {
        KernelRuntimeStatus {
            runtime_id: self.runtime_id().into(),
            active_kernel: "rust-kernel-runtime-candidate".into(),
            controller_transport: "rust-runtime-scaffold".into(),
            mutates_runtime: false,
            mihomo_fallback: true,
        }
    }

    async fn replacement_readiness(&self) -> KernelReplacementReadiness {
        KernelReplacementReadiness {
            mutates_runtime: false,
            active_kernel: "rust-kernel-runtime-candidate".into(),
            controller_transport: "rust-runtime-scaffold".into(),
            rust_owned_control_plane: rust_owned_control_plane(),
            mihomo_owned_data_plane: rust_runtime_fallback_boundaries(),
            blocked_replacement_areas: blocked_replacement_areas(),
            next_safe_batch: "r6-rust-default-canary".into(),
        }
    }

    async fn shadow_components(&self) -> KernelShadowComponentsReport {
        KernelShadowComponentsReport {
            runtime_id: self.runtime_id().into(),
            active_kernel: "rust-kernel-runtime-candidate".into(),
            mutates_runtime: false,
            components: shadow_components(),
            live_execution_blockers: blocked_replacement_areas(),
            next_safe_batch: "r6-rust-default-canary".into(),
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
                "RustKernelRuntime is selectable only through explicit opt-in or canary gates".into(),
                "R6 default canary must keep automatic Mihomo fallback armed before any wider cutover".into(),
                "Unsupported protocol, TUN, and adapter paths remain Mihomo fallback boundaries".into(),
            ],
            blocked_replacement_areas: blocked_replacement_areas(),
            next_safe_batch: "r6-rust-default-canary".into(),
        }
    }
}

fn rust_runtime_supported_safe_subset() -> Vec<String> {
    vec![
        "rule-decision-projection".into(),
        "dns-decision-projection".into(),
        "adapter-capability-classification".into(),
        "loopback-only-forwarding-surface".into(),
        "health-and-rollback-state".into(),
    ]
}

fn rust_runtime_fallback_boundaries() -> Vec<String> {
    vec![
        "unsupported outbound and inbound protocols".into(),
        "TUN and transparent proxy packet capture".into(),
        "real adapter dialing and production egress".into(),
        "emergency rollback to Mihomo default".into(),
    ]
}

fn rust_runtime_capabilities() -> Vec<KernelRuntimeCapability> {
    vec![
        KernelRuntimeCapability {
            name: "runtimeSelection".into(),
            status: "scaffolded".into(),
            supported: true,
            fallback_required: false,
            facts: vec!["Mihomo and Rust runtime kinds can be represented without changing defaults".into()],
        },
        KernelRuntimeCapability {
            name: "supportedSubsetDecisionPath".into(),
            status: "opt-in-ready".into(),
            supported: true,
            fallback_required: true,
            facts: vec!["R6 MVP owns rule, DNS, and adapter decisions for the supported safe subset".into()],
        },
        KernelRuntimeCapability {
            name: "productionForwarding".into(),
            status: "fallback-required".into(),
            supported: false,
            fallback_required: true,
            facts: vec!["production forwarding still falls back to Mihomo outside the capped canary subset".into()],
        },
    ]
}

fn parse_kernel_runtime_kind(requested_runtime_kind: Option<String>) -> KernelRuntimeKind {
    let requested_runtime_kind = requested_runtime_kind.unwrap_or_default().trim().to_ascii_lowercase();
    match requested_runtime_kind.as_str() {
        "rust" | "rust-kernel-runtime" => KernelRuntimeKind::Rust,
        _ => KernelRuntimeKind::Mihomo,
    }
}

pub async fn rust_kernel_runtime_candidate_report() -> RustKernelRuntimeCandidateReport {
    RustKernelRuntimeCandidateReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        kind: KernelRuntimeKind::Rust,
        mutates_runtime: false,
        selectable: true,
        default_allowed: false,
        mihomo_fallback: true,
        supported_safe_subset: rust_runtime_supported_safe_subset(),
        fallback_boundaries: rust_runtime_fallback_boundaries(),
        capabilities: rust_runtime_capabilities(),
        blockers: vec!["Rust runtime cannot become default until canary and rollback evidence exist".into()],
        warnings: vec!["candidate remains non-default outside explicit opt-in or capped canary gates".into()],
        facts: vec![
            "Rust runtime MVP is selectable for the supported safe subset".into(),
            "Mihomo remains fallback for unsupported protocols, TUN, adapters, and rollback".into(),
        ],
        next_safe_batch: "r6-rust-default-canary".into(),
    }
}

pub async fn kernel_runtime_selection_scaffold(
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
) -> KernelRuntimeSelectionScaffoldReport {
    let requested_runtime_kind = parse_kernel_runtime_kind(requested_runtime_kind);
    let rust_runtime_opt_in_decision = rust_runtime_opt_in_decision.unwrap_or(false);
    let rust_candidate = rust_kernel_runtime_candidate_report().await;
    let mut blockers = Vec::new();
    let requested_rust = matches!(requested_runtime_kind, KernelRuntimeKind::Rust);

    if requested_rust && !rust_runtime_opt_in_decision {
        blockers.push("Rust runtime selection requires an explicit opt-in decision".into());
    }
    if !requested_rust && rust_runtime_opt_in_decision {
        blockers.push("Rust runtime opt-in decision requires requested_runtime_kind=rust".into());
    }
    let selected_runtime_kind = if requested_rust && rust_runtime_opt_in_decision && blockers.is_empty() {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };

    KernelRuntimeSelectionScaffoldReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "kernel-runtime-selection-scaffold".into(),
        mutates_runtime: false,
        current_default_runtime_kind: KernelRuntimeKind::Mihomo,
        selected_runtime_kind,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_candidate_available: true,
        rust_candidate_default_allowed: false,
        mihomo_fallback: true,
        rust_candidate,
        blockers,
        warnings: vec!["runtime selection keeps Mihomo as the non-canary default".into()],
        facts: vec![
            "Rust runtime selection is explicit and bounded to supported safe subset gates".into(),
            "R6 canary is the next step before any wider default cutover".into(),
        ],
        next_safe_batch: "r6-rust-default-canary".into(),
    }
}

async fn rust_kernel_runtime_supported_subset_report() -> Result<RustKernelRuntimeSupportedSubsetReport> {
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
    let app_rules = build_rules_from_runtime_config(config);
    let app_proxies = build_proxies_from_runtime_config(config);
    let app_rule_count = app_rules.rules.len();
    let app_proxy_count = app_proxies.proxies.len();
    let rule_decision_owned = app_rule_count > 0;
    let adapter_decision_owned = app_proxy_count > 0;
    let dns_decision_owned = true;
    let forwarding_surface_owned = true;
    let mut blockers = Vec::new();

    if !rule_decision_owned {
        blockers.push("Rust opt-in MVP requires at least one app-owned rule decision".into());
    }
    if !adapter_decision_owned {
        blockers.push("Rust opt-in MVP requires at least one app-owned adapter decision".into());
    }

    Ok(RustKernelRuntimeSupportedSubsetReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-rust-supported-subset".into(),
        mutates_runtime: false,
        rule_decision_owned,
        dns_decision_owned,
        adapter_decision_owned,
        forwarding_surface_owned,
        app_rule_count,
        app_proxy_count,
        supported_subset: rust_runtime_supported_safe_subset(),
        fallback_boundaries: rust_runtime_fallback_boundaries(),
        blockers,
        warnings: vec!["supported subset is opt-in metadata and loopback-only forwarding in this batch".into()],
        facts: vec![
            "rule and adapter decisions are derived from the Rust runtime config projection".into(),
            "DNS decision ownership remains bounded to existing Rust DNS planning/shadow state".into(),
        ],
    })
}

fn rust_kernel_runtime_health_state_report(
    opt_in_ready: bool,
    loopback_forwarding_evidence: Option<&KernelLoopbackForwardingRollbackDrillReport>,
) -> RustKernelRuntimeHealthStateReport {
    let loopback_healthy = loopback_forwarding_evidence.is_some_and(|evidence| evidence.passed);
    let health_ready = opt_in_ready && loopback_healthy;

    RustKernelRuntimeHealthStateReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-rust-runtime-health-state".into(),
        status: if health_ready { "ready" } else { "blocked" }.into(),
        health_ready,
        rollback_armed: true,
        mihomo_fallback: true,
        observed_checks: vec![
            "supported subset decision path".into(),
            "loopback forwarding rollback drill".into(),
            "Mihomo fallback boundary".into(),
        ],
        blockers: if health_ready {
            Vec::new()
        } else {
            vec!["Rust opt-in MVP health requires successful loopback rollback evidence".into()]
        },
        warnings: vec!["health state does not authorize default Rust runtime selection".into()],
        facts: vec![
            "Mihomo fallback remains armed for unsupported and emergency paths".into(),
            "rollback can return selection to Mihomo without retiring the sidecar".into(),
        ],
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
    let evidence = dns_default_runtime_shadow_evidence(yaml.map(Into::into), domain.map(Into::into)).await?;
    let mut blockers = evidence
        .blockers
        .iter()
        .map(|blocker| blocker.as_str().into())
        .collect::<Vec<String>>();
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
        warnings.push(
            format!(
                "app rule inventory has {} rule(s), Mihomo reports {} rule(s)",
                app_rules.rules.len(),
                mihomo_rules.rules.len()
            )
            .into(),
        );
    }
    if mismatched_sample_count > 0 {
        warnings.push(
            format!(
                "{} sampled rule position(s) differ between app and Mihomo inventory",
                mismatched_sample_count
            )
            .into(),
        );
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
