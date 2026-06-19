use async_trait::async_trait;
use serde::Serialize;
use smartstring::alias::String;
use tauri_plugin_mihomo::models::Protocol;

use crate::core::{CoreManager, handle::Handle, manager::RunningMode};

const MIHOMO_RUNTIME_ID: &str = "mihomo-kernel-runtime";
const NEXT_SAFE_BATCH: &str = "rust-shadow-components";
const NEXT_SHADOW_BATCH: &str = "dns-shadow-resolver-evidence";

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
            status: "planned-read-only".into(),
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
            status: "planned-read-only".into(),
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
            status: "planned-read-only".into(),
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
            status: "planned-read-only".into(),
            mutates_runtime: false,
            live_execution_allowed: false,
            evidence: vec![
                "observe connection/session shape before Rust forwarding takeover".into(),
                "must keep Mihomo as the only live forwarding owner".into(),
            ],
            next_step: "connection-session-shadow-model".into(),
        },
    ]
}
