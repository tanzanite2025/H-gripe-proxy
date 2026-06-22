use super::{
    RUST_RUNTIME_ID, RustFallbackRetirementReadinessLockReport, RustFallbackRetirementReadinessManifest,
    RustFallbackRetirementReadinessStatus, RustFallbackRetirementScopeArea,
};
use crate::utils::dirs;
use anyhow::Result;
use smartstring::alias::String;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;

const RUST_FALLBACK_RETIREMENT_COMPONENT: &str = "rust-fallback-retirement-readiness";
const RUST_FALLBACK_RETIREMENT_KERNEL_AREA: &str = "fallback-retirement";
const RUST_FALLBACK_RETIREMENT_NEXT_BATCH: &str = "rust-runtime-real-canary";
const RUST_FALLBACK_RETIREMENT_MANIFEST_FILE: &str = "manifest.yaml";
const ROLLBACK_FILE: &str = "rollback.yaml";

pub async fn rust_fallback_retirement_readiness_manifest() -> Result<RustFallbackRetirementReadinessManifest> {
    let dns_rollback = fallback_retirement_runtime_path("rust-dns-runtime-parity")?;
    let adapter_rollback = fallback_retirement_runtime_path("rust-adapter-egress-parity")?;
    let tun_rollback = fallback_retirement_runtime_path("rust-tun-system-proxy-parity")?;
    let manifest_path = fallback_retirement_manifest_path()?;

    let mut supported_scope = vec![
        fallback_retirement_scope_area(
            "dns-runtime",
            "Rust DNS runtime patch/probe/apply/rollback parity",
            "Mihomo default DNS remains fallback until canary leak evidence passes",
            Some(dns_rollback),
            true,
            vec![
                "DNS fallback retirement requires applied Rust DNS parity rollback record".into(),
                "DNS fallback retirement requires post-canary DNS leak evidence".into(),
            ],
        )
        .await?,
        fallback_retirement_scope_area(
            "adapter-egress",
            "Rust DIRECT/REJECT/proxy-group egress selection and runtime patch parity",
            "Mihomo adapter/proxy protocol dialing remains fallback for unsupported nodes",
            Some(adapter_rollback),
            true,
            vec![
                "adapter fallback retirement requires applied Rust adapter rollback record".into(),
                "adapter fallback retirement requires canary egress health evidence".into(),
            ],
        )
        .await?,
        fallback_retirement_scope_area(
            "protocol-forwarding-subset",
            "Rust loopback TCP/HTTP accept loop and byte forwarding",
            "Mihomo SOCKS, remote proxy protocols, and default forwarding remain fallback",
            None,
            true,
            vec![
                "protocol fallback retirement requires real traffic canary beyond loopback smoke".into(),
                "protocol fallback retirement requires unsupported protocol escape evidence".into(),
            ],
        )
        .await?,
        fallback_retirement_scope_area(
            "tun-system-proxy",
            "Rust route-mode decision, OS system-proxy apply, TUN bridge, and rollback",
            "Mihomo/service packet capture and transparent forwarding remain fallback",
            Some(tun_rollback),
            true,
            vec![
                "TUN fallback retirement requires applied Rust TUN/system-proxy rollback record".into(),
                "TUN fallback retirement requires platform leak and route restoration evidence".into(),
            ],
        )
        .await?,
    ];

    let blockers = fallback_retirement_blockers(&supported_scope);
    let emergency_rollback_paths = supported_scope
        .iter()
        .filter_map(|area| {
            area.rollback_record_path
                .as_ref()
                .filter(|_| area.rollback_record_present)
                .cloned()
        })
        .collect();
    let status = if blockers.is_empty() {
        RustFallbackRetirementReadinessStatus::Ready
    } else {
        RustFallbackRetirementReadinessStatus::Blocked
    };
    for area in &mut supported_scope {
        area.fallback_retirement_allowed = status == RustFallbackRetirementReadinessStatus::Ready;
    }

    Ok(RustFallbackRetirementReadinessManifest {
        runtime_id: RUST_RUNTIME_ID.into(),
        component: RUST_FALLBACK_RETIREMENT_COMPONENT.into(),
        kernel_area: RUST_FALLBACK_RETIREMENT_KERNEL_AREA.into(),
        status,
        generated_at_epoch_seconds: fallback_retirement_epoch_seconds(),
        supported_scope,
        unsupported_fallback_scope: vec![
            "remote proxy protocols and adapter dialing".into(),
            "SOCKS protocol handling".into(),
            "system-wide packet capture default ownership".into(),
            "full Mihomo fallback binary removal".into(),
        ],
        emergency_rollback_paths,
        manifest_path: Some(manifest_path.to_string_lossy().to_string().into()),
        fallback_retirement_execution_allowed: status == RustFallbackRetirementReadinessStatus::Ready,
        mutates_runtime: false,
        removes_mihomo_fallback: false,
        blockers,
        warnings: vec![
            "readiness manifest does not remove Mihomo fallback".into(),
            "fallback retirement execution still requires real canary evidence".into(),
        ],
        facts: fallback_retirement_facts(),
        next_safe_batch: RUST_FALLBACK_RETIREMENT_NEXT_BATCH.into(),
    })
}

