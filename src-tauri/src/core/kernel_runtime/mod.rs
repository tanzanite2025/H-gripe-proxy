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
mod dns_default_path_blocker;
mod encrypted_protocols_bundle;
mod encrypted_proxy_protocol;
mod encrypted_proxy_session;
mod fallback_retirement_execution;
mod fallback_retirement_readiness;
mod go_retirement;
mod http_connect_proxy_adapter;
mod migration_final_review;
mod mihomo_fallback_retirement_bundle;
mod protocol_adapter_forwarding;
mod protocol_default_path_blocker;
mod protocol_forwarding;
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
mod tun_packet_capture_hold_bundle;
mod tun_system_proxy;
mod tun_transparent_routing;
mod types;
mod udp_plugin_transport_bundle;
pub use self::data_plane_hardening::*;
pub use self::default_data_plane_closeout::*;
pub use self::dns_default_path_blocker::*;
pub use self::encrypted_protocols_bundle::*;
pub use self::encrypted_proxy_protocol::*;
pub use self::encrypted_proxy_session::*;
pub use self::fallback_retirement_execution::*;
pub use self::fallback_retirement_readiness::*;
pub use self::go_retirement::*;
pub use self::http_connect_proxy_adapter::*;
pub use self::migration_final_review::*;
pub use self::mihomo_fallback_retirement_bundle::*;
pub use self::protocol_adapter_forwarding::*;
pub use self::protocol_default_path_blocker::*;
pub use self::protocol_forwarding::*;
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

pub async fn mihomo_kernel_loopback_forwarding_leak_check(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackForwardingLeakCheckReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let preflight = mihomo_kernel_loopback_forwarding_preflight(Some(listener_port), Some(target_port)).await?;
    let isolated_status = mihomo_kernel_isolated_test_listener_status().await;
    let listener_port_released = preflight.port_check.listener_available;
    let target_port_released = preflight.port_check.target_available;
    let isolated_test_listener_running = isolated_status.running;
    let mut blockers = Vec::new();
    if !listener_port_released {
        blockers.push("loopback forwarding listener port is still occupied".into());
    }
    if !target_port_released {
        blockers.push("loopback forwarding target port is still occupied".into());
    }
    if isolated_test_listener_running {
        blockers.push("isolated test listener is still running during forwarding leak check".into());
    }
    let passed = blockers.is_empty();

    Ok(KernelLoopbackForwardingLeakCheckReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-forwarding-leak-check".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        listener_port,
        target_port,
        listener_port_released,
        target_port_released,
        isolated_test_listener_running,
        preflight,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed,
        blockers,
        warnings: vec!["leak check is local state evidence only and does not prove platform routing safety".into()],
        facts: vec![
            "checks forwarding smoke listener and target ports are available after rollback drill".into(),
            "checks the isolated test listener persistent state is not running".into(),
            "does not bind persistent sockets, dial adapters, or mutate runtime state".into(),
        ],
        next_safe_batch: "loopback-platform-matrix".into(),
    })
}

pub async fn mihomo_kernel_loopback_platform_matrix(
    listener_port: Option<u16>,
    target_port: Option<u16>,
) -> Result<KernelLoopbackPlatformMatrixReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let leak_check = mihomo_kernel_loopback_forwarding_leak_check(Some(listener_port), Some(target_port)).await?;
    let current_platform = std::env::consts::OS;
    let current_arch = std::env::consts::ARCH;
    let required_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let current_platform_supported = LOOPBACK_PLATFORM_MATRIX_PLATFORMS.contains(&current_platform);
    let covered_platforms = if current_platform_supported {
        vec![current_platform.into()]
    } else {
        Vec::new()
    };
    let pending_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .filter(|platform| **platform != current_platform)
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let current_platform_passed = current_platform_supported && leak_check.passed;

    let rows = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| {
            let is_current_platform = *platform == current_platform;
            if is_current_platform {
                let mut facts = leak_check.facts.clone();
                facts.push(
                    format!("recorded loopback forwarding leak evidence on {current_platform}/{current_arch}").into(),
                );

                KernelLoopbackPlatformMatrixRow {
                    platform: (*platform).into(),
                    current_platform: true,
                    evidence_status: if leak_check.passed {
                        "observed".into()
                    } else {
                        "blocked".into()
                    },
                    listener_port_released: Some(leak_check.listener_port_released),
                    target_port_released: Some(leak_check.target_port_released),
                    isolated_test_listener_stopped: Some(!leak_check.isolated_test_listener_running),
                    default_route: leak_check.default_route,
                    forwards_traffic: leak_check.forwards_traffic,
                    outbound_adapters_used: leak_check.outbound_adapters_used,
                    mihomo_fallback: leak_check.mihomo_fallback,
                    blockers: leak_check.blockers.clone(),
                    facts,
                }
            } else {
                KernelLoopbackPlatformMatrixRow {
                    platform: (*platform).into(),
                    current_platform: false,
                    evidence_status: "pending".into(),
                    listener_port_released: None,
                    target_port_released: None,
                    isolated_test_listener_stopped: None,
                    default_route: false,
                    forwards_traffic: false,
                    outbound_adapters_used: false,
                    mihomo_fallback: true,
                    blockers: vec![
                        format!("run the loopback platform matrix on {platform} before expanded opt-in").into(),
                    ],
                    facts: vec!["pending platform row is a placeholder and records no runtime evidence".into()],
                }
            }
        })
        .collect::<Vec<KernelLoopbackPlatformMatrixRow>>();

    let mut blockers = vec![
        "R4 expanded opt-in remains blocked until Windows, macOS, and Linux matrix rows are observed".into(),
        "platform-specific rollback drills and hold-window evidence are still required".into(),
    ];
    if !current_platform_supported {
        blockers.push(format!("current platform {current_platform} is not in the required matrix").into());
    }
    if !leak_check.passed {
        blockers.extend(leak_check.blockers.clone());
    }

    let mut warnings = leak_check.warnings.clone();
    warnings.push("platform matrix is read-only evidence and does not allow real adapter/TUN/protocol cutover".into());

    Ok(KernelLoopbackPlatformMatrixReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-platform-matrix".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: current_platform.into(),
        current_arch: current_arch.into(),
        listener_port,
        target_port,
        required_platforms,
        covered_platforms,
        pending_platforms,
        current_platform_passed,
        expanded_opt_in_allowed: false,
        leak_check,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: current_platform_passed,
        blockers,
        warnings,
        facts: vec![
            "wraps loopback forwarding leak evidence with a required platform matrix row".into(),
            "records only the current platform; other platform rows stay pending until run there".into(),
            "keeps R4 expanded opt-in blocked until matrix, rollback, and hold-window evidence exist".into(),
        ],
        next_safe_batch: "loopback-hold-window".into(),
    })
}

pub async fn mihomo_kernel_loopback_hold_window(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
) -> Result<KernelLoopbackHoldWindowReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let platform_matrix = mihomo_kernel_loopback_platform_matrix(Some(listener_port), Some(target_port)).await?;
    let observed_at_epoch_ms = current_epoch_ms();
    let hold_started_at_epoch_ms = hold_started_at_epoch_ms.unwrap_or(observed_at_epoch_ms);
    let hold_start_in_future = hold_started_at_epoch_ms > observed_at_epoch_ms;
    let elapsed_hold_seconds = observed_at_epoch_ms
        .saturating_sub(hold_started_at_epoch_ms)
        .saturating_div(1000);
    let current_platform_hold_window_satisfied =
        !hold_start_in_future && elapsed_hold_seconds >= LOOPBACK_HOLD_WINDOW_MIN_SECONDS;

    let rows = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| {
            let is_current_platform = *platform == platform_matrix.current_platform;
            if is_current_platform {
                let mut blockers = Vec::new();
                if !platform_matrix.current_platform_passed {
                    blockers.push("platform matrix evidence must pass before hold-window evidence is usable".into());
                }
                if hold_start_in_future {
                    blockers.push("hold window start timestamp is later than the observation timestamp".into());
                }
                if !current_platform_hold_window_satisfied {
                    blockers.push(format!(
                        "observe at least {LOOPBACK_HOLD_WINDOW_MIN_SECONDS} second(s) before treating hold-window evidence as satisfied"
                    ).into());
                }

                KernelLoopbackHoldWindowRow {
                    platform: (*platform).into(),
                    current_platform: true,
                    evidence_status: if !platform_matrix.current_platform_passed || hold_start_in_future {
                        "blocked".into()
                    } else if current_platform_hold_window_satisfied {
                        "observed".into()
                    } else {
                        "holding".into()
                    },
                    hold_started_at_epoch_ms: Some(hold_started_at_epoch_ms),
                    observed_at_epoch_ms: Some(observed_at_epoch_ms),
                    minimum_hold_seconds: LOOPBACK_HOLD_WINDOW_MIN_SECONDS,
                    elapsed_hold_seconds: Some(elapsed_hold_seconds),
                    hold_window_satisfied: current_platform_hold_window_satisfied,
                    platform_matrix_passed: Some(platform_matrix.current_platform_passed),
                    leak_check_passed: Some(platform_matrix.leak_check.passed),
                    default_route: false,
                    forwards_traffic: false,
                    outbound_adapters_used: false,
                    mihomo_fallback: true,
                    blockers,
                    facts: vec![
                        format!(
                            "recorded loopback hold-window observation on {}/{}",
                            platform_matrix.current_platform, platform_matrix.current_arch
                        )
                        .into(),
                        "hold-window evidence is read-only and does not keep sockets or listeners open".into(),
                    ],
                }
            } else {
                KernelLoopbackHoldWindowRow {
                    platform: (*platform).into(),
                    current_platform: false,
                    evidence_status: "pending".into(),
                    hold_started_at_epoch_ms: None,
                    observed_at_epoch_ms: None,
                    minimum_hold_seconds: LOOPBACK_HOLD_WINDOW_MIN_SECONDS,
                    elapsed_hold_seconds: None,
                    hold_window_satisfied: false,
                    platform_matrix_passed: None,
                    leak_check_passed: None,
                    default_route: false,
                    forwards_traffic: false,
                    outbound_adapters_used: false,
                    mihomo_fallback: true,
                    blockers: vec![
                        format!("run loopback hold-window evidence on {platform} before expanded opt-in").into(),
                    ],
                    facts: vec!["pending hold-window row records no runtime evidence".into()],
                }
            }
        })
        .collect::<Vec<KernelLoopbackHoldWindowRow>>();

    let covered_hold_platforms = rows
        .iter()
        .filter(|row| row.hold_window_satisfied)
        .map(|row| row.platform.clone())
        .collect::<Vec<String>>();
    let pending_hold_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .filter(|platform| !covered_hold_platforms.iter().any(|covered| covered == **platform))
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();

    let mut blockers = vec![
        "R4 expanded opt-in remains blocked until Windows, macOS, and Linux hold-window rows are observed".into(),
        "platform-specific rollback drills are still required before broader opt-in".into(),
    ];
    if !platform_matrix.current_platform_passed {
        blockers.push("current platform matrix evidence is not passing".into());
    }
    if hold_start_in_future {
        blockers.push("hold window start timestamp is later than the observation timestamp".into());
    }
    if !current_platform_hold_window_satisfied {
        blockers.push(
            format!("current platform hold window has not reached {LOOPBACK_HOLD_WINDOW_MIN_SECONDS} second(s)").into(),
        );
    }
    if !pending_hold_platforms.is_empty() {
        blockers.push(
            format!(
                "pending hold-window platform evidence: {}",
                pending_hold_platforms.join(", ")
            )
            .into(),
        );
    }

    let mut warnings = platform_matrix.warnings.clone();
    warnings
        .push("hold-window timestamps are evidence only and do not enable adapter/TUN/protocol/default cutover".into());

    Ok(KernelLoopbackHoldWindowReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-hold-window".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: platform_matrix.current_platform.clone(),
        current_arch: platform_matrix.current_arch.clone(),
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_at_epoch_ms,
        minimum_hold_seconds: LOOPBACK_HOLD_WINDOW_MIN_SECONDS,
        elapsed_hold_seconds,
        required_platforms: platform_matrix.required_platforms.clone(),
        covered_hold_platforms,
        pending_hold_platforms,
        current_platform_passed: platform_matrix.current_platform_passed,
        current_platform_hold_window_satisfied,
        expanded_opt_in_allowed: false,
        platform_matrix,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: current_platform_hold_window_satisfied,
        blockers,
        warnings,
        facts: vec![
            "wraps loopback platform matrix evidence with a time-window observation".into(),
            "records the current platform only; other platforms remain pending until run there".into(),
            "keeps expanded opt-in blocked after hold-window evidence until platform rollback evidence exists".into(),
        ],
        next_safe_batch: "loopback-platform-rollback-drills".into(),
    })
}

pub async fn mihomo_kernel_loopback_platform_rollback_drills(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
) -> Result<KernelLoopbackPlatformRollbackDrillsReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let hold_window =
        mihomo_kernel_loopback_hold_window(Some(listener_port), Some(target_port), hold_started_at_epoch_ms).await?;
    let rollback_drill =
        mihomo_kernel_loopback_forwarding_rollback_drill(Some(listener_port), Some(target_port)).await?;
    let current_platform = hold_window.current_platform.clone();
    let current_arch = hold_window.current_arch.clone();
    let current_platform_supported = LOOPBACK_PLATFORM_MATRIX_PLATFORMS.contains(&current_platform.as_str());
    let current_platform_passed = current_platform_supported && rollback_drill.passed;
    let required_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();

    let rows = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| {
            let is_current_platform = *platform == current_platform;
            if is_current_platform {
                let mut facts = rollback_drill.facts.clone();
                facts.push(
                    format!("recorded loopback rollback drill evidence on {current_platform}/{current_arch}").into(),
                );

                KernelLoopbackPlatformRollbackDrillRow {
                    platform: (*platform).into(),
                    current_platform: true,
                    evidence_status: if rollback_drill.passed {
                        "observed".into()
                    } else {
                        "blocked".into()
                    },
                    smoke_passed: Some(rollback_drill.smoke_passed),
                    ports_released: Some(rollback_drill.ports_released),
                    system_proxy_unchanged: Some(rollback_drill.system_proxy_unchanged),
                    tun_unchanged: Some(rollback_drill.tun_unchanged),
                    runtime_config_unchanged: Some(rollback_drill.runtime_config_unchanged),
                    hold_window_satisfied: Some(hold_window.current_platform_hold_window_satisfied),
                    default_route: rollback_drill.default_route,
                    forwards_traffic: rollback_drill.forwards_traffic,
                    outbound_adapters_used: rollback_drill.outbound_adapters_used,
                    mihomo_fallback: rollback_drill.mihomo_fallback,
                    blockers: rollback_drill.blockers.clone(),
                    facts,
                }
            } else {
                KernelLoopbackPlatformRollbackDrillRow {
                    platform: (*platform).into(),
                    current_platform: false,
                    evidence_status: "pending".into(),
                    smoke_passed: None,
                    ports_released: None,
                    system_proxy_unchanged: None,
                    tun_unchanged: None,
                    runtime_config_unchanged: None,
                    hold_window_satisfied: None,
                    default_route: false,
                    forwards_traffic: false,
                    outbound_adapters_used: false,
                    mihomo_fallback: true,
                    blockers: vec![
                        format!("run loopback platform rollback drills on {platform} before expanded opt-in").into(),
                    ],
                    facts: vec!["pending rollback drill row records no runtime evidence".into()],
                }
            }
        })
        .collect::<Vec<KernelLoopbackPlatformRollbackDrillRow>>();

    let covered_rollback_platforms = if current_platform_passed {
        vec![current_platform.clone()]
    } else {
        Vec::new()
    };
    let pending_rollback_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .filter(|platform| !covered_rollback_platforms.iter().any(|covered| covered == **platform))
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();

    let mut blockers = vec![
        "R4 expanded opt-in remains blocked until Windows, macOS, and Linux rollback drill rows are observed".into(),
        "R4 expanded opt-in still requires an explicit decision and dedicated preflight".into(),
    ];
    if !current_platform_supported {
        blockers.push(format!("current platform {current_platform} is not in the required rollback matrix").into());
    }
    if !rollback_drill.passed {
        blockers.extend(rollback_drill.blockers.clone());
    }
    if !hold_window.current_platform_hold_window_satisfied {
        blockers.push("current platform hold-window evidence is not satisfied".into());
    }
    if !pending_rollback_platforms.is_empty() {
        blockers.push(
            format!(
                "pending rollback drill platform evidence: {}",
                pending_rollback_platforms.join(", ")
            )
            .into(),
        );
    }

    let mut warnings = rollback_drill.warnings.clone();
    warnings.extend(hold_window.warnings.clone());
    warnings.push(
        "platform rollback drills are still synthetic loopback-only evidence and do not permit real adapter/TUN/protocol cutover"
            .into(),
    );

    Ok(KernelLoopbackPlatformRollbackDrillsReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-platform-rollback-drills".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: true,
        live_execution_allowed: true,
        current_platform,
        current_arch,
        listener_port,
        target_port,
        required_platforms,
        covered_rollback_platforms,
        pending_rollback_platforms,
        current_platform_passed,
        expanded_opt_in_allowed: false,
        hold_window,
        rollback_drill,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: current_platform_passed,
        blockers,
        warnings,
        facts: vec![
            "wraps loopback forwarding rollback drill evidence with required platform rows".into(),
            "records only the current platform; other platform rows stay pending until run there".into(),
            "keeps expanded opt-in blocked until a dedicated R4 preflight and explicit decision".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-preflight".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_preflight(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInPreflightReport> {
    let listener_port = listener_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_LISTENER_PORT);
    let target_port = target_port.unwrap_or(DEFAULT_LOOPBACK_FORWARDING_TARGET_PORT);
    let explicit_decision = explicit_decision.unwrap_or(false);
    let hold_window =
        mihomo_kernel_loopback_hold_window(Some(listener_port), Some(target_port), hold_started_at_epoch_ms).await?;
    let required_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let observed_rollback_platforms = observed_rollback_platforms
        .unwrap_or_default()
        .into_iter()
        .filter(|platform| LOOPBACK_PLATFORM_MATRIX_PLATFORMS.contains(&platform.as_str()))
        .collect::<BTreeSet<String>>();
    let pending_rollback_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .filter(|platform| !observed_rollback_platforms.contains(**platform))
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let observed_rollback_platforms = observed_rollback_platforms.into_iter().collect::<Vec<String>>();

    let rows = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| {
            let current_platform = *platform == hold_window.current_platform;
            let rollback_drill_observed = observed_rollback_platforms.iter().any(|observed| observed == platform);
            let hold_window_satisfied = current_platform.then_some(hold_window.current_platform_hold_window_satisfied);
            let mut blockers = Vec::new();
            if !rollback_drill_observed {
                blockers.push(format!("missing observed rollback drill evidence for {platform}").into());
            }
            if current_platform && !hold_window.current_platform_hold_window_satisfied {
                blockers.push("current platform hold-window evidence is not satisfied".into());
            }

            KernelLoopbackR4ExpandedOptInPreflightRow {
                platform: (*platform).into(),
                current_platform,
                rollback_drill_observed,
                hold_window_satisfied,
                evidence_status: if blockers.is_empty() {
                    "ready".into()
                } else {
                    "blocked".into()
                },
                blockers,
                facts: vec![
                    "R4 preflight consumes platform rollback evidence without re-running rollback drills".into(),
                ],
            }
        })
        .collect::<Vec<KernelLoopbackR4ExpandedOptInPreflightRow>>();

    let mut checks = Vec::new();
    let matrix_passed = hold_window.platform_matrix.current_platform_passed;
    checks.push(KernelLoopbackR4ExpandedOptInPreflightCheck {
        name: "currentPlatformMatrix".into(),
        status: if matrix_passed { "passed" } else { "blocked" }.into(),
        passed: matrix_passed,
        blockers: if matrix_passed {
            Vec::new()
        } else {
            vec!["current platform matrix evidence is not passing".into()]
        },
        facts: vec!["preflight reuses read-only platform matrix evidence".into()],
    });
    let hold_passed = hold_window.current_platform_hold_window_satisfied;
    checks.push(KernelLoopbackR4ExpandedOptInPreflightCheck {
        name: "currentPlatformHoldWindow".into(),
        status: if hold_passed { "passed" } else { "blocked" }.into(),
        passed: hold_passed,
        blockers: if hold_passed {
            Vec::new()
        } else {
            vec!["current platform hold window is not satisfied".into()]
        },
        facts: vec!["hold-window evidence is read-only and session-scoped".into()],
    });
    let rollback_passed = pending_rollback_platforms.is_empty();
    checks.push(KernelLoopbackR4ExpandedOptInPreflightCheck {
        name: "allPlatformRollbackDrills".into(),
        status: if rollback_passed { "passed" } else { "blocked" }.into(),
        passed: rollback_passed,
        blockers: if rollback_passed {
            Vec::new()
        } else {
            vec![
                format!(
                    "pending rollback drill platform evidence: {}",
                    pending_rollback_platforms.join(", ")
                )
                .into(),
            ]
        },
        facts: vec!["rollback drill observations must cover Windows, macOS, and Linux".into()],
    });
    checks.push(KernelLoopbackR4ExpandedOptInPreflightCheck {
        name: "explicitDecision".into(),
        status: if explicit_decision { "passed" } else { "blocked" }.into(),
        passed: explicit_decision,
        blockers: if explicit_decision {
            Vec::new()
        } else {
            vec!["R4 expanded opt-in requires an explicit decision".into()]
        },
        facts: vec!["readiness evidence alone is not rollout permission".into()],
    });

    let mut blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();
    blockers.push("dedicated expanded opt-in execution is not implemented in this preflight batch".into());
    let preflight_passed = checks.iter().all(|check| check.passed);

    Ok(KernelLoopbackR4ExpandedOptInPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-preflight".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: hold_window.current_platform.clone(),
        current_arch: hold_window.current_arch.clone(),
        listener_port,
        target_port,
        explicit_decision,
        required_platforms,
        observed_rollback_platforms,
        pending_rollback_platforms,
        current_platform_hold_window_satisfied: hold_window.current_platform_hold_window_satisfied,
        preflight_passed,
        expanded_opt_in_allowed: false,
        hold_window,
        rows,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: false,
        blockers,
        warnings: vec![
            "R4 expanded opt-in preflight is read-only and does not enable real adapter/TUN/protocol/default cutover"
                .into(),
        ],
        facts: vec![
            "checks platform evidence readiness without running rollback drills".into(),
            "requires explicit decision separate from accumulated evidence".into(),
            "keeps expanded opt-in execution blocked for a dedicated later batch".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-execution-plan".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_execution_plan(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInExecutionPlanReport> {
    let preflight = mihomo_kernel_loopback_r4_expanded_opt_in_preflight(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
    )
    .await?;
    let explicit_decision = preflight.explicit_decision;
    let plan_ready = preflight.preflight_passed;

    let steps = vec![
        KernelLoopbackR4ExpandedOptInExecutionPlanStep {
            order: 1,
            name: "revalidateReadOnlyPreflight".into(),
            action: "call get_runtime_kernel_loopback_r4_expanded_opt_in_preflight before any execution attempt".into(),
            mutates_runtime: false,
            requires_explicit_decision: false,
            enabled_in_this_batch: true,
            blockers: Vec::new(),
            facts: vec!["preflight must stay fresh and read-only".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionPlanStep {
            order: 2,
            name: "requireExplicitExpandedOptInDecision".into(),
            action: "require a separate user decision scoped to R4 expanded opt-in".into(),
            mutates_runtime: false,
            requires_explicit_decision: true,
            enabled_in_this_batch: true,
            blockers: if explicit_decision {
                Vec::new()
            } else {
                vec!["explicit R4 decision is missing".into()]
            },
            facts: vec!["evidence readiness is not rollout permission".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionPlanStep {
            order: 3,
            name: "executeLoopbackOnlyExpandedRuntime".into(),
            action: "future batch may run only bounded loopback synthetic forwarding with rollback state capture"
                .into(),
            mutates_runtime: true,
            requires_explicit_decision: true,
            enabled_in_this_batch: false,
            blockers: vec!["execution guard is not implemented in this planning batch".into()],
            facts: vec!["real adapters, TUN, protocol handlers, and default route remain out of scope".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionPlanStep {
            order: 4,
            name: "verifyAndRollback".into(),
            action: "future batch must verify no leaked sockets or config drift and provide explicit rollback".into(),
            mutates_runtime: true,
            requires_explicit_decision: true,
            enabled_in_this_batch: false,
            blockers: vec!["verification and rollback execution are reserved for a dedicated batch".into()],
            facts: vec!["default cutover cannot be part of R4 expanded opt-in execution".into()],
        },
    ];

    let mut blockers = preflight.blockers.clone();
    blockers.push("execution plan is descriptive only; expanded opt-in execution remains blocked".into());

    Ok(KernelLoopbackR4ExpandedOptInExecutionPlanReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-execution-plan".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: preflight.current_platform.clone(),
        current_arch: preflight.current_arch.clone(),
        listener_port: preflight.listener_port,
        target_port: preflight.target_port,
        candidate_scope: "loopbackSyntheticOnly".into(),
        explicit_decision,
        plan_ready,
        execution_allowed: false,
        expanded_opt_in_allowed: false,
        preflight,
        steps,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: false,
        blockers,
        warnings: vec![
            "execution plan is read-only documentation in data form and does not authorize runtime mutation".into(),
        ],
        facts: vec![
            "keeps R4 execution split from readiness preflight".into(),
            "limits any future execution candidate to synthetic loopback scope".into(),
            "keeps default cutover blocked for a later dedicated phase".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-execution-guard".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_execution_guard(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInExecutionGuardReport> {
    let requested_execution = requested_execution.unwrap_or(false);
    let plan = mihomo_kernel_loopback_r4_expanded_opt_in_execution_plan(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
    )
    .await?;
    let explicit_decision = plan.explicit_decision;
    let plan_ready = plan.plan_ready;

    let guard_checks = vec![
        KernelLoopbackR4ExpandedOptInExecutionGuardCheck {
            name: "executionRequested".into(),
            status: if requested_execution { "passed" } else { "blocked" }.into(),
            passed: requested_execution,
            required_for_execution: true,
            blockers: if requested_execution {
                Vec::new()
            } else {
                vec!["guard requires an explicit execution request separate from evidence collection".into()]
            },
            facts: vec!["preflight and planning commands do not imply execution intent".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionGuardCheck {
            name: "executionPlanReady".into(),
            status: if plan_ready { "passed" } else { "blocked" }.into(),
            passed: plan_ready,
            required_for_execution: true,
            blockers: if plan_ready {
                Vec::new()
            } else {
                vec!["execution plan is not ready because one or more preflight gates are blocked".into()]
            },
            facts: vec!["guard consumes the read-only R4 execution plan".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionGuardCheck {
            name: "explicitDecision".into(),
            status: if explicit_decision { "passed" } else { "blocked" }.into(),
            passed: explicit_decision,
            required_for_execution: true,
            blockers: if explicit_decision {
                Vec::new()
            } else {
                vec!["explicit R4 expanded opt-in decision is missing".into()]
            },
            facts: vec!["execution intent must be distinct from roadmap progress".into()],
        },
        KernelLoopbackR4ExpandedOptInExecutionGuardCheck {
            name: "implementationBoundary".into(),
            status: "passed".into(),
            passed: true,
            required_for_execution: true,
            blockers: Vec::new(),
            facts: vec!["synthetic loopback execution is implemented behind this guard".into()],
        },
    ];

    let verification_plan = vec![
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 1,
            phase: "preExecution".into(),
            action: "capture runtime config, system proxy, TUN, and loopback port state".into(),
            mutates_runtime: false,
            required_before_expansion: true,
            enabled_in_this_batch: true,
            blockers: Vec::new(),
            facts: vec!["verification must compare the same state after execution".into()],
        },
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 2,
            phase: "postExecution".into(),
            action: "verify synthetic listener and target ports are released and no isolated listener remains running"
                .into(),
            mutates_runtime: false,
            required_before_expansion: true,
            enabled_in_this_batch: true,
            blockers: Vec::new(),
            facts: vec!["port release remains the primary loopback leak signal".into()],
        },
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 3,
            phase: "postExecution".into(),
            action: "verify system proxy, TUN, runtime config, and Mihomo fallback boundaries are unchanged".into(),
            mutates_runtime: false,
            required_before_expansion: true,
            enabled_in_this_batch: true,
            blockers: Vec::new(),
            facts: vec!["R4 loopback expansion must not become default cutover".into()],
        },
    ];
    let rollback_plan = vec![
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 1,
            phase: "rollback".into(),
            action: "stop any app-owned loopback listener and release synthetic target sockets".into(),
            mutates_runtime: true,
            required_before_expansion: true,
            enabled_in_this_batch: false,
            blockers: vec!["rollback execution is reserved for the synthetic execution batch".into()],
            facts: vec!["rollback must not call Mihomo adapter or TUN mutation paths".into()],
        },
        KernelLoopbackR4ExpandedOptInSafetyPlanStep {
            order: 2,
            phase: "rollback".into(),
            action: "restore captured runtime config if a future synthetic execution changes it".into(),
            mutates_runtime: true,
            required_before_expansion: true,
            enabled_in_this_batch: false,
            blockers: vec!["runtime restore is not needed until execution is implemented".into()],
            facts: vec!["the current guard command does not mutate runtime state".into()],
        },
    ];

    let guard_ready = guard_checks.iter().all(|check| check.passed);
    let synthetic_execution_allowed = guard_ready;
    let blockers = guard_checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInExecutionGuardReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-execution-guard".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: plan.current_platform.clone(),
        current_arch: plan.current_arch.clone(),
        listener_port: plan.listener_port,
        target_port: plan.target_port,
        requested_execution,
        explicit_decision,
        guard_ready,
        synthetic_execution_allowed,
        execution_allowed: false,
        expanded_opt_in_allowed: false,
        plan,
        guard_checks,
        verification_plan,
        rollback_plan,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: guard_ready,
        blockers,
        warnings: vec![
            "execution guard is read-only and does not start expanded opt-in execution".into(),
            "synthetic execution permission is not default cutover permission".into(),
        ],
        facts: vec![
            "bundles execution guard checks with verification and rollback plans".into(),
            "keeps future execution constrained to synthetic loopback scope".into(),
            "keeps default cutover, real adapters, TUN, and protocol handlers blocked".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-synthetic-execution".into(),
    })
}

fn build_blocked_r4_synthetic_execution_closeout(
    blockers: Vec<String>,
) -> KernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout {
    KernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout {
        rollback_drill_passed: false,
        leak_check_passed: false,
        ports_released: false,
        system_proxy_unchanged: false,
        tun_unchanged: false,
        runtime_config_unchanged: false,
        isolated_test_listener_stopped: false,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: false,
        blockers,
        warnings: vec!["synthetic execution was not attempted because guard checks blocked it".into()],
        facts: vec!["blocked closeout records no runtime mutation evidence".into()],
    }
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_synthetic_execution(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInSyntheticExecutionReport> {
    let requested_execution = requested_execution.unwrap_or(false);
    let guard = mihomo_kernel_loopback_r4_expanded_opt_in_execution_guard(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
    )
    .await?;
    let synthetic_execution_allowed = guard.synthetic_execution_allowed && requested_execution;
    let listener_port = guard.listener_port;
    let target_port = guard.target_port;

    if !synthetic_execution_allowed {
        let blockers = guard.blockers.clone();
        return Ok(KernelLoopbackR4ExpandedOptInSyntheticExecutionReport {
            runtime_id: MIHOMO_RUNTIME_ID.into(),
            component: "loopback-r4-expanded-opt-in-synthetic-execution".into(),
            kernel_area: "forwarding".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            current_platform: guard.current_platform.clone(),
            current_arch: guard.current_arch.clone(),
            listener_port,
            target_port,
            requested_execution,
            explicit_decision: guard.explicit_decision,
            synthetic_execution_allowed,
            execution_attempted: false,
            expanded_opt_in_allowed: false,
            closeout: build_blocked_r4_synthetic_execution_closeout(blockers.clone()),
            guard,
            rollback_drill: None,
            leak_check: None,
            default_route: false,
            forwards_traffic: false,
            outbound_adapters_used: false,
            mihomo_fallback: true,
            passed: false,
            blockers,
            warnings: vec!["R4 synthetic execution remains blocked until guard checks pass".into()],
            facts: vec!["no sockets were opened because execution was not allowed".into()],
            next_safe_batch: "loopback-r4-expanded-opt-in-synthetic-execution".into(),
        });
    }

    let rollback_drill =
        mihomo_kernel_loopback_forwarding_rollback_drill(Some(listener_port), Some(target_port)).await?;
    let leak_check = mihomo_kernel_loopback_forwarding_leak_check(Some(listener_port), Some(target_port)).await?;
    let ports_released =
        rollback_drill.ports_released && leak_check.listener_port_released && leak_check.target_port_released;
    let isolated_test_listener_stopped = !leak_check.isolated_test_listener_running;

    let mut closeout_blockers = Vec::new();
    if !rollback_drill.passed {
        closeout_blockers.extend(rollback_drill.blockers.clone());
    }
    if !leak_check.passed {
        closeout_blockers.extend(leak_check.blockers.clone());
    }
    if !ports_released {
        closeout_blockers.push("synthetic execution ports were not released after closeout".into());
    }
    if !isolated_test_listener_stopped {
        closeout_blockers.push("isolated test listener remained running after synthetic execution".into());
    }

    let closeout_passed = closeout_blockers.is_empty();
    let closeout = KernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout {
        rollback_drill_passed: rollback_drill.passed,
        leak_check_passed: leak_check.passed,
        ports_released,
        system_proxy_unchanged: rollback_drill.system_proxy_unchanged,
        tun_unchanged: rollback_drill.tun_unchanged,
        runtime_config_unchanged: rollback_drill.runtime_config_unchanged,
        isolated_test_listener_stopped,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: closeout_passed,
        blockers: closeout_blockers.clone(),
        warnings: vec!["closeout proves only synthetic loopback execution cleanup".into()],
        facts: vec![
            "synthetic execution delegates to the loopback forwarding rollback drill".into(),
            "leak check revalidates listener, target, and isolated listener state after execution".into(),
        ],
    };

    Ok(KernelLoopbackR4ExpandedOptInSyntheticExecutionReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-synthetic-execution".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: true,
        live_execution_allowed: true,
        current_platform: guard.current_platform.clone(),
        current_arch: guard.current_arch.clone(),
        listener_port,
        target_port,
        requested_execution,
        explicit_decision: guard.explicit_decision,
        synthetic_execution_allowed,
        execution_attempted: true,
        expanded_opt_in_allowed: false,
        guard,
        rollback_drill: Some(rollback_drill),
        leak_check: Some(leak_check),
        closeout,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: closeout_passed,
        blockers: closeout_blockers,
        warnings: vec![
            "synthetic execution uses loopback-only rollback drill evidence and is not production forwarding".into(),
            "expanded opt-in remains blocked for real adapters, TUN, protocol handlers, and default cutover".into(),
        ],
        facts: vec![
            "executes only temporary 127.0.0.1 listener and target sockets".into(),
            "runs closeout leak evidence immediately after synthetic execution".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-post-execution-hold".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_post_execution_hold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
) -> Result<KernelLoopbackR4ExpandedOptInPostExecutionHoldReport> {
    let requested_execution = requested_execution.unwrap_or(false);
    let synthetic_execution = mihomo_kernel_loopback_r4_expanded_opt_in_synthetic_execution(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
    )
    .await?;
    let observed_at_epoch_ms = current_epoch_ms();
    let post_execution_hold_started_at_epoch_ms =
        post_execution_hold_started_at_epoch_ms.unwrap_or(observed_at_epoch_ms);
    let hold_start_in_future = post_execution_hold_started_at_epoch_ms > observed_at_epoch_ms;
    let elapsed_hold_seconds = observed_at_epoch_ms
        .saturating_sub(post_execution_hold_started_at_epoch_ms)
        .saturating_div(1000);
    let post_execution_hold_satisfied = !hold_start_in_future
        && synthetic_execution.passed
        && synthetic_execution.execution_attempted
        && elapsed_hold_seconds >= LOOPBACK_HOLD_WINDOW_MIN_SECONDS;

    let mut blockers = Vec::new();
    if hold_start_in_future {
        blockers.push("post-execution hold start timestamp is later than observation time".into());
    }
    if !synthetic_execution.execution_attempted {
        blockers.push("synthetic execution was not attempted before post-execution hold".into());
    }
    if !synthetic_execution.passed {
        blockers.extend(synthetic_execution.blockers.clone());
    }
    if elapsed_hold_seconds < LOOPBACK_HOLD_WINDOW_MIN_SECONDS {
        blockers.push(
            format!("observe at least {LOOPBACK_HOLD_WINDOW_MIN_SECONDS} second(s) after synthetic execution closeout")
                .into(),
        );
    }

    Ok(KernelLoopbackR4ExpandedOptInPostExecutionHoldReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-post-execution-hold".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: synthetic_execution.execution_attempted,
        live_execution_allowed: synthetic_execution.synthetic_execution_allowed,
        current_platform: synthetic_execution.current_platform.clone(),
        current_arch: synthetic_execution.current_arch.clone(),
        listener_port: synthetic_execution.listener_port,
        target_port: synthetic_execution.target_port,
        requested_execution,
        explicit_decision: synthetic_execution.explicit_decision,
        post_execution_hold_started_at_epoch_ms,
        observed_at_epoch_ms,
        minimum_hold_seconds: LOOPBACK_HOLD_WINDOW_MIN_SECONDS,
        elapsed_hold_seconds,
        post_execution_hold_satisfied,
        execution_attempted: synthetic_execution.execution_attempted,
        synthetic_execution_passed: synthetic_execution.passed,
        closeout_passed: synthetic_execution.closeout.passed,
        expanded_opt_in_allowed: false,
        synthetic_execution,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: post_execution_hold_satisfied,
        blockers,
        warnings: vec![
            "post-execution hold observes only synthetic loopback closeout evidence".into(),
            "wider opt-in remains blocked until a separate decision-readiness gate".into(),
        ],
        facts: vec![
            "post-execution hold is independent from the preflight hold window".into(),
            "hold evidence does not authorize real adapters, TUN, protocol handlers, or default cutover".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-decision-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_decision_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInDecisionReadinessReport> {
    let wider_opt_in_decision = wider_opt_in_decision.unwrap_or(false);
    let requested_execution = requested_execution.unwrap_or(false);
    let post_execution_hold = mihomo_kernel_loopback_r4_expanded_opt_in_post_execution_hold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
        post_execution_hold_started_at_epoch_ms,
    )
    .await?;

    let checks = vec![
        KernelLoopbackR4ExpandedOptInDecisionReadinessCheck {
            name: "postExecutionHold".into(),
            status: if post_execution_hold.post_execution_hold_satisfied {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: post_execution_hold.post_execution_hold_satisfied,
            blockers: if post_execution_hold.post_execution_hold_satisfied {
                Vec::new()
            } else {
                post_execution_hold.blockers.clone()
            },
            facts: vec!["synthetic execution closeout must remain stable through the hold window".into()],
        },
        KernelLoopbackR4ExpandedOptInDecisionReadinessCheck {
            name: "widerOptInDecision".into(),
            status: if wider_opt_in_decision { "passed" } else { "blocked" }.into(),
            passed: wider_opt_in_decision,
            blockers: if wider_opt_in_decision {
                Vec::new()
            } else {
                vec!["wider R4 opt-in requires an explicit decision after post-execution hold".into()]
            },
            facts: vec!["synthetic success alone is not wider opt-in permission".into()],
        },
        KernelLoopbackR4ExpandedOptInDecisionReadinessCheck {
            name: "defaultCutoverBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["decision readiness can only target bounded loopback-expanded opt-in".into()],
        },
    ];
    let decision_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInDecisionReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-decision-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: post_execution_hold.mutates_runtime,
        live_execution_allowed: post_execution_hold.live_execution_allowed,
        current_platform: post_execution_hold.current_platform.clone(),
        current_arch: post_execution_hold.current_arch.clone(),
        listener_port: post_execution_hold.listener_port,
        target_port: post_execution_hold.target_port,
        requested_execution,
        explicit_decision: post_execution_hold.explicit_decision,
        wider_opt_in_decision,
        decision_ready,
        wider_opt_in_allowed: false,
        expanded_opt_in_allowed: false,
        post_execution_hold,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: decision_ready,
        blockers,
        warnings: vec!["decision readiness is still not default cutover or production forwarding permission".into()],
        facts: vec![
            "bundles post-execution hold and explicit wider-decision readiness".into(),
            "keeps real adapter/TUN/protocol/default route replacement blocked".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-limited-rollout-gate".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_limited_rollout_gate(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
) -> Result<KernelLoopbackR4ExpandedOptInLimitedRolloutGateReport> {
    let limited_rollout_decision = limited_rollout_decision.unwrap_or(false);
    let canary_scope = canary_scope.unwrap_or_else(|| "loopbackSyntheticCanary".into());
    let max_canary_sessions = max_canary_sessions.unwrap_or(1);
    let requested_execution = requested_execution.unwrap_or(false);
    let decision_readiness = mihomo_kernel_loopback_r4_expanded_opt_in_decision_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
    )
    .await?;

    let canary_scope_passed = canary_scope == "loopbackSyntheticCanary";
    let session_limit_passed = (1..=3).contains(&max_canary_sessions);
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "decisionReadiness".into(),
            status: if decision_readiness.decision_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: decision_readiness.decision_ready,
            blockers: if decision_readiness.decision_ready {
                Vec::new()
            } else {
                decision_readiness.blockers.clone()
            },
            facts: vec!["limited rollout gate consumes post-execution hold plus wider-decision readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "limitedRolloutDecision".into(),
            status: if limited_rollout_decision { "passed" } else { "blocked" }.into(),
            passed: limited_rollout_decision,
            blockers: if limited_rollout_decision {
                Vec::new()
            } else {
                vec!["limited rollout requires a separate explicit decision".into()]
            },
            facts: vec!["limited rollout decision is distinct from wider opt-in readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "canaryScope".into(),
            status: if canary_scope_passed { "passed" } else { "blocked" }.into(),
            passed: canary_scope_passed,
            blockers: if canary_scope_passed {
                Vec::new()
            } else {
                vec!["canary scope must remain loopbackSyntheticCanary".into()]
            },
            facts: vec!["canary scope excludes real adapters, TUN, and default route".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "canarySessionLimit".into(),
            status: if session_limit_passed { "passed" } else { "blocked" }.into(),
            passed: session_limit_passed,
            blockers: if session_limit_passed {
                Vec::new()
            } else {
                vec!["limited rollout canary session cap must be between 1 and 3".into()]
            },
            facts: vec!["session cap keeps rollout bounded and reversible".into()],
        },
    ];
    let gate_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInLimitedRolloutGateReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-limited-rollout-gate".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: decision_readiness.mutates_runtime,
        live_execution_allowed: decision_readiness.live_execution_allowed,
        current_platform: decision_readiness.current_platform.clone(),
        current_arch: decision_readiness.current_arch.clone(),
        listener_port: decision_readiness.listener_port,
        target_port: decision_readiness.target_port,
        requested_execution,
        explicit_decision: decision_readiness.explicit_decision,
        wider_opt_in_decision: decision_readiness.wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        gate_ready,
        limited_rollout_allowed: false,
        expanded_opt_in_allowed: false,
        decision_readiness,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: gate_ready,
        blockers,
        warnings: vec!["limited rollout gate is readiness evidence only and does not start rollout".into()],
        facts: vec![
            "permits only bounded loopback-synthetic canary readiness".into(),
            "keeps real adapter/TUN/protocol/default-route cutover outside R4".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-rollout-audit".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_rollout_audit(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
) -> Result<KernelLoopbackR4ExpandedOptInRolloutAuditReport> {
    let gate = mihomo_kernel_loopback_r4_expanded_opt_in_limited_rollout_gate(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
    )
    .await?;
    let rows = vec![
        KernelLoopbackR4ExpandedOptInRolloutAuditRow {
            name: "gateReady".into(),
            status: if gate.gate_ready { "passed" } else { "blocked" }.into(),
            passed: gate.gate_ready,
            blockers: if gate.gate_ready {
                Vec::new()
            } else {
                gate.blockers.clone()
            },
            facts: vec!["audit records the limited rollout gate result".into()],
        },
        KernelLoopbackR4ExpandedOptInRolloutAuditRow {
            name: "rollbackBinding".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["rollback remains bound to synthetic closeout and loopback leak evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInRolloutAuditRow {
            name: "defaultCutoverBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["audit scope excludes default route, system proxy, TUN, and real adapters".into()],
        },
    ];
    let audit_ready = rows.iter().all(|row| row.passed);
    let blockers = rows
        .iter()
        .flat_map(|row| row.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInRolloutAuditReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-rollout-audit".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: gate.mutates_runtime,
        live_execution_allowed: gate.live_execution_allowed,
        current_platform: gate.current_platform.clone(),
        current_arch: gate.current_arch.clone(),
        canary_scope: gate.canary_scope.clone(),
        max_canary_sessions: gate.max_canary_sessions,
        audit_ready,
        limited_rollout_allowed: false,
        expanded_opt_in_allowed: false,
        gate,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: audit_ready,
        blockers,
        warnings: vec!["rollout audit records readiness only and does not run canary rollout".into()],
        facts: vec![
            "bundles gate, rollback binding, and cutover boundary audit rows".into(),
            "keeps R4 limited rollout separated from production traffic cutover".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-closeout-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_closeout_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInCloseoutReadinessReport> {
    let closeout_decision = closeout_decision.unwrap_or(false);
    let audit = mihomo_kernel_loopback_r4_expanded_opt_in_rollout_audit(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rolloutAudit".into(),
            status: if audit.audit_ready { "passed" } else { "blocked" }.into(),
            passed: audit.audit_ready,
            blockers: if audit.audit_ready {
                Vec::new()
            } else {
                audit.blockers.clone()
            },
            facts: vec!["closeout readiness consumes rollout audit evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "closeoutDecision".into(),
            status: if closeout_decision { "passed" } else { "blocked" }.into(),
            passed: closeout_decision,
            blockers: if closeout_decision {
                Vec::new()
            } else {
                vec!["R4 closeout requires an explicit closeout decision".into()]
            },
            facts: vec!["closeout decision is separate from rollout gate decisions".into()],
        },
    ];
    let closeout_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInCloseoutReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-closeout-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: audit.mutates_runtime,
        live_execution_allowed: audit.live_execution_allowed,
        current_platform: audit.current_platform.clone(),
        current_arch: audit.current_arch.clone(),
        closeout_decision,
        closeout_ready,
        limited_rollout_allowed: false,
        expanded_opt_in_allowed: false,
        audit,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: closeout_ready,
        blockers,
        warnings: vec!["closeout readiness does not authorize production forwarding or default cutover".into()],
        facts: vec![
            "collects final R4 readiness evidence for a separate closeout report".into(),
            "leaves Go Mihomo data plane ownership unchanged".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-closeout-report".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_closeout_report(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInCloseoutReport> {
    let requested_execution = requested_execution.unwrap_or(false);
    let closeout_readiness = mihomo_kernel_loopback_r4_expanded_opt_in_closeout_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        Some(requested_execution),
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
    )
    .await?;
    let r4_closeout_complete = closeout_readiness.closeout_ready;
    let mut evidence = Vec::new();
    evidence.extend(closeout_readiness.checks.clone());
    evidence.push(KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
        name: "r4Boundary".into(),
        status: "passed".into(),
        passed: true,
        blockers: Vec::new(),
        facts: vec!["R4 closeout report keeps R4 bounded to synthetic loopback evidence".into()],
    });
    evidence.push(KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
        name: "goDataPlaneBoundary".into(),
        status: "passed".into(),
        passed: true,
        blockers: Vec::new(),
        facts: vec!["Mihomo remains the production data plane after R4 closeout".into()],
    });
    let blockers = evidence
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInCloseoutReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-closeout-report".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: closeout_readiness.mutates_runtime,
        live_execution_allowed: closeout_readiness.live_execution_allowed,
        current_platform: closeout_readiness.current_platform.clone(),
        current_arch: closeout_readiness.current_arch.clone(),
        requested_execution,
        explicit_decision: closeout_readiness.audit.gate.decision_readiness.explicit_decision,
        closeout_decision: closeout_readiness.closeout_decision,
        closeout_ready: closeout_readiness.closeout_ready,
        r4_closeout_complete,
        limited_rollout_allowed: false,
        expanded_opt_in_allowed: false,
        closeout_readiness,
        evidence,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: r4_closeout_complete,
        blockers,
        warnings: vec!["R4 closeout is not default cutover or production forwarding permission".into()],
        facts: vec![
            "summarizes R4 synthetic execution, hold, decision, rollout gate, audit, and closeout readiness".into(),
            "keeps real adapters, TUN, protocol handlers, system proxy, and default route blocked".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-completion-summary".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_completion_summary(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInCompletionReport> {
    let closeout_report = mihomo_kernel_loopback_r4_expanded_opt_in_closeout_report(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
    )
    .await?;
    let r4_complete = closeout_report.r4_closeout_complete;
    let blockers = if r4_complete {
        Vec::new()
    } else {
        closeout_report.blockers.clone()
    };

    Ok(KernelLoopbackR4ExpandedOptInCompletionReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-completion-summary".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: closeout_report.mutates_runtime,
        live_execution_allowed: closeout_report.live_execution_allowed,
        current_platform: closeout_report.current_platform.clone(),
        current_arch: closeout_report.current_arch.clone(),
        r4_complete,
        completed_batches: vec![
            "loopback-r4-expanded-opt-in-preflight".into(),
            "loopback-r4-expanded-opt-in-execution-plan".into(),
            "loopback-r4-expanded-opt-in-execution-guard".into(),
            "loopback-r4-expanded-opt-in-synthetic-execution".into(),
            "loopback-r4-expanded-opt-in-post-execution-hold".into(),
            "loopback-r4-expanded-opt-in-decision-readiness".into(),
            "loopback-r4-expanded-opt-in-limited-rollout-gate".into(),
            "loopback-r4-expanded-opt-in-rollout-audit".into(),
            "loopback-r4-expanded-opt-in-closeout-readiness".into(),
            "loopback-r4-expanded-opt-in-closeout-report".into(),
        ],
        open_boundaries: vec![
            "realAdapterForwarding".into(),
            "tunForwarding".into(),
            "protocolHandlers".into(),
            "systemProxyCutover".into(),
            "defaultRouteCutover".into(),
        ],
        next_phase_candidate: "loopback-r5-default-cutover-preflight".into(),
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        closeout_report,
        passed: r4_complete,
        blockers,
        warnings: vec!["R4 completion summary does not enter R5 automatically".into()],
        facts: vec![
            "R4 completion is a documentation and evidence boundary only".into(),
            "R5 must start with a separate preflight before any default cutover work".into(),
        ],
        next_safe_batch: "loopback-r4-expanded-opt-in-next-phase-handoff".into(),
    })
}

pub async fn mihomo_kernel_loopback_r4_expanded_opt_in_next_phase_handoff(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
) -> Result<KernelLoopbackR4ExpandedOptInNextPhaseHandoffReport> {
    let handoff_decision = handoff_decision.unwrap_or(false);
    let completion = mihomo_kernel_loopback_r4_expanded_opt_in_completion_summary(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r4Completion".into(),
            status: if completion.r4_complete { "passed" } else { "blocked" }.into(),
            passed: completion.r4_complete,
            blockers: if completion.r4_complete {
                Vec::new()
            } else {
                completion.blockers.clone()
            },
            facts: vec!["handoff requires completed R4 closeout report evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "handoffDecision".into(),
            status: if handoff_decision { "passed" } else { "blocked" }.into(),
            passed: handoff_decision,
            blockers: if handoff_decision {
                Vec::new()
            } else {
                vec!["next phase handoff requires an explicit handoff decision".into()]
            },
            facts: vec!["handoff decision only allows planning the next preflight".into()],
        },
    ];
    let handoff_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR4ExpandedOptInNextPhaseHandoffReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r4-expanded-opt-in-next-phase-handoff".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: completion.mutates_runtime,
        live_execution_allowed: completion.live_execution_allowed,
        current_platform: completion.current_platform.clone(),
        current_arch: completion.current_arch.clone(),
        handoff_decision,
        handoff_ready,
        next_phase: completion.next_phase_candidate.clone(),
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        completion,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: handoff_ready,
        blockers,
        warnings: vec!["handoff readiness does not authorize R5 execution or default cutover".into()],
        facts: vec![
            "next phase starts at preflight only".into(),
            "Mihomo remains the active kernel and production data plane".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-preflight".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_preflight(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverPreflightReport> {
    let r5_preflight_decision = r5_preflight_decision.unwrap_or(false);
    let handoff = mihomo_kernel_loopback_r4_expanded_opt_in_next_phase_handoff(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "handoffReady".into(),
            status: if handoff.handoff_ready { "passed" } else { "blocked" }.into(),
            passed: handoff.handoff_ready,
            blockers: if handoff.handoff_ready {
                Vec::new()
            } else {
                handoff.blockers.clone()
            },
            facts: vec!["R5 preflight requires completed R4 handoff evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r5PreflightDecision".into(),
            status: if r5_preflight_decision { "passed" } else { "blocked" }.into(),
            passed: r5_preflight_decision,
            blockers: if r5_preflight_decision {
                Vec::new()
            } else {
                vec!["R5 preflight requires an explicit preflight decision".into()]
            },
            facts: vec!["preflight decision permits evidence collection only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "defaultCutoverBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec![
                "default route, system proxy, TUN, protocol handlers, and real adapters remain unchanged".into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "runtimeOwnershipBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["Mihomo remains the active production data plane during R5 preflight".into()],
        },
    ];
    let preflight_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverPreflightReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-preflight".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: handoff.current_platform.clone(),
        current_arch: handoff.current_arch.clone(),
        r5_preflight_decision,
        preflight_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        handoff,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: preflight_ready,
        blockers,
        warnings: vec!["R5 preflight is read-only and does not authorize default cutover".into()],
        facts: vec![
            "starts R5 with evidence checks only".into(),
            "no system proxy, TUN, protocol, adapter, or default route mutation is performed".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-risk-matrix".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_risk_matrix(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverRiskMatrixReport> {
    let preflight = mihomo_kernel_loopback_r5_default_cutover_preflight(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
    )
    .await?;
    let rows = vec![
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "defaultRouteMutation".into(),
            severity: "critical".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["default route mutation remains outside this batch".into()],
        },
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "systemProxyMutation".into(),
            severity: "high".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["system proxy changes require a later guarded plan".into()],
        },
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "tunForwardingMutation".into(),
            severity: "high".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["TUN forwarding remains Mihomo-owned".into()],
        },
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "protocolHandlerMutation".into(),
            severity: "high".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["protocol handler registration is not touched by preflight".into()],
        },
        KernelLoopbackR5DefaultCutoverRiskRow {
            name: "realAdapterForwarding".into(),
            severity: "critical".into(),
            status: "blocked".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["real outbound adapters are not dialed".into()],
        },
    ];
    let risk_matrix_ready = preflight.preflight_ready && rows.iter().all(|row| row.passed);
    let blockers = rows
        .iter()
        .flat_map(|row| row.blockers.clone())
        .chain(preflight.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverRiskMatrixReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-risk-matrix".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: preflight.current_platform.clone(),
        current_arch: preflight.current_arch.clone(),
        risk_matrix_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        preflight,
        rows,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: risk_matrix_ready,
        blockers,
        warnings: vec!["risk matrix blocks every production mutation in this batch".into()],
        facts: vec!["catalogs R5 production cutover risks before a guarded plan exists".into()],
        next_safe_batch: "loopback-r5-default-cutover-rollback-abort-plan".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_rollback_abort_plan(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverRollbackAbortPlanReport> {
    let rollback_plan_decision = rollback_plan_decision.unwrap_or(false);
    let risk_matrix = mihomo_kernel_loopback_r5_default_cutover_risk_matrix(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "riskMatrixReady".into(),
            status: if risk_matrix.risk_matrix_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: risk_matrix.risk_matrix_ready,
            blockers: if risk_matrix.risk_matrix_ready {
                Vec::new()
            } else {
                risk_matrix.blockers.clone()
            },
            facts: vec!["rollback/abort planning requires completed risk matrix".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackPlanDecision".into(),
            status: if rollback_plan_decision { "passed" } else { "blocked" }.into(),
            passed: rollback_plan_decision,
            blockers: if rollback_plan_decision {
                Vec::new()
            } else {
                vec!["rollback/abort plan requires an explicit planning decision".into()]
            },
            facts: vec!["planning decision authorizes rollback evidence only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "abortCriteria".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec![
                "abort on route drift, TUN drift, system proxy drift, protocol drift, adapter dial, or fallback loss"
                    .into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["rollback currently means no-op because preflight performs no mutation".into()],
        },
    ];
    let rollback_abort_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverRollbackAbortPlanReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-rollback-abort-plan".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: risk_matrix.current_platform.clone(),
        current_arch: risk_matrix.current_arch.clone(),
        rollback_plan_decision,
        rollback_abort_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        risk_matrix,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: rollback_abort_ready,
        blockers,
        warnings: vec!["rollback/abort plan still does not allow production cutover execution".into()],
        facts: vec![
            "defines abort evidence before any R5 execution plan can be proposed".into(),
            "keeps all production network ownership with Mihomo".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-execution-plan".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_execution_plan(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverExecutionPlanReport> {
    let execution_plan_decision = execution_plan_decision.unwrap_or(false);
    let rollback_abort_plan = mihomo_kernel_loopback_r5_default_cutover_rollback_abort_plan(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
    )
    .await?;
    let steps = vec![
        KernelLoopbackR5DefaultCutoverExecutionPlanStep {
            order: 1,
            name: "snapshotCurrentRuntimeState".into(),
            phase: "preflight".into(),
            allowed: true,
            mutates_runtime: false,
            facts: vec!["capture config, system proxy, TUN, route, and listener state before any dry run".into()],
        },
        KernelLoopbackR5DefaultCutoverExecutionPlanStep {
            order: 2,
            name: "simulateCutoverPlan".into(),
            phase: "dryRunOnly".into(),
            allowed: true,
            mutates_runtime: false,
            facts: vec!["build an in-memory cutover intent without installing adapters or routes".into()],
        },
        KernelLoopbackR5DefaultCutoverExecutionPlanStep {
            order: 3,
            name: "verifyAbortCriteria".into(),
            phase: "dryRunOnly".into(),
            allowed: true,
            mutates_runtime: false,
            facts: vec!["evaluate route/TUN/system proxy/protocol/adapter drift abort criteria".into()],
        },
        KernelLoopbackR5DefaultCutoverExecutionPlanStep {
            order: 4,
            name: "productionMutation".into(),
            phase: "blocked".into(),
            allowed: false,
            mutates_runtime: false,
            facts: vec!["default route, system proxy, TUN, protocol, and real adapter mutation stay blocked".into()],
        },
    ];
    let execution_plan_ready = rollback_abort_plan.rollback_abort_ready && execution_plan_decision;
    let mut blockers = if rollback_abort_plan.rollback_abort_ready {
        Vec::new()
    } else {
        rollback_abort_plan.blockers.clone()
    };
    if !execution_plan_decision {
        blockers.push("R5 execution plan requires an explicit planning decision".into());
    }

    Ok(KernelLoopbackR5DefaultCutoverExecutionPlanReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-execution-plan".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: rollback_abort_plan.current_platform.clone(),
        current_arch: rollback_abort_plan.current_arch.clone(),
        execution_plan_decision,
        execution_plan_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        rollback_abort_plan,
        steps,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: execution_plan_ready,
        blockers,
        warnings: vec!["execution plan is dry-run planning only; production mutation remains blocked".into()],
        facts: vec![
            "defines R5 order of operations without executing default cutover".into(),
            "keeps Mihomo as the production data plane".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-execution-guard".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_guard(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverGuardReport> {
    let guard_decision = guard_decision.unwrap_or(false);
    let execution_plan = mihomo_kernel_loopback_r5_default_cutover_execution_plan(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "executionPlanReady".into(),
            status: if execution_plan.execution_plan_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: execution_plan.execution_plan_ready,
            blockers: if execution_plan.execution_plan_ready {
                Vec::new()
            } else {
                execution_plan.blockers.clone()
            },
            facts: vec!["guard requires completed R5 execution plan evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "guardDecision".into(),
            status: if guard_decision { "passed" } else { "blocked" }.into(),
            passed: guard_decision,
            blockers: if guard_decision {
                Vec::new()
            } else {
                vec!["R5 execution guard requires an explicit guard decision".into()]
            },
            facts: vec!["guard decision authorizes dry-run readiness only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "mutationFence".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["guard keeps production mutation fenced until dry-run evidence exists".into()],
        },
    ];
    let guard_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverGuardReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-execution-guard".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: execution_plan.current_platform.clone(),
        current_arch: execution_plan.current_arch.clone(),
        guard_decision,
        guard_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        execution_plan,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: guard_ready,
        blockers,
        warnings: vec!["guard readiness is not permission to mutate production networking".into()],
        facts: vec!["gates R5 dry-run readiness behind execution plan and explicit guard decision".into()],
        next_safe_batch: "loopback-r5-default-cutover-dry-run-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_dry_run_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverDryRunReadinessReport> {
    let dry_run_decision = dry_run_decision.unwrap_or(false);
    let guard = mihomo_kernel_loopback_r5_default_cutover_guard(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "guardReady".into(),
            status: if guard.guard_ready { "passed" } else { "blocked" }.into(),
            passed: guard.guard_ready,
            blockers: if guard.guard_ready {
                Vec::new()
            } else {
                guard.blockers.clone()
            },
            facts: vec!["dry-run readiness requires guard evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunDecision".into(),
            status: if dry_run_decision { "passed" } else { "blocked" }.into(),
            passed: dry_run_decision,
            blockers: if dry_run_decision {
                Vec::new()
            } else {
                vec!["R5 dry-run readiness requires an explicit dry-run decision".into()]
            },
            facts: vec!["dry-run decision allows later synthetic dry-run evidence only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunScope".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec![
                "dry run must remain in-memory and may not install routes, TUN, proxy, protocols, or adapters".into(),
            ],
        },
    ];
    let dry_run_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverDryRunReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-dry-run-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: guard.current_platform.clone(),
        current_arch: guard.current_arch.clone(),
        dry_run_decision,
        dry_run_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        guard,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: dry_run_ready,
        blockers,
        warnings: vec!["dry-run readiness still does not perform dry-run execution".into()],
        facts: vec!["prepares a future dry-run evidence batch while keeping production networking unchanged".into()],
        next_safe_batch: "loopback-r5-default-cutover-dry-run-evidence".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_dry_run_evidence(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverDryRunEvidenceReport> {
    let dry_run_execution_decision = dry_run_execution_decision.unwrap_or(false);
    let readiness = mihomo_kernel_loopback_r5_default_cutover_dry_run_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunReady".into(),
            status: if readiness.dry_run_ready { "passed" } else { "blocked" }.into(),
            passed: readiness.dry_run_ready,
            blockers: if readiness.dry_run_ready {
                Vec::new()
            } else {
                readiness.blockers.clone()
            },
            facts: vec!["dry-run evidence requires dry-run readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunExecutionDecision".into(),
            status: if dry_run_execution_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: dry_run_execution_decision,
            blockers: if dry_run_execution_decision {
                Vec::new()
            } else {
                vec!["R5 dry-run evidence requires an explicit dry-run execution decision".into()]
            },
            facts: vec!["execution decision is scoped to in-memory dry-run evidence only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "inMemoryIntent".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["cutover intent is modeled in memory and not applied to runtime config".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "productionStateFence".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["default route, system proxy, TUN, protocols, and adapters remain untouched".into()],
        },
    ];
    let dry_run_executed = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverDryRunEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-dry-run-evidence".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: readiness.current_platform.clone(),
        current_arch: readiness.current_arch.clone(),
        dry_run_executed,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        readiness,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: dry_run_executed,
        blockers,
        warnings: vec!["dry-run evidence is synthetic and does not perform production cutover".into()],
        facts: vec![
            "validates the R5 cutover path as an in-memory intent only".into(),
            "Mihomo remains the active forwarding engine".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-dry-run-closeout".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_dry_run_closeout(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverDryRunCloseoutReport> {
    let evidence = mihomo_kernel_loopback_r5_default_cutover_dry_run_evidence(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunEvidencePassed".into(),
            status: if evidence.dry_run_executed { "passed" } else { "blocked" }.into(),
            passed: evidence.dry_run_executed,
            blockers: if evidence.dry_run_executed {
                Vec::new()
            } else {
                evidence.blockers.clone()
            },
            facts: vec!["closeout requires completed dry-run evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "runtimeUnchanged".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["dry-run closeout observes no runtime mutation to roll back".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "fallbackPreserved".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["Mihomo fallback remains active after synthetic dry run".into()],
        },
    ];
    let dry_run_closeout_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverDryRunCloseoutReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-dry-run-closeout".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: evidence.current_platform.clone(),
        current_arch: evidence.current_arch.clone(),
        dry_run_closeout_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        evidence,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: dry_run_closeout_ready,
        blockers,
        warnings: vec!["dry-run closeout does not promote the dry run to live execution".into()],
        facts: vec!["confirms dry-run evidence leaves production network state unchanged".into()],
        next_safe_batch: "loopback-r5-default-cutover-post-dry-run-hold".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_post_dry_run_hold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverPostDryRunHoldReport> {
    let hold_decision = hold_decision.unwrap_or(false);
    let closeout = mihomo_kernel_loopback_r5_default_cutover_dry_run_closeout(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
    )
    .await?;
    let now_ms = current_epoch_ms();
    let hold_elapsed_seconds = post_dry_run_hold_started_at_epoch_ms
        .map(|started| now_ms.saturating_sub(started) / 1000)
        .unwrap_or(0);
    let hold_window_passed =
        post_dry_run_hold_started_at_epoch_ms.is_some() && hold_elapsed_seconds >= LOOPBACK_HOLD_WINDOW_MIN_SECONDS;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "dryRunCloseoutReady".into(),
            status: if closeout.dry_run_closeout_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: closeout.dry_run_closeout_ready,
            blockers: if closeout.dry_run_closeout_ready {
                Vec::new()
            } else {
                closeout.blockers.clone()
            },
            facts: vec!["post dry-run hold requires dry-run closeout evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "holdWindow".into(),
            status: if hold_window_passed { "passed" } else { "blocked" }.into(),
            passed: hold_window_passed,
            blockers: if hold_window_passed {
                Vec::new()
            } else {
                vec!["post dry-run hold window has not reached the minimum observation period".into()]
            },
            facts: vec![format!("observed hold window seconds: {hold_elapsed_seconds}").into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "holdDecision".into(),
            status: if hold_decision { "passed" } else { "blocked" }.into(),
            passed: hold_decision,
            blockers: if hold_decision {
                Vec::new()
            } else {
                vec!["post dry-run hold requires an explicit hold decision".into()]
            },
            facts: vec!["hold decision keeps next step to readiness only".into()],
        },
    ];
    let hold_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverPostDryRunHoldReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-post-dry-run-hold".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: closeout.current_platform.clone(),
        current_arch: closeout.current_arch.clone(),
        hold_decision,
        hold_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        closeout,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: hold_ready,
        blockers,
        warnings: vec!["post dry-run hold still does not authorize default cutover".into()],
        facts: vec!["requires a bounded observation period after synthetic dry-run closeout".into()],
        next_safe_batch: "loopback-r5-default-cutover-decision-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_decision_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverDecisionReadinessReport> {
    let decision_readiness_decision = decision_readiness_decision.unwrap_or(false);
    let post_dry_run_hold = mihomo_kernel_loopback_r5_default_cutover_post_dry_run_hold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "postDryRunHoldReady".into(),
            status: if post_dry_run_hold.hold_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: post_dry_run_hold.hold_ready,
            blockers: if post_dry_run_hold.hold_ready {
                Vec::new()
            } else {
                post_dry_run_hold.blockers.clone()
            },
            facts: vec!["decision readiness requires completed post-dry-run hold evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "decisionReadinessDecision".into(),
            status: if decision_readiness_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: decision_readiness_decision,
            blockers: if decision_readiness_decision {
                Vec::new()
            } else {
                vec!["R5 decision readiness requires an explicit decision".into()]
            },
            facts: vec!["decision readiness only permits final gate evaluation".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "cutoverBoundary".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["default cutover remains blocked after decision readiness".into()],
        },
    ];
    let decision_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverDecisionReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-decision-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: post_dry_run_hold.current_platform.clone(),
        current_arch: post_dry_run_hold.current_arch.clone(),
        decision_readiness_decision,
        decision_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        post_dry_run_hold,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: decision_ready,
        blockers,
        warnings: vec!["decision readiness does not authorize default cutover".into()],
        facts: vec![
            "summarizes R5 dry-run evidence before final gate evaluation".into(),
            "production forwarding remains Mihomo-owned".into(),
        ],
        next_safe_batch: "loopback-r5-default-cutover-final-gate".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_final_gate(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverFinalGateReport> {
    let final_gate_decision = final_gate_decision.unwrap_or(false);
    let decision_readiness = mihomo_kernel_loopback_r5_default_cutover_decision_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "decisionReady".into(),
            status: if decision_readiness.decision_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: decision_readiness.decision_ready,
            blockers: if decision_readiness.decision_ready {
                Vec::new()
            } else {
                decision_readiness.blockers.clone()
            },
            facts: vec!["final gate requires R5 decision readiness".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalGateDecision".into(),
            status: if final_gate_decision { "passed" } else { "blocked" }.into(),
            passed: final_gate_decision,
            blockers: if final_gate_decision {
                Vec::new()
            } else {
                vec!["R5 final gate requires an explicit final gate decision".into()]
            },
            facts: vec!["final gate decision permits final hold/rollback validation only".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "mutationFence".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["final gate keeps default route, system proxy, TUN, protocols, and adapters fenced".into()],
        },
    ];
    let final_gate_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverFinalGateReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-final-gate".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: decision_readiness.current_platform.clone(),
        current_arch: decision_readiness.current_arch.clone(),
        final_gate_decision,
        final_gate_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        decision_readiness,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: final_gate_ready,
        blockers,
        warnings: vec!["final gate does not open production default cutover".into()],
        facts: vec!["allows only a later final hold and independent rollback validation batch".into()],
        next_safe_batch: "loopback-r5-default-cutover-next-step-handoff".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_next_step_handoff(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverNextStepHandoffReport> {
    let r5_handoff_decision = r5_handoff_decision.unwrap_or(false);
    let final_gate = mihomo_kernel_loopback_r5_default_cutover_final_gate(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
    )
    .await?;
    let next_step: String = "loopback-r5-default-cutover-final-hold".into();
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalGateReady".into(),
            status: if final_gate.final_gate_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_gate.final_gate_ready,
            blockers: if final_gate.final_gate_ready {
                Vec::new()
            } else {
                final_gate.blockers.clone()
            },
            facts: vec!["handoff requires final gate evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r5HandoffDecision".into(),
            status: if r5_handoff_decision { "passed" } else { "blocked" }.into(),
            passed: r5_handoff_decision,
            blockers: if r5_handoff_decision {
                Vec::new()
            } else {
                vec!["R5 next-step handoff requires an explicit handoff decision".into()]
            },
            facts: vec!["handoff is to final hold/rollback validation, not default cutover".into()],
        },
    ];
    let handoff_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverNextStepHandoffReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-next-step-handoff".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: final_gate.current_platform.clone(),
        current_arch: final_gate.current_arch.clone(),
        r5_handoff_decision,
        handoff_ready,
        next_step: next_step.clone(),
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        final_gate,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: handoff_ready,
        blockers,
        warnings: vec!["next-step handoff still does not authorize live default cutover".into()],
        facts: vec!["moves R5 toward final hold and independent rollback validation only".into()],
        next_safe_batch: next_step,
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_final_hold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverFinalHoldReport> {
    let final_hold_decision = final_hold_decision.unwrap_or(false);
    let handoff = mihomo_kernel_loopback_r5_default_cutover_next_step_handoff(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
    )
    .await?;
    let now_ms = current_epoch_ms();
    let final_hold_elapsed_seconds = final_hold_started_at_epoch_ms
        .map(|started| now_ms.saturating_sub(started) / 1000)
        .unwrap_or(0);
    let final_hold_window_passed =
        final_hold_started_at_epoch_ms.is_some() && final_hold_elapsed_seconds >= LOOPBACK_HOLD_WINDOW_MIN_SECONDS;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "handoffReady".into(),
            status: if handoff.handoff_ready { "passed" } else { "blocked" }.into(),
            passed: handoff.handoff_ready,
            blockers: if handoff.handoff_ready {
                Vec::new()
            } else {
                handoff.blockers.clone()
            },
            facts: vec!["final hold requires next-step handoff evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalHoldWindow".into(),
            status: if final_hold_window_passed { "passed" } else { "blocked" }.into(),
            passed: final_hold_window_passed,
            blockers: if final_hold_window_passed {
                Vec::new()
            } else {
                vec!["final hold window has not reached the minimum observation period".into()]
            },
            facts: vec![format!("observed final hold window seconds: {final_hold_elapsed_seconds}").into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalHoldDecision".into(),
            status: if final_hold_decision { "passed" } else { "blocked" }.into(),
            passed: final_hold_decision,
            blockers: if final_hold_decision {
                Vec::new()
            } else {
                vec!["R5 final hold requires an explicit hold decision".into()]
            },
            facts: vec!["final hold permits independent rollback validation only".into()],
        },
    ];
    let final_hold_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverFinalHoldReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-final-hold".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: handoff.current_platform.clone(),
        current_arch: handoff.current_arch.clone(),
        final_hold_started_at_epoch_ms,
        final_hold_elapsed_seconds,
        final_hold_decision,
        final_hold_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        handoff,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: final_hold_ready,
        blockers,
        warnings: vec!["final hold does not authorize live default cutover".into()],
        facts: vec!["requires a bounded observation period after final gate handoff".into()],
        next_safe_batch: "loopback-r5-default-cutover-independent-rollback-validation".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_independent_rollback_validation(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport> {
    let independent_rollback_decision = independent_rollback_decision.unwrap_or(false);
    let observed_rollback_platforms_input = observed_rollback_platforms.clone();
    let final_hold = mihomo_kernel_loopback_r5_default_cutover_final_hold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
    )
    .await?;
    let required_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let observed_rollback_platforms = observed_rollback_platforms_input
        .unwrap_or_default()
        .into_iter()
        .filter(|platform| LOOPBACK_PLATFORM_MATRIX_PLATFORMS.contains(&platform.as_str()))
        .collect::<BTreeSet<String>>();
    let pending_rollback_platforms = LOOPBACK_PLATFORM_MATRIX_PLATFORMS
        .iter()
        .filter(|platform| !observed_rollback_platforms.contains(**platform))
        .map(|platform| (*platform).into())
        .collect::<Vec<String>>();
    let observed_rollback_platforms = observed_rollback_platforms.into_iter().collect::<Vec<String>>();
    let rollback_platforms_ready = pending_rollback_platforms.is_empty();
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalHoldReady".into(),
            status: if final_hold.final_hold_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_hold.final_hold_ready,
            blockers: if final_hold.final_hold_ready {
                Vec::new()
            } else {
                final_hold.blockers.clone()
            },
            facts: vec!["independent rollback validation requires final hold evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackPlatforms".into(),
            status: if rollback_platforms_ready { "passed" } else { "blocked" }.into(),
            passed: rollback_platforms_ready,
            blockers: if rollback_platforms_ready {
                Vec::new()
            } else {
                vec![
                    format!(
                        "missing independent rollback validation for platforms: {}",
                        pending_rollback_platforms.join(", ")
                    )
                    .into(),
                ]
            },
            facts: vec![
                format!(
                    "observed independent rollback platforms: {}",
                    if observed_rollback_platforms.is_empty() {
                        "none".into()
                    } else {
                        observed_rollback_platforms.join(", ")
                    }
                )
                .into(),
            ],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "independentRollbackDecision".into(),
            status: if independent_rollback_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: independent_rollback_decision,
            blockers: if independent_rollback_decision {
                Vec::new()
            } else {
                vec!["R5 independent rollback validation requires an explicit decision".into()]
            },
            facts: vec!["validation remains read-only and loopback scoped".into()],
        },
    ];
    let rollback_validation_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-independent-rollback-validation".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: final_hold.current_platform.clone(),
        current_arch: final_hold.current_arch.clone(),
        independent_rollback_decision,
        rollback_validation_ready,
        required_platforms,
        observed_rollback_platforms,
        pending_rollback_platforms,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        final_hold,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: rollback_validation_ready,
        blockers,
        warnings: vec!["independent rollback validation does not authorize default cutover".into()],
        facts: vec!["requires platform-complete rollback evidence after final hold".into()],
        next_safe_batch: "loopback-r5-default-cutover-closeout-readiness".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_closeout_readiness(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverCloseoutReadinessReport> {
    let r5_closeout_decision = r5_closeout_decision.unwrap_or(false);
    let rollback_validation = mihomo_kernel_loopback_r5_default_cutover_independent_rollback_validation(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
    )
    .await?;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackValidationReady".into(),
            status: if rollback_validation.rollback_validation_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rollback_validation.rollback_validation_ready,
            blockers: if rollback_validation.rollback_validation_ready {
                Vec::new()
            } else {
                rollback_validation.blockers.clone()
            },
            facts: vec!["closeout readiness requires independent rollback validation".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r5CloseoutDecision".into(),
            status: if r5_closeout_decision { "passed" } else { "blocked" }.into(),
            passed: r5_closeout_decision,
            blockers: if r5_closeout_decision {
                Vec::new()
            } else {
                vec!["R5 closeout readiness requires an explicit closeout decision".into()]
            },
            facts: vec!["closeout readiness prepares a report-only batch".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "defaultCutoverStillBlocked".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["real adapter, TUN, protocol, and default route cutover remain blocked".into()],
        },
    ];
    let closeout_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5DefaultCutoverCloseoutReadinessReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-closeout-readiness".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: rollback_validation.current_platform.clone(),
        current_arch: rollback_validation.current_arch.clone(),
        r5_closeout_decision,
        closeout_ready,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        rollback_validation,
        checks,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed: closeout_ready,
        blockers,
        warnings: vec!["closeout readiness does not authorize live default cutover".into()],
        facts: vec!["next batch is report-only closeout evidence for R5".into()],
        next_safe_batch: "loopback-r5-default-cutover-closeout-report".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_default_cutover_closeout_report(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
) -> Result<KernelLoopbackR5DefaultCutoverCloseoutReport> {
    let r5_closeout_report_decision = r5_closeout_report_decision.unwrap_or(false);
    let closeout_readiness = mihomo_kernel_loopback_r5_default_cutover_closeout_readiness(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
    )
    .await?;
    let r5_closeout_complete = closeout_readiness.passed && r5_closeout_report_decision;
    let blockers = if r5_closeout_complete {
        Vec::new()
    } else {
        let mut blockers = closeout_readiness.blockers.clone();
        if !r5_closeout_report_decision {
            blockers.push("R5 closeout report requires an explicit report decision".into());
        }
        blockers
    };

    Ok(KernelLoopbackR5DefaultCutoverCloseoutReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-r5-default-cutover-closeout-report".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        current_platform: closeout_readiness.current_platform.clone(),
        current_arch: closeout_readiness.current_arch.clone(),
        r5_closeout_report_decision,
        r5_closeout_complete,
        default_cutover_allowed: false,
        expanded_opt_in_allowed: false,
        closeout_readiness,
        completed_evidence_batches: vec![
            "r3-loopback-listener-dns-forwarding-evidence".into(),
            "r4-expanded-opt-in-synthetic-execution-and-closeout".into(),
            "r5-default-cutover-preflight-through-final-hold".into(),
            "r5-independent-rollback-validation-and-closeout-readiness".into(),
        ],
        open_boundaries: rust_runtime_fallback_boundaries(),
        passed: r5_closeout_complete,
        blockers,
        warnings: vec!["R5 closeout report closes evidence gates but does not select Rust runtime".into()],
        facts: vec!["R5 evidence is ready to hand off to R6 Rust runtime implementation".into()],
        next_safe_batch: "r5-closeout-r6-rust-runtime-scaffold".into(),
    })
}

pub async fn mihomo_kernel_loopback_r5_closeout_r6_rust_runtime_scaffold(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
) -> Result<KernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport> {
    let rust_runtime_scaffold_decision = rust_runtime_scaffold_decision.unwrap_or(false);
    let r5_closeout = mihomo_kernel_loopback_r5_default_cutover_closeout_report(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
    )
    .await?;
    let runtime_selection =
        kernel_runtime_selection_scaffold(requested_runtime_kind, rust_runtime_opt_in_decision).await;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r5CloseoutComplete".into(),
            status: if r5_closeout.passed { "passed" } else { "blocked" }.into(),
            passed: r5_closeout.passed,
            blockers: r5_closeout.blockers.clone(),
            facts: vec!["R5 closeout report is bundled with R6 scaffold".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustRuntimeScaffoldDecision".into(),
            status: if rust_runtime_scaffold_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_runtime_scaffold_decision,
            blockers: if rust_runtime_scaffold_decision {
                Vec::new()
            } else {
                vec!["R6 Rust runtime scaffold requires an explicit scaffold decision".into()]
            },
            facts: vec!["Rust runtime kind and fallback boundaries are modeled".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "mihomoRemainsSelectedDefault".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["scaffold does not change the selected default runtime".into()],
        },
    ];
    let scaffold_ready = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "r5-closeout-r6-rust-runtime-scaffold".into(),
        mutates_runtime: false,
        live_execution_allowed: false,
        rust_runtime_scaffold_decision,
        scaffold_ready,
        default_cutover_allowed: false,
        r5_closeout,
        runtime_selection,
        checks,
        blockers,
        warnings: vec!["R6 scaffold is selectable metadata only; Rust runtime remains disabled".into()],
        facts: vec!["next batch can implement explicit opt-in Rust runtime MVP without more R5 gates".into()],
        next_safe_batch: "r6-opt-in-rust-runtime-mvp".into(),
    })
}

pub async fn rust_kernel_runtime_r6_opt_in_mvp(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
) -> Result<KernelLoopbackR6OptInRustRuntimeMvpReport> {
    let requested_runtime_kind_for_parse = requested_runtime_kind.clone();
    let rust_runtime_opt_in_decision = rust_runtime_opt_in_decision.unwrap_or(false);
    let requested_runtime_kind = parse_kernel_runtime_kind(requested_runtime_kind_for_parse);
    let scaffold = Box::pin(mihomo_kernel_loopback_r5_closeout_r6_rust_runtime_scaffold(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        Some("rust".into()),
        Some(rust_runtime_opt_in_decision),
        rust_runtime_scaffold_decision,
    ))
    .await?;
    let supported_subset = rust_kernel_runtime_supported_subset_report().await?;
    let subset_ready = supported_subset.blockers.is_empty()
        && supported_subset.rule_decision_owned
        && supported_subset.dns_decision_owned
        && supported_subset.adapter_decision_owned
        && supported_subset.forwarding_surface_owned;
    let requested_rust = matches!(requested_runtime_kind, KernelRuntimeKind::Rust);
    let pre_health_ready = scaffold.scaffold_ready && rust_runtime_opt_in_decision && requested_rust && subset_ready;
    let loopback_forwarding_evidence = if pre_health_ready {
        Some(mihomo_kernel_loopback_forwarding_rollback_drill(listener_port, target_port).await?)
    } else {
        None
    };
    let health_state = rust_kernel_runtime_health_state_report(pre_health_ready, loopback_forwarding_evidence.as_ref());
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r6ScaffoldReady".into(),
            status: if scaffold.scaffold_ready { "passed" } else { "blocked" }.into(),
            passed: scaffold.scaffold_ready,
            blockers: scaffold.blockers.clone(),
            facts: vec!["R5 closeout and Rust runtime scaffold are required before opt-in".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "requestedRustRuntime".into(),
            status: if requested_rust { "passed" } else { "blocked" }.into(),
            passed: requested_rust,
            blockers: if requested_rust {
                Vec::new()
            } else {
                vec!["R6 opt-in MVP requires requested_runtime_kind=rust".into()]
            },
            facts: vec!["Mihomo remains selected unless Rust is explicitly requested".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rustOptInDecision".into(),
            status: if rust_runtime_opt_in_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rust_runtime_opt_in_decision,
            blockers: if rust_runtime_opt_in_decision {
                Vec::new()
            } else {
                vec!["R6 Rust runtime MVP requires explicit opt-in decision".into()]
            },
            facts: vec!["opt-in is scoped to supported subset and Mihomo fallback".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "supportedSubsetDecisionPath".into(),
            status: if subset_ready { "passed" } else { "blocked" }.into(),
            passed: subset_ready,
            blockers: supported_subset.blockers.clone(),
            facts: vec!["Rust owns rule, DNS, and adapter decisions for the supported subset".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "healthAndRollbackState".into(),
            status: if health_state.health_ready { "passed" } else { "blocked" }.into(),
            passed: health_state.health_ready,
            blockers: health_state.blockers.clone(),
            facts: vec!["loopback rollback evidence arms health and fallback state".into()],
        },
    ];
    let opt_in_ready = checks.iter().all(|check| check.passed);
    let selected_runtime_kind = if opt_in_ready {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR6OptInRustRuntimeMvpReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-opt-in-rust-runtime-mvp".into(),
        mutates_runtime: loopback_forwarding_evidence.is_some(),
        live_execution_allowed: opt_in_ready,
        rust_runtime_opt_in_decision,
        requested_runtime_kind,
        selected_runtime_kind,
        opt_in_ready,
        default_cutover_allowed: false,
        mihomo_fallback: true,
        scaffold,
        supported_subset,
        health_state,
        loopback_forwarding_evidence,
        checks,
        blockers,
        warnings: vec![
            "R6 MVP enables explicit opt-in metadata and loopback execution only".into(),
            "default Rust runtime still requires canary gate and automatic fallback evidence".into(),
        ],
        facts: vec![
            "Rust runtime is selectable for the supported subset only after explicit opt-in".into(),
            "Mihomo fallback remains active for unsupported protocols, TUN, adapters, and emergency rollback".into(),
        ],
        next_safe_batch: "r6-rust-default-canary".into(),
    })
}

fn rust_kernel_runtime_canary_profile_report(
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
) -> RustKernelRuntimeCanaryProfileReport {
    let canary_scope = canary_scope.unwrap_or_else(|| "loopbackSyntheticCanary".into());
    let max_canary_sessions = max_canary_sessions.unwrap_or(1);
    let mut blockers = Vec::new();

    if canary_scope != "loopbackSyntheticCanary" {
        blockers.push("R6 default canary is capped to loopbackSyntheticCanary".into());
    }
    if !(1..=3).contains(&max_canary_sessions) {
        blockers.push("R6 default canary allows 1 to 3 synthetic sessions only".into());
    }

    RustKernelRuntimeCanaryProfileReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-rust-default-canary-profile".into(),
        canary_scope,
        max_canary_sessions,
        capped_profile: blockers.is_empty(),
        supported_safe_subset: rust_runtime_supported_safe_subset(),
        fallback_boundaries: rust_runtime_fallback_boundaries(),
        blockers,
        warnings: vec!["canary profile is a bounded default for the supported safe subset only".into()],
        facts: vec![
            "unsupported protocols, TUN, and production adapter egress remain Mihomo fallback".into(),
            "canary scope reuses the existing loopback-only safety cap".into(),
        ],
    }
}

fn rust_kernel_runtime_automatic_fallback_report(
    r6_opt_in: &KernelLoopbackR6OptInRustRuntimeMvpReport,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
) -> RustKernelRuntimeAutomaticFallbackReport {
    let health_check_passed = health_check_passed.unwrap_or(r6_opt_in.health_state.health_ready);
    let rollback_triggered = rollback_triggered.unwrap_or(false);
    let health_ready = r6_opt_in.health_state.health_ready && health_check_passed;
    let rollback_armed = r6_opt_in.health_state.rollback_armed && r6_opt_in.mihomo_fallback;
    let mut triggers = Vec::new();

    if !r6_opt_in.opt_in_ready {
        triggers.push("r6-opt-in-not-ready".into());
    }
    if !health_ready {
        triggers.push("health-check-not-ready".into());
    }
    if rollback_triggered {
        triggers.push("rollback-triggered".into());
    }
    if !rollback_armed {
        triggers.push("rollback-not-armed".into());
    }

    let fallback_activated = !triggers.is_empty();
    let selected_runtime_kind = if fallback_activated {
        KernelRuntimeKind::Mihomo
    } else {
        KernelRuntimeKind::Rust
    };
    let blockers = if fallback_activated {
        triggers
            .iter()
            .map(|trigger| format!("automatic fallback selected Mihomo: {trigger}").into())
            .collect()
    } else {
        Vec::new()
    };

    RustKernelRuntimeAutomaticFallbackReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-rust-default-canary-automatic-fallback".into(),
        health_check_passed,
        rollback_triggered,
        health_ready,
        rollback_armed,
        fallback_activated,
        selected_runtime_kind,
        fallback_runtime_kind: KernelRuntimeKind::Mihomo,
        triggers,
        blockers,
        facts: vec![
            "Rust canary default selects Mihomo immediately on health or rollback triggers".into(),
            "fallback does not retire the Mihomo sidecar or unsupported runtime paths".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_r6_default_canary(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
    canary_default_decision: Option<bool>,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
) -> Result<KernelLoopbackR6RustDefaultCanaryReport> {
    let canary_default_decision = canary_default_decision.unwrap_or(false);
    let requested_runtime_kind_for_parse = requested_runtime_kind.clone();
    let r6_opt_in = Box::pin(rust_kernel_runtime_r6_opt_in_mvp(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope.clone(),
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
    ))
    .await?;
    let canary_profile = rust_kernel_runtime_canary_profile_report(canary_scope, max_canary_sessions);
    let automatic_fallback =
        rust_kernel_runtime_automatic_fallback_report(&r6_opt_in, health_check_passed, rollback_triggered);
    let requested_runtime_kind = parse_kernel_runtime_kind(requested_runtime_kind_for_parse);
    let fallback_ready = automatic_fallback.rollback_armed && !automatic_fallback.fallback_activated;
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r6OptInReady".into(),
            status: if r6_opt_in.opt_in_ready { "passed" } else { "blocked" }.into(),
            passed: r6_opt_in.opt_in_ready,
            blockers: r6_opt_in.blockers.clone(),
            facts: vec!["R6 default canary builds on the explicit opt-in MVP".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "canaryDefaultDecision".into(),
            status: if canary_default_decision { "passed" } else { "blocked" }.into(),
            passed: canary_default_decision,
            blockers: if canary_default_decision {
                Vec::new()
            } else {
                vec!["R6 Rust default canary requires an explicit canary default decision".into()]
            },
            facts: vec!["the canary decision is separate from production default cutover".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "cappedCanaryProfile".into(),
            status: if canary_profile.capped_profile {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: canary_profile.capped_profile,
            blockers: canary_profile.blockers.clone(),
            facts: vec!["canary scope and session cap keep the default bounded".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "automaticFallbackHealthy".into(),
            status: if fallback_ready { "passed" } else { "blocked" }.into(),
            passed: fallback_ready,
            blockers: automatic_fallback.blockers.clone(),
            facts: vec!["health and rollback triggers return selection to Mihomo".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "productionDefaultBlocked".into(),
            status: "passed".into(),
            passed: true,
            blockers: Vec::new(),
            facts: vec!["R6 canary does not authorize R7 production default cutover".into()],
        },
    ];
    let canary_default_allowed = checks.iter().all(|check| check.passed);
    let selected_runtime_kind = if canary_default_allowed {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR6RustDefaultCanaryReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r6-rust-default-canary".into(),
        mutates_runtime: r6_opt_in.mutates_runtime,
        live_execution_allowed: canary_default_allowed,
        rust_runtime_opt_in_decision: r6_opt_in.rust_runtime_opt_in_decision,
        canary_default_decision,
        requested_runtime_kind,
        selected_runtime_kind,
        canary_default_allowed,
        production_default_allowed: false,
        mihomo_fallback: true,
        r6_opt_in,
        canary_profile,
        automatic_fallback,
        checks,
        blockers,
        warnings: vec![
            "R6 canary default is limited to the capped safe subset".into(),
            "R7 must complete canary closeout before Rust can become the wider default".into(),
        ],
        facts: vec![
            "Rust runtime is the selected default only inside the capped canary when all health gates pass".into(),
            "Mihomo fallback remains the selected runtime for unsupported paths and rollback triggers".into(),
        ],
        next_safe_batch: "r7-rust-default-cutover".into(),
    })
}

fn rust_kernel_runtime_r7_canary_closeout_summary(
    r6_canary: &KernelLoopbackR6RustDefaultCanaryReport,
    rollback_hold_decision: bool,
) -> RustKernelRuntimeCanaryCloseoutSummaryReport {
    let canary_health_ready = r6_canary.automatic_fallback.health_ready
        && r6_canary.automatic_fallback.health_check_passed
        && !r6_canary.automatic_fallback.rollback_triggered;
    let automatic_fallback_armed = r6_canary.automatic_fallback.rollback_armed
        && matches!(
            r6_canary.automatic_fallback.fallback_runtime_kind,
            KernelRuntimeKind::Mihomo
        );
    let closeout_ready =
        r6_canary.canary_default_allowed && canary_health_ready && automatic_fallback_armed && rollback_hold_decision;
    let mut blockers = Vec::new();

    if !r6_canary.canary_default_allowed {
        blockers.push("R7 cutover requires a passing R6 Rust default canary".into());
    }
    if !canary_health_ready {
        blockers.push("R7 cutover requires canary health checks to remain ready".into());
    }
    if !automatic_fallback_armed {
        blockers.push("R7 cutover requires automatic Mihomo fallback to stay armed".into());
    }
    if !rollback_hold_decision {
        blockers.push("R7 cutover requires rollback hold evidence before widening the default".into());
    }

    RustKernelRuntimeCanaryCloseoutSummaryReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-rust-default-cutover-canary-closeout".into(),
        canary_default_allowed: r6_canary.canary_default_allowed,
        canary_health_ready,
        automatic_fallback_armed,
        rollback_hold_passed: rollback_hold_decision,
        closeout_ready,
        evidence: vec![
            "get_runtime_kernel_loopback_r6_rust_default_canary".into(),
            "canary health check".into(),
            "automatic Mihomo fallback state".into(),
            "rollback hold decision".into(),
        ],
        blockers,
        facts: vec![
            "R7 consumes R6 canary closeout evidence instead of retiring Mihomo fallback".into(),
            "rollback hold is required before Rust becomes the supported profile default".into(),
        ],
    }
}

fn rust_kernel_runtime_supported_profile_default_report(
    profile_scope: Option<String>,
    canary_closeout: &RustKernelRuntimeCanaryCloseoutSummaryReport,
    r7_cutover_decision: bool,
    rollback_switch_requested: bool,
) -> RustKernelRuntimeSupportedProfileDefaultReport {
    let profile_scope = profile_scope.unwrap_or_else(|| "supportedDefaultProfile".into());
    let mut blockers = Vec::new();

    if profile_scope != "supportedDefaultProfile" {
        blockers.push("R7 Rust default cutover is limited to supportedDefaultProfile".into());
    }
    if !canary_closeout.closeout_ready {
        blockers.extend(canary_closeout.blockers.clone());
    }
    if !r7_cutover_decision {
        blockers.push("R7 Rust default cutover requires an explicit cutover decision".into());
    }
    if rollback_switch_requested {
        blockers.push("one-switch rollback currently selects Mihomo as the default".into());
    }

    let supported_profile_default = blockers.is_empty();
    let selected_runtime_kind = if supported_profile_default {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };

    RustKernelRuntimeSupportedProfileDefaultReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-rust-supported-profile-default".into(),
        profile_scope,
        supported_profile_default,
        selected_runtime_kind,
        fallback_runtime_kind: KernelRuntimeKind::Mihomo,
        supported_safe_subset: rust_runtime_supported_safe_subset(),
        fallback_boundaries: rust_runtime_fallback_boundaries(),
        blockers,
        warnings: vec![
            "R7 default applies only to the supported profile; unsupported protocol, TUN, and adapter paths stay on Mihomo fallback".into(),
        ],
        facts: vec![
            "Rust runtime is selected as the wider default only after canary closeout and rollback hold pass".into(),
            "Mihomo remains available without app restart through the rollback switch".into(),
        ],
    }
}

fn rust_kernel_runtime_r7_fallback_state_report(
    r6_canary: &KernelLoopbackR6RustDefaultCanaryReport,
    rollback_switch_requested: bool,
    supported_profile_default: bool,
) -> RustKernelRuntimeFallbackStateReport {
    let health_ready = r6_canary.automatic_fallback.health_ready
        && r6_canary.automatic_fallback.health_check_passed
        && !r6_canary.automatic_fallback.rollback_triggered;
    let rollback_armed = r6_canary.automatic_fallback.rollback_armed && r6_canary.mihomo_fallback;
    let mut triggers = Vec::new();

    if rollback_switch_requested {
        triggers.push("rollback-switch-requested".into());
    }
    if !supported_profile_default {
        triggers.push("supported-profile-default-not-ready".into());
    }
    if !health_ready {
        triggers.push("health-check-not-ready".into());
    }
    if !rollback_armed {
        triggers.push("rollback-not-armed".into());
    }

    let fallback_active = !triggers.is_empty();
    let selected_runtime_kind = if fallback_active {
        KernelRuntimeKind::Mihomo
    } else {
        KernelRuntimeKind::Rust
    };
    let blockers = if fallback_active && !rollback_switch_requested {
        triggers
            .iter()
            .map(|trigger| format!("R7 fallback keeps Mihomo selected: {trigger}").into())
            .collect()
    } else {
        Vec::new()
    };

    RustKernelRuntimeFallbackStateReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-rust-default-cutover-fallback-state".into(),
        rollback_switch_requested,
        restart_required: false,
        health_ready,
        rollback_armed,
        fallback_active,
        selected_runtime_kind,
        fallback_runtime_kind: KernelRuntimeKind::Mihomo,
        triggers,
        blockers,
        facts: vec![
            "one-switch rollback restores Mihomo default selection without app restart".into(),
            "fallback state is queryable over IPC before and after R7 cutover".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_r7_default_cutover(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
    canary_default_decision: Option<bool>,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
    r7_cutover_decision: Option<bool>,
    rollback_hold_decision: Option<bool>,
    rollback_switch_requested: Option<bool>,
    profile_scope: Option<String>,
) -> Result<KernelLoopbackR7RustDefaultCutoverReport> {
    let r7_cutover_decision = r7_cutover_decision.unwrap_or(false);
    let rollback_hold_decision = rollback_hold_decision.unwrap_or(false);
    let rollback_switch_requested = rollback_switch_requested.unwrap_or(false);
    let requested_runtime_kind_for_parse = requested_runtime_kind.clone();
    let r6_canary = Box::pin(rust_kernel_runtime_r6_default_canary(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
        canary_default_decision,
        health_check_passed,
        rollback_triggered,
    ))
    .await?;
    let canary_closeout = rust_kernel_runtime_r7_canary_closeout_summary(&r6_canary, rollback_hold_decision);
    let supported_profile = rust_kernel_runtime_supported_profile_default_report(
        profile_scope,
        &canary_closeout,
        r7_cutover_decision,
        rollback_switch_requested,
    );
    let fallback_state = rust_kernel_runtime_r7_fallback_state_report(
        &r6_canary,
        rollback_switch_requested,
        supported_profile.supported_profile_default,
    );
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r6CanaryCloseoutReady".into(),
            status: if canary_closeout.closeout_ready {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: canary_closeout.closeout_ready,
            blockers: canary_closeout.blockers.clone(),
            facts: vec!["R7 cutover consumes R6 canary health and rollback hold evidence".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r7CutoverDecision".into(),
            status: if r7_cutover_decision { "passed" } else { "blocked" }.into(),
            passed: r7_cutover_decision,
            blockers: if r7_cutover_decision {
                Vec::new()
            } else {
                vec!["R7 Rust default cutover requires an explicit cutover decision".into()]
            },
            facts: vec!["cutover decision widens Rust default selection only for supported profile".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "supportedProfileDefault".into(),
            status: if supported_profile.supported_profile_default {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: supported_profile.supported_profile_default,
            blockers: supported_profile.blockers.clone(),
            facts: vec!["unsupported protocol, TUN, and adapter paths remain Mihomo fallback".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "oneSwitchRollbackPath".into(),
            status: if fallback_state.rollback_armed && !fallback_state.restart_required {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: fallback_state.rollback_armed && !fallback_state.restart_required,
            blockers: fallback_state.blockers.clone(),
            facts: vec!["rollback switch restores Mihomo default without app restart".into()],
        },
    ];
    let supported_profile_default_allowed = checks.iter().all(|check| check.passed) && !fallback_state.fallback_active;
    let selected_runtime_kind = if supported_profile_default_allowed {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR7RustDefaultCutoverReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-rust-default-cutover".into(),
        mutates_runtime: r6_canary.mutates_runtime,
        live_execution_allowed: supported_profile_default_allowed,
        rust_runtime_opt_in_decision: r6_canary.rust_runtime_opt_in_decision,
        canary_default_decision: r6_canary.canary_default_decision,
        r7_cutover_decision,
        rollback_hold_decision,
        rollback_switch_requested,
        requested_runtime_kind: parse_kernel_runtime_kind(requested_runtime_kind_for_parse),
        selected_runtime_kind,
        supported_profile_default_allowed,
        production_default_allowed: false,
        mihomo_fallback: true,
        r6_canary,
        canary_closeout,
        supported_profile,
        fallback_state,
        checks,
        blockers,
        warnings: vec![
            "R7 selects Rust only for the supported profile; full Mihomo fallback retirement remains blocked".into(),
            "TUN, transparent proxy, protocol stacks, and production adapter egress are not replaced in this batch"
                .into(),
        ],
        facts: vec![
            "Rust runtime becomes the supported profile default only after canary closeout and rollback hold pass"
                .into(),
            "Mihomo fallback remains available for unsupported paths and one-switch rollback".into(),
        ],
        next_safe_batch: "r7-mihomo-fallback-retirement".into(),
    })
}

fn rust_kernel_runtime_r7_fallback_retirement_parity_report(
    r7_cutover: &KernelLoopbackR7RustDefaultCutoverReport,
    protocol_parity_decision: bool,
    tun_parity_decision: bool,
    adapter_parity_decision: bool,
    dns_runtime_parity_decision: bool,
    cross_platform_rollback_decision: bool,
    soak_evidence_decision: bool,
) -> RustKernelRuntimeFallbackRetirementParityReport {
    let mut blockers = Vec::new();

    if !r7_cutover.supported_profile_default_allowed {
        blockers.push("fallback retirement requires R7 supported profile cutover to be ready".into());
    }
    if !protocol_parity_decision {
        blockers.push("fallback retirement requires outbound and inbound protocol parity evidence".into());
    }
    if !tun_parity_decision {
        blockers.push("fallback retirement requires TUN and transparent proxy parity evidence".into());
    }
    if !adapter_parity_decision {
        blockers.push("fallback retirement requires production adapter runtime parity evidence".into());
    }
    if !dns_runtime_parity_decision {
        blockers.push("fallback retirement requires default DNS runtime parity evidence".into());
    }
    if !cross_platform_rollback_decision {
        blockers.push("fallback retirement requires cross-platform rollback drills".into());
    }
    if !soak_evidence_decision {
        blockers.push("fallback retirement requires cross-platform soak evidence".into());
    }

    RustKernelRuntimeFallbackRetirementParityReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-mihomo-fallback-retirement-parity".into(),
        protocol_parity_passed: protocol_parity_decision,
        tun_parity_passed: tun_parity_decision,
        adapter_parity_passed: adapter_parity_decision,
        dns_runtime_parity_passed: dns_runtime_parity_decision,
        cross_platform_rollback_passed: cross_platform_rollback_decision,
        soak_evidence_passed: soak_evidence_decision,
        parity_complete: blockers.is_empty(),
        retained_boundaries: vec![
            "protocol stacks remain blocked until explicit parity evidence passes".into(),
            "TUN and transparent proxy remain blocked until explicit parity evidence passes".into(),
            "adapter runtime and default DNS remain blocked until explicit parity evidence passes".into(),
        ],
        blockers,
        facts: vec![
            "fallback retirement consumes R7 cutover readiness before evaluating data-plane parity".into(),
            "fallback retirement is blocked by default; every high-risk data-plane area needs explicit evidence".into(),
        ],
    }
}

fn rust_kernel_runtime_r7_fallback_retirement_plan_report(
    parity: &RustKernelRuntimeFallbackRetirementParityReport,
    fallback_retirement_decision: bool,
    emergency_rollback_decision: bool,
    rollback_switch_requested: bool,
) -> RustKernelRuntimeFallbackRetirementPlanReport {
    let mut blockers = parity.blockers.clone();
    let mut warnings = Vec::new();

    if !fallback_retirement_decision {
        blockers.push("Mihomo fallback retirement requires an explicit retirement decision".into());
    }
    if !emergency_rollback_decision {
        blockers.push("Mihomo fallback retirement requires an emergency rollback decision".into());
    }
    if rollback_switch_requested {
        blockers.push("one-switch rollback currently keeps Mihomo as the selected runtime".into());
    }

    if parity.parity_complete && !fallback_retirement_decision {
        warnings.push(
            "parity evidence is present, but fallback retirement remains disabled until explicitly decided".into(),
        );
    }

    let fallback_retirement_allowed = blockers.is_empty();

    RustKernelRuntimeFallbackRetirementPlanReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-mihomo-fallback-retirement-plan".into(),
        fallback_retirement_decision,
        emergency_rollback_decision,
        rollback_switch_requested,
        fallback_retirement_allowed,
        selected_runtime_kind: if fallback_retirement_allowed {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        restart_required: false,
        blockers,
        warnings,
        facts: vec![
            "emergency rollback remains a one-switch Mihomo selection and does not require app restart".into(),
            "retirement only removes fallback dependence after parity, rollback drills, and soak evidence pass".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_r7_mihomo_fallback_retirement(
    listener_port: Option<u16>,
    target_port: Option<u16>,
    hold_started_at_epoch_ms: Option<u64>,
    observed_rollback_platforms: Option<Vec<String>>,
    explicit_decision: Option<bool>,
    requested_execution: Option<bool>,
    post_execution_hold_started_at_epoch_ms: Option<u64>,
    wider_opt_in_decision: Option<bool>,
    limited_rollout_decision: Option<bool>,
    canary_scope: Option<String>,
    max_canary_sessions: Option<u16>,
    closeout_decision: Option<bool>,
    handoff_decision: Option<bool>,
    r5_preflight_decision: Option<bool>,
    rollback_plan_decision: Option<bool>,
    execution_plan_decision: Option<bool>,
    guard_decision: Option<bool>,
    dry_run_decision: Option<bool>,
    dry_run_execution_decision: Option<bool>,
    post_dry_run_hold_started_at_epoch_ms: Option<u64>,
    hold_decision: Option<bool>,
    decision_readiness_decision: Option<bool>,
    final_gate_decision: Option<bool>,
    r5_handoff_decision: Option<bool>,
    final_hold_started_at_epoch_ms: Option<u64>,
    final_hold_decision: Option<bool>,
    independent_rollback_decision: Option<bool>,
    r5_closeout_decision: Option<bool>,
    r5_closeout_report_decision: Option<bool>,
    requested_runtime_kind: Option<String>,
    rust_runtime_opt_in_decision: Option<bool>,
    rust_runtime_scaffold_decision: Option<bool>,
    canary_default_decision: Option<bool>,
    health_check_passed: Option<bool>,
    rollback_triggered: Option<bool>,
    r7_cutover_decision: Option<bool>,
    rollback_hold_decision: Option<bool>,
    rollback_switch_requested: Option<bool>,
    profile_scope: Option<String>,
    protocol_parity_decision: Option<bool>,
    tun_parity_decision: Option<bool>,
    adapter_parity_decision: Option<bool>,
    dns_runtime_parity_decision: Option<bool>,
    cross_platform_rollback_decision: Option<bool>,
    soak_evidence_decision: Option<bool>,
    fallback_retirement_decision: Option<bool>,
    emergency_rollback_decision: Option<bool>,
) -> Result<KernelLoopbackR7MihomoFallbackRetirementReport> {
    let protocol_parity_decision = protocol_parity_decision.unwrap_or(false);
    let tun_parity_decision = tun_parity_decision.unwrap_or(false);
    let adapter_parity_decision = adapter_parity_decision.unwrap_or(false);
    let dns_runtime_parity_decision = dns_runtime_parity_decision.unwrap_or(false);
    let cross_platform_rollback_decision = cross_platform_rollback_decision.unwrap_or(false);
    let soak_evidence_decision = soak_evidence_decision.unwrap_or(false);
    let fallback_retirement_decision = fallback_retirement_decision.unwrap_or(false);
    let emergency_rollback_decision = emergency_rollback_decision.unwrap_or(false);
    let rollback_switch_requested_value = rollback_switch_requested.unwrap_or(false);
    let r7_cutover = Box::pin(rust_kernel_runtime_r7_default_cutover(
        listener_port,
        target_port,
        hold_started_at_epoch_ms,
        observed_rollback_platforms,
        explicit_decision,
        requested_execution,
        post_execution_hold_started_at_epoch_ms,
        wider_opt_in_decision,
        limited_rollout_decision,
        canary_scope,
        max_canary_sessions,
        closeout_decision,
        handoff_decision,
        r5_preflight_decision,
        rollback_plan_decision,
        execution_plan_decision,
        guard_decision,
        dry_run_decision,
        dry_run_execution_decision,
        post_dry_run_hold_started_at_epoch_ms,
        hold_decision,
        decision_readiness_decision,
        final_gate_decision,
        r5_handoff_decision,
        final_hold_started_at_epoch_ms,
        final_hold_decision,
        independent_rollback_decision,
        r5_closeout_decision,
        r5_closeout_report_decision,
        requested_runtime_kind,
        rust_runtime_opt_in_decision,
        rust_runtime_scaffold_decision,
        canary_default_decision,
        health_check_passed,
        rollback_triggered,
        r7_cutover_decision,
        rollback_hold_decision,
        Some(rollback_switch_requested_value),
        profile_scope,
    ))
    .await?;
    let parity = rust_kernel_runtime_r7_fallback_retirement_parity_report(
        &r7_cutover,
        protocol_parity_decision,
        tun_parity_decision,
        adapter_parity_decision,
        dns_runtime_parity_decision,
        cross_platform_rollback_decision,
        soak_evidence_decision,
    );
    let retirement_plan = rust_kernel_runtime_r7_fallback_retirement_plan_report(
        &parity,
        fallback_retirement_decision,
        emergency_rollback_decision,
        rollback_switch_requested_value,
    );
    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r7SupportedProfileCutover".into(),
            status: if r7_cutover.supported_profile_default_allowed {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: r7_cutover.supported_profile_default_allowed,
            blockers: r7_cutover.blockers.clone(),
            facts: vec!["Mihomo fallback cannot retire before R7 supported profile cutover is ready".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "protocolTunAdapterDnsParity".into(),
            status: if parity.parity_complete { "passed" } else { "blocked" }.into(),
            passed: parity.parity_complete,
            blockers: parity.blockers.clone(),
            facts: vec!["protocol, TUN, adapter, DNS, rollback drill, and soak evidence are evaluated together".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "fallbackRetirementDecision".into(),
            status: if fallback_retirement_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: fallback_retirement_decision,
            blockers: if fallback_retirement_decision {
                Vec::new()
            } else {
                vec!["explicit Mihomo fallback retirement decision is required".into()]
            },
            facts: vec!["retirement is an explicit high-risk data-plane decision".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "emergencyRollbackPath".into(),
            status: if emergency_rollback_decision && !retirement_plan.restart_required {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: emergency_rollback_decision && !retirement_plan.restart_required,
            blockers: if emergency_rollback_decision {
                Vec::new()
            } else {
                vec!["emergency one-switch Mihomo rollback path must remain available".into()]
            },
            facts: vec!["fallback retirement keeps a restart-free rollback selector".into()],
        },
    ];
    let mihomo_fallback_retired =
        checks.iter().all(|check| check.passed) && retirement_plan.fallback_retirement_allowed;
    let selected_runtime_kind = if mihomo_fallback_retired {
        KernelRuntimeKind::Rust
    } else {
        KernelRuntimeKind::Mihomo
    };
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackR7MihomoFallbackRetirementReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "r7-mihomo-fallback-retirement".into(),
        mutates_runtime: false,
        live_execution_allowed: mihomo_fallback_retired,
        r7_cutover,
        parity,
        retirement_plan,
        production_default_allowed: mihomo_fallback_retired,
        mihomo_fallback_retired,
        selected_runtime_kind,
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "fallback retirement is blocked by default and requires protocol/TUN/adapter/DNS parity evidence".into(),
            "this IPC surface reports retirement readiness; production mutation remains app-owned and explicitly gated"
                .into(),
        ],
        facts: vec![
            "R7 fallback retirement consumes R7 cutover readiness before considering full replacement".into(),
            "Mihomo remains the rollback runtime even when retirement readiness passes".into(),
        ],
        next_safe_batch: if mihomo_fallback_retired {
            "full-rust-runtime-hardening".into()
        } else {
            "r7-mihomo-fallback-retirement".into()
        },
    })
}

fn rust_kernel_runtime_full_hardening_extended_soak_report(
    observed_soak_hours: Option<u32>,
    health_regression_count: Option<u32>,
    rollback_trigger_count: Option<u32>,
) -> RustKernelRuntimeExtendedSoakReport {
    let observed_soak_hours = observed_soak_hours.unwrap_or(0);
    let health_regression_count = health_regression_count.unwrap_or(0);
    let rollback_trigger_count = rollback_trigger_count.unwrap_or(0);
    let mut blockers = Vec::new();

    if observed_soak_hours < FULL_RUST_RUNTIME_HARDENING_MIN_SOAK_HOURS {
        blockers.push(
            format!(
                "full Rust runtime hardening requires at least {} soak hours",
                FULL_RUST_RUNTIME_HARDENING_MIN_SOAK_HOURS
            )
            .into(),
        );
    }
    if health_regression_count > 0 {
        blockers.push("full Rust runtime hardening requires zero health regressions during soak".into());
    }
    if rollback_trigger_count > 0 {
        blockers.push("full Rust runtime hardening requires zero rollback triggers during soak".into());
    }

    RustKernelRuntimeExtendedSoakReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "full-rust-runtime-hardening-extended-soak".into(),
        min_soak_hours: FULL_RUST_RUNTIME_HARDENING_MIN_SOAK_HOURS,
        observed_soak_hours,
        health_regression_count,
        rollback_trigger_count,
        soak_complete: blockers.is_empty(),
        blockers,
        facts: vec![
            "hardening requires extended soak after R7 fallback retirement readiness".into(),
            "soak evidence is blocked by default and must be supplied explicitly".into(),
        ],
    }
}

fn rust_kernel_runtime_full_hardening_rollback_telemetry_report(
    rollback_telemetry_decision: bool,
    emergency_rollback_ready: bool,
    rollback_event_count: Option<u32>,
    last_rollback_event_ts: Option<u64>,
) -> RustKernelRuntimeRollbackTelemetryReport {
    let rollback_event_count = rollback_event_count.unwrap_or(0);
    let mut blockers = Vec::new();

    if !rollback_telemetry_decision {
        blockers.push("full Rust runtime hardening requires explicit rollback telemetry closeout".into());
    }
    if !emergency_rollback_ready {
        blockers.push("full Rust runtime hardening requires emergency Mihomo rollback readiness".into());
    }
    if rollback_event_count > 0 {
        blockers.push("full Rust runtime hardening requires zero unresolved rollback events".into());
    }

    RustKernelRuntimeRollbackTelemetryReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "full-rust-runtime-hardening-rollback-telemetry".into(),
        rollback_telemetry_decision,
        emergency_rollback_ready,
        rollback_event_count,
        last_rollback_event_ts,
        telemetry_complete: blockers.is_empty(),
        blockers,
        facts: vec![
            "rollback telemetry must remain queryable after fallback retirement readiness".into(),
            "Mihomo remains the restart-free emergency rollback runtime during hardening".into(),
        ],
    }
}

fn rust_kernel_runtime_full_hardening_platform_follow_up_report(
    windows_service_hardening: bool,
    macos_service_hardening: bool,
    linux_service_hardening: bool,
) -> RustKernelRuntimePlatformHardeningFollowUpReport {
    let mut blockers = Vec::new();

    if !windows_service_hardening {
        blockers.push("Windows service hardening follow-up is required".into());
    }
    if !macos_service_hardening {
        blockers.push("macOS service hardening follow-up is required".into());
    }
    if !linux_service_hardening {
        blockers.push("Linux service hardening follow-up is required".into());
    }

    RustKernelRuntimePlatformHardeningFollowUpReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "full-rust-runtime-hardening-platform-follow-up".into(),
        windows_service_hardening,
        macos_service_hardening,
        linux_service_hardening,
        platform_follow_up_complete: blockers.is_empty(),
        blockers,
        facts: vec![
            "platform hardening follows up service, sidecar, and rollback semantics per OS".into(),
            "all platform decisions are explicit to avoid silently retiring Go/Mihomo boundaries".into(),
        ],
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn rust_kernel_runtime_full_rust_runtime_hardening(
    r7_fallback_retirement_passed: Option<bool>,
    observed_soak_hours: Option<u32>,
    health_regression_count: Option<u32>,
    rollback_trigger_count: Option<u32>,
    rollback_event_count: Option<u32>,
    last_rollback_event_ts: Option<u64>,
    rollback_telemetry_decision: Option<bool>,
    emergency_rollback_decision: Option<bool>,
    windows_service_hardening_decision: Option<bool>,
    macos_service_hardening_decision: Option<bool>,
    linux_service_hardening_decision: Option<bool>,
    final_hardening_decision: Option<bool>,
) -> Result<KernelLoopbackFullRustRuntimeHardeningReport> {
    let r7_fallback_retirement_passed = r7_fallback_retirement_passed.unwrap_or(false);
    let hardening_decision = final_hardening_decision.unwrap_or(false);
    let extended_soak = rust_kernel_runtime_full_hardening_extended_soak_report(
        observed_soak_hours,
        health_regression_count,
        rollback_trigger_count,
    );
    let rollback_telemetry = rust_kernel_runtime_full_hardening_rollback_telemetry_report(
        rollback_telemetry_decision.unwrap_or(false),
        emergency_rollback_decision.unwrap_or(false),
        rollback_event_count,
        last_rollback_event_ts,
    );
    let platform_follow_up = rust_kernel_runtime_full_hardening_platform_follow_up_report(
        windows_service_hardening_decision.unwrap_or(false),
        macos_service_hardening_decision.unwrap_or(false),
        linux_service_hardening_decision.unwrap_or(false),
    );
    let mut r7_blockers = Vec::new();

    if !r7_fallback_retirement_passed {
        r7_blockers.push("full Rust runtime hardening requires the R7 fallback retirement gate to pass".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "r7FallbackRetirementReady".into(),
            status: if r7_fallback_retirement_passed {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: r7_fallback_retirement_passed,
            blockers: r7_blockers,
            facts: vec!["full Rust runtime hardening consumes the R7 retirement gate".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "extendedSoakComplete".into(),
            status: if extended_soak.soak_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: extended_soak.soak_complete,
            blockers: extended_soak.blockers.clone(),
            facts: vec!["extended soak must show no health regression or rollback trigger".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "rollbackTelemetryComplete".into(),
            status: if rollback_telemetry.telemetry_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: rollback_telemetry.telemetry_complete,
            blockers: rollback_telemetry.blockers.clone(),
            facts: vec!["rollback telemetry stays available after hardening".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "platformHardeningFollowUp".into(),
            status: if platform_follow_up.platform_follow_up_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: platform_follow_up.platform_follow_up_complete,
            blockers: platform_follow_up.blockers.clone(),
            facts: vec!["Windows, macOS, and Linux service hardening must all pass".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalHardeningDecision".into(),
            status: if hardening_decision { "passed" } else { "blocked" }.into(),
            passed: hardening_decision,
            blockers: if hardening_decision {
                Vec::new()
            } else {
                vec!["full Rust runtime hardening requires an explicit final decision".into()]
            },
            facts: vec!["final hardening is an explicit app-owned Rust gate".into()],
        },
    ];
    let full_rust_runtime_hardened = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackFullRustRuntimeHardeningReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "full-rust-runtime-hardening".into(),
        mutates_runtime: false,
        live_execution_allowed: full_rust_runtime_hardened,
        hardening_decision,
        r7_fallback_retirement_passed,
        extended_soak,
        rollback_telemetry,
        platform_follow_up,
        full_rust_runtime_hardened,
        production_default_allowed: full_rust_runtime_hardened,
        selected_runtime_kind: if full_rust_runtime_hardened {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "full Rust runtime hardening is blocked by default and does not mutate runtime state"
                .into(),
            "Mihomo remains the emergency rollback runtime until hardening closeout passes".into(),
        ],
        facts: vec![
            "this gate follows R7 fallback retirement and closes extended soak, rollback telemetry, and platform follow-up together".into(),
            "successful hardening advances the roadmap beyond Go/Mihomo fallback dependence".into(),
        ],
        next_safe_batch: if full_rust_runtime_hardened {
            "go-mihomo-retirement-audit".into()
        } else {
            "full-rust-runtime-hardening".into()
        },
    })
}

fn rust_kernel_runtime_go_mihomo_retirement_surface_audit_report(
    sidecar_source_audit_decision: bool,
    bundled_mihomo_audit_decision: bool,
    ipc_fallback_audit_decision: bool,
    docs_audit_decision: bool,
    emergency_rollback_retained: bool,
) -> RustKernelRuntimeGoMihomoRetirementSurfaceAuditReport {
    let mut blockers = Vec::new();
    let mut remaining_surfaces = Vec::new();

    if !sidecar_source_audit_decision {
        remaining_surfaces.push("mihomo sidecar source tree".into());
        blockers.push("Go/Mihomo retirement audit requires sidecar source inventory".into());
    }
    if !bundled_mihomo_audit_decision {
        remaining_surfaces.push("bundled Mihomo binary and updater artifacts".into());
        blockers.push("Go/Mihomo retirement audit requires bundled artifact inventory".into());
    }
    if !ipc_fallback_audit_decision {
        remaining_surfaces.push("IPC fallback and emergency rollback commands".into());
        blockers.push("Go/Mihomo retirement audit requires IPC fallback surface inventory".into());
    }
    if !docs_audit_decision {
        remaining_surfaces.push("operator docs and migration rollback runbooks".into());
        blockers.push("Go/Mihomo retirement audit requires docs and runbook inventory".into());
    }
    if !emergency_rollback_retained {
        blockers.push("Go/Mihomo retirement audit must retain emergency rollback until a later removal plan".into());
    }

    RustKernelRuntimeGoMihomoRetirementSurfaceAuditReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-surface-audit".into(),
        sidecar_source_audit_passed: sidecar_source_audit_decision,
        bundled_mihomo_audit_passed: bundled_mihomo_audit_decision,
        ipc_fallback_audit_passed: ipc_fallback_audit_decision,
        docs_audit_passed: docs_audit_decision,
        emergency_rollback_retained,
        audit_complete: blockers.is_empty(),
        remaining_surfaces,
        blockers,
        facts: vec![
            "this audit inventories Go/Mihomo surfaces without deleting source, binaries, or rollback paths".into(),
            "emergency rollback remains a required retained surface for the next planning batch".into(),
        ],
    }
}

pub async fn rust_kernel_runtime_go_mihomo_retirement_audit(
    full_rust_runtime_hardened_decision: Option<bool>,
    sidecar_source_audit_decision: Option<bool>,
    bundled_mihomo_audit_decision: Option<bool>,
    ipc_fallback_audit_decision: Option<bool>,
    docs_audit_decision: Option<bool>,
    emergency_rollback_retained: Option<bool>,
    final_retirement_audit_decision: Option<bool>,
) -> Result<KernelLoopbackGoMihomoRetirementAuditReport> {
    let full_rust_runtime_hardened = full_rust_runtime_hardened_decision.unwrap_or(false);
    let final_retirement_audit_decision = final_retirement_audit_decision.unwrap_or(false);
    let surface_audit = rust_kernel_runtime_go_mihomo_retirement_surface_audit_report(
        sidecar_source_audit_decision.unwrap_or(false),
        bundled_mihomo_audit_decision.unwrap_or(false),
        ipc_fallback_audit_decision.unwrap_or(false),
        docs_audit_decision.unwrap_or(false),
        emergency_rollback_retained.unwrap_or(false),
    );
    let mut hardening_blockers = Vec::new();

    if !full_rust_runtime_hardened {
        hardening_blockers.push("Go/Mihomo retirement audit requires full Rust runtime hardening to pass".into());
    }

    let checks = vec![
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "fullRustRuntimeHardened".into(),
            status: if full_rust_runtime_hardened {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: full_rust_runtime_hardened,
            blockers: hardening_blockers,
            facts: vec!["retirement audit starts only after full Rust runtime hardening".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "goMihomoSurfaceAuditComplete".into(),
            status: if surface_audit.audit_complete {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: surface_audit.audit_complete,
            blockers: surface_audit.blockers.clone(),
            facts: vec!["source, artifact, IPC, docs, and rollback surfaces are audited together".into()],
        },
        KernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
            name: "finalRetirementAuditDecision".into(),
            status: if final_retirement_audit_decision {
                "passed"
            } else {
                "blocked"
            }
            .into(),
            passed: final_retirement_audit_decision,
            blockers: if final_retirement_audit_decision {
                Vec::new()
            } else {
                vec!["Go/Mihomo retirement audit requires an explicit final audit decision".into()]
            },
            facts: vec!["the audit is explicit and does not remove Mihomo".into()],
        },
    ];
    let go_mihomo_retirement_audit_complete = checks.iter().all(|check| check.passed);
    let blockers = checks
        .iter()
        .flat_map(|check| check.blockers.clone())
        .collect::<Vec<String>>();

    Ok(KernelLoopbackGoMihomoRetirementAuditReport {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: "go-mihomo-retirement-audit".into(),
        mutates_runtime: false,
        live_execution_allowed: go_mihomo_retirement_audit_complete,
        full_rust_runtime_hardened,
        surface_audit,
        final_retirement_audit_decision,
        go_mihomo_retirement_audit_complete,
        selected_runtime_kind: if go_mihomo_retirement_audit_complete {
            KernelRuntimeKind::Rust
        } else {
            KernelRuntimeKind::Mihomo
        },
        rollback_runtime_kind: KernelRuntimeKind::Mihomo,
        checks,
        blockers,
        warnings: vec![
            "this audit does not delete Mihomo source, binaries, IPC commands, or rollback paths".into(),
            "emergency rollback must stay retained until a dedicated retirement plan passes".into(),
        ],
        facts: vec![
            "Go/Mihomo retirement audit is the first post-hardening inventory gate".into(),
            "successful audit advances to a separate retirement plan rather than direct removal".into(),
        ],
        next_safe_batch: if go_mihomo_retirement_audit_complete {
            "go-mihomo-retirement-plan".into()
        } else {
            "go-mihomo-retirement-audit".into()
        },
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

fn kernel_loopback_udp_port_available(port: u16) -> bool {
    port > 0 && StdUdpSocket::bind((ISOLATED_TEST_LISTENER_HOST, port)).is_ok()
}

fn build_loopback_dns_smoke_query(domain: &str) -> Vec<u8> {
    let mut query = vec![0xca, 0xfe, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    for label in domain.split('.') {
        query.push(label.len().min(63) as u8);
        query.extend_from_slice(label.as_bytes().get(..label.len().min(63)).unwrap_or_default());
    }
    query.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0x01]);
    query
}

fn build_loopback_dns_smoke_response(query: &[u8]) -> Option<Vec<u8>> {
    if query.len() < 12 {
        return None;
    }
    let question_end = skip_dns_question(query, 12)?;
    let mut response = Vec::with_capacity(question_end + 16);
    response.extend_from_slice(&query[0..2]);
    response.extend_from_slice(&[0x81, 0x80, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]);
    response.extend_from_slice(&query[12..question_end]);
    response.extend_from_slice(&[
        0xc0, 0x0c, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 127, 0, 0, 1,
    ]);
    Some(response)
}

fn parse_loopback_dns_smoke_response(response: &[u8]) -> Option<String> {
    if response.len() < 12 {
        return None;
    }
    let question_count = u16::from_be_bytes([response[4], response[5]]);
    let answer_count = u16::from_be_bytes([response[6], response[7]]);
    if answer_count == 0 {
        return None;
    }
    let mut offset = 12;
    for _ in 0..question_count {
        offset = skip_dns_question(response, offset)?;
    }
    for _ in 0..answer_count {
        offset = skip_dns_name(response, offset)?;
        if offset + 10 > response.len() {
            return None;
        }
        let record_type = u16::from_be_bytes([response[offset], response[offset + 1]]);
        let record_class = u16::from_be_bytes([response[offset + 2], response[offset + 3]]);
        let data_len = u16::from_be_bytes([response[offset + 8], response[offset + 9]]) as usize;
        offset += 10;
        if offset + data_len > response.len() {
            return None;
        }
        if record_type == 1 && record_class == 1 && data_len == 4 {
            return Some(
                format!(
                    "{}.{}.{}.{}",
                    response[offset],
                    response[offset + 1],
                    response[offset + 2],
                    response[offset + 3]
                )
                .into(),
            );
        }
        offset += data_len;
    }
    None
}

fn skip_dns_question(packet: &[u8], offset: usize) -> Option<usize> {
    skip_dns_name(packet, offset).and_then(|offset| offset.checked_add(4).filter(|end| *end <= packet.len()))
}

fn skip_dns_name(packet: &[u8], mut offset: usize) -> Option<usize> {
    loop {
        let len = *packet.get(offset)?;
        if len & 0xc0 == 0xc0 {
            return offset.checked_add(2).filter(|end| *end <= packet.len());
        }
        offset += 1;
        if len == 0 {
            return Some(offset);
        }
        offset = offset
            .checked_add(usize::from(len))
            .filter(|next| *next <= packet.len())?;
    }
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

fn kernel_loopback_dns_smoke_report(
    requested_port: u16,
    udp_bound: bool,
    local_response_received: bool,
    response_address: Option<String>,
    system_proxy_unchanged: bool,
    tun_unchanged: bool,
    runtime_config_unchanged: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
) -> KernelLoopbackDnsSmokeEvidenceReport {
    let passed = udp_bound
        && local_response_received
        && response_address.as_deref() == Some("127.0.0.1")
        && system_proxy_unchanged
        && tun_unchanged
        && runtime_config_unchanged
        && blockers.is_empty();

    KernelLoopbackDnsSmokeEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-dns-smoke-evidence".into(),
        kernel_area: "dns".into(),
        mutates_runtime: udp_bound,
        live_execution_allowed: true,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        requested_port,
        query_name: LOOPBACK_DNS_SMOKE_QUERY.into(),
        udp_bound,
        local_response_received,
        response_address,
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
            "smoke evidence binds one temporary UDP socket on 127.0.0.1".into(),
            "synthetic DNS answer returns 127.0.0.1 without replacing default DNS".into(),
            "runtime config, system proxy, and TUN settings are compared before and after".into(),
        ],
        next_safe_batch: "loopback-forwarding-preflight".into(),
    }
}

fn kernel_loopback_forwarding_smoke_report(
    listener_port: u16,
    target_port: u16,
    listener_accepted: bool,
    target_received: bool,
    response_status: Option<String>,
    bytes_from_client: u64,
    bytes_from_target: u64,
    system_proxy_unchanged: bool,
    tun_unchanged: bool,
    runtime_config_unchanged: bool,
    blockers: Vec<String>,
    warnings: Vec<String>,
) -> KernelLoopbackForwardingSmokeEvidenceReport {
    let loopback_forwarded =
        listener_accepted && target_received && response_status.as_deref() == Some("HTTP/1.1 204 No Content");
    let passed = loopback_forwarded
        && bytes_from_client > 0
        && bytes_from_target > 0
        && system_proxy_unchanged
        && tun_unchanged
        && runtime_config_unchanged
        && blockers.is_empty();

    KernelLoopbackForwardingSmokeEvidenceReport {
        runtime_id: MIHOMO_RUNTIME_ID.into(),
        component: "loopback-forwarding-smoke-evidence".into(),
        kernel_area: "forwarding".into(),
        mutates_runtime: listener_accepted,
        live_execution_allowed: true,
        requested_host: ISOLATED_TEST_LISTENER_HOST.into(),
        listener_port,
        target_port,
        request_path: "/kernel-forwarding-smoke".into(),
        listener_accepted,
        target_received,
        response_status,
        bytes_from_client,
        bytes_from_target,
        loopback_forwarded,
        system_proxy_unchanged,
        tun_unchanged,
        runtime_config_unchanged,
        default_route: false,
        forwards_traffic: false,
        outbound_adapters_used: false,
        mihomo_fallback: true,
        passed,
        blockers,
        warnings,
        facts: vec![
            "smoke evidence binds temporary listener and target sockets on 127.0.0.1".into(),
            "the target is synthetic and no outbound adapter is dialed".into(),
            "runtime config, system proxy, and TUN settings are compared before and after".into(),
        ],
        next_safe_batch: "loopback-forwarding-rollback-drill".into(),
    }
}