pub async fn lock_rust_fallback_retirement_readiness(
    explicit_opt_in: bool,
) -> Result<RustFallbackRetirementReadinessLockReport> {
    let mut manifest = rust_fallback_retirement_readiness_manifest().await?;
    let mut blockers = manifest.blockers.clone();
    if !explicit_opt_in {
        blockers.push("explicit opt-in is required to lock fallback retirement readiness".into());
    }

    if !blockers.is_empty() {
        return Ok(RustFallbackRetirementReadinessLockReport {
            status: RustFallbackRetirementReadinessStatus::Blocked,
            reason: "fallback retirement readiness lock is blocked".into(),
            manifest,
            explicit_opt_in,
            manifest_path: None,
            mutates_runtime: false,
            removes_mihomo_fallback: false,
            blockers,
            warnings: vec!["Mihomo fallback remains required for blocked or unsupported paths".into()],
            facts: fallback_retirement_facts(),
        });
    }

    manifest.status = RustFallbackRetirementReadinessStatus::Locked;
    manifest.fallback_retirement_execution_allowed = true;
    let manifest_path = fallback_retirement_manifest_path()?;
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&manifest_path, serde_yaml_ng::to_string(&manifest)?.as_bytes()).await?;

    Ok(RustFallbackRetirementReadinessLockReport {
        status: RustFallbackRetirementReadinessStatus::Locked,
        reason: "fallback retirement readiness manifest locked".into(),
        manifest,
        explicit_opt_in,
        manifest_path: Some(manifest_path.to_string_lossy().to_string().into()),
        mutates_runtime: false,
        removes_mihomo_fallback: false,
        blockers: Vec::new(),
        warnings: vec!["locked manifest only permits the next canary batch; it does not remove Mihomo fallback".into()],
        facts: fallback_retirement_facts(),
    })
}

async fn fallback_retirement_scope_area(
    area: &str,
    rust_owned_capability: &str,
    mihomo_fallback_scope: &str,
    rollback_record_path: Option<std::path::PathBuf>,
    canary_evidence_required: bool,
    missing_evidence_blockers: Vec<String>,
) -> Result<RustFallbackRetirementScopeArea> {
    let rollback_record_present = match rollback_record_path.as_ref() {
        Some(path) => fs::try_exists(path).await?,
        None => false,
    };
    let mut blockers = Vec::new();
    if rollback_record_path.is_some() && !rollback_record_present {
        blockers.push(format!("{area} rollback record is missing").into());
    }
    if canary_evidence_required {
        blockers.extend(missing_evidence_blockers);
    }

    Ok(RustFallbackRetirementScopeArea {
        area: area.into(),
        rust_owned_capability: rust_owned_capability.into(),
        mihomo_fallback_scope: mihomo_fallback_scope.into(),
        rollback_record_path: rollback_record_path.map(|path| path.to_string_lossy().to_string().into()),
        rollback_record_present,
        canary_evidence_required,
        fallback_retirement_allowed: false,
        blockers,
        warnings: vec!["unsupported traffic must keep Mihomo fallback until execution scope narrows".into()],
    })
}

fn fallback_retirement_blockers(scope: &[RustFallbackRetirementScopeArea]) -> Vec<String> {
    scope.iter().flat_map(|area| area.blockers.iter().cloned()).collect()
}

fn fallback_retirement_runtime_path(component: &str) -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?.join(component).join(ROLLBACK_FILE))
}

fn fallback_retirement_manifest_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join(RUST_FALLBACK_RETIREMENT_COMPONENT)
        .join(RUST_FALLBACK_RETIREMENT_MANIFEST_FILE))
}

fn fallback_retirement_facts() -> Vec<String> {
    vec![
        "readiness is based on concrete Rust parity artifacts and rollback records".into(),
        "Mihomo fallback remains retained for unsupported protocols and packet capture".into(),
        "the manifest is the input to real canary execution, not fallback removal".into(),
    ]
}

fn fallback_retirement_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
