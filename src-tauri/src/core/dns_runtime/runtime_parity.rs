use super::*;
use crate::core::CoreManager;
use serde_yaml_ng::{Mapping, Value};
use std::time::{SystemTime, UNIX_EPOCH};

const RUST_DNS_RUNTIME_PARITY_ROLLBACK_FILE: &str = "rollback.yaml";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsRuntimeParityStatus {
    Ready,
    Applied,
    Restored,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsRuntimePatchPlan {
    pub patch_yaml: String,
    pub dns_keys: Vec<String>,
    pub hosts_keys: Vec<String>,
    pub nameservers: Vec<String>,
    pub supported_nameservers: Vec<String>,
    pub unsupported_nameservers: Vec<String>,
    pub preserved_features: Vec<String>,
    pub blocked_features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsRuntimeLeakCheck {
    pub check_id: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsRuntimeParityReport {
    pub status: RustDnsRuntimeParityStatus,
    pub reason: String,
    pub plan: DnsResolverPlan,
    pub probe: Option<DnsResolverRuntimeProbeReport>,
    pub patch: RustDnsRuntimePatchPlan,
    pub previous_patch_yaml: Option<String>,
    pub rollback_record_path: Option<String>,
    pub explicit_opt_in: bool,
    pub apply_runtime: bool,
    pub mutates_runtime: bool,
    pub reload_mihomo: bool,
    pub mihomo_fallback_retained: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RustDnsRuntimeRollbackRecord {
    previous_patch_yaml: String,
    applied_patch_yaml: String,
    created_at_epoch_seconds: u64,
}

pub async fn rust_dns_runtime_parity(
    yaml: String,
    test_domain: Option<String>,
    explicit_opt_in: bool,
    apply_runtime: bool,
) -> Result<RustDnsRuntimeParityReport> {
    let candidate = build_rust_dns_runtime_patch_plan(&yaml)?;
    let probe = if candidate.plan.status == DnsResolverPlanStatus::Ready {
        Some(dns_controlled_runtime_probe(&yaml, test_domain).await?)
    } else {
        None
    };
    let mut report = build_rust_dns_runtime_parity_report(
        candidate.plan,
        probe,
        candidate.patch_mapping,
        candidate.patch,
        explicit_opt_in,
        apply_runtime,
    );

    if apply_runtime && report.blockers.is_empty() {
        match apply_rust_dns_runtime_patch(&report.patch.patch_yaml).await {
            Ok((previous_patch_yaml, rollback_record_path)) => {
                report.status = RustDnsRuntimeParityStatus::Applied;
                report.reason = "Rust DNS runtime parity patch applied through the runtime config bridge".into();
                report.previous_patch_yaml = Some(previous_patch_yaml);
                report.rollback_record_path = Some(rollback_record_path);
                report.mutates_runtime = true;
                report.reload_mihomo = true;
            }
            Err(error) => {
                let message = error.to_string();
                report.blockers.push(message.clone().into());
                report.status = RustDnsRuntimeParityStatus::Blocked;
                report.reason = format!("Rust DNS runtime parity apply failed: {message}").into();
                crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                    "rust_dns_runtime_parity_apply",
                    false,
                    Some(message),
                    None,
                );
            }
        }
    }

    Ok(report)
}

pub async fn rust_dns_runtime_parity_rollback() -> Result<RustDnsRuntimeParityReport> {
    let rollback_record_path = rust_dns_runtime_parity_rollback_record_path()?;
    let record_yaml = fs::read_to_string(&rollback_record_path)
        .await
        .with_context(|| format!("failed to read {}", rollback_record_path.display()))?;
    let record: RustDnsRuntimeRollbackRecord = serde_yaml_ng::from_str(&record_yaml)
        .with_context(|| format!("failed to parse {}", rollback_record_path.display()))?;
    let patch = parse_patch_yaml(&record.previous_patch_yaml)?;
    Config::runtime().await.edit_draft(|draft| {
        draft.patch_dns_runtime_config(&patch);
    });
    CoreManager::global()
        .update_config_checked()
        .await
        .context("failed to restore DNS runtime config")?;
    crate::core::handle::Handle::refresh_clash();
    crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
        "rust_dns_runtime_parity_rollback",
        true,
        None,
        Some("restored previous DNS runtime patch".into()),
    );

    let plan = rejected_resolver_plan("rollback restored previous runtime DNS patch");
    Ok(RustDnsRuntimeParityReport {
        status: RustDnsRuntimeParityStatus::Restored,
        reason: "previous DNS runtime patch restored".into(),
        plan,
        probe: None,
        patch: patch_plan_from_mapping(&patch)?,
        previous_patch_yaml: Some(record.previous_patch_yaml),
        rollback_record_path: Some(rollback_record_path.to_string_lossy().to_string()),
        explicit_opt_in: true,
        apply_runtime: true,
        mutates_runtime: true,
        reload_mihomo: true,
        mihomo_fallback_retained: true,
        blockers: Vec::new(),
        warnings: Vec::new(),
        facts: vec![
            "rollback uses the Rust-owned DNS runtime rollback record".into(),
            "rollback restores dns/hosts through the same runtime config bridge".into(),
            "Mihomo remains the fallback runtime after rollback".into(),
        ],
    })
}

struct RustDnsRuntimePatchCandidate {
    plan: DnsResolverPlan,
    patch_mapping: Mapping,
    patch: RustDnsRuntimePatchPlan,
}

fn build_rust_dns_runtime_patch_plan(yaml: &str) -> Result<RustDnsRuntimePatchCandidate> {
    let plan = build_dns_resolver_plan(yaml)?;
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    let root = value
        .as_mapping()
        .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
    let dns = root
        .get("dns")
        .and_then(Value::as_mapping)
        .ok_or_else(|| anyhow!("dns config is missing"))?;

    let mut patch_mapping = Mapping::new();
    patch_mapping.insert("dns".into(), Value::Mapping(dns.clone()));
    if let Some(hosts) = root.get("hosts").cloned() {
        patch_mapping.insert("hosts".into(), hosts);
    }

    let patch = patch_plan_from_mapping_with_plan(&patch_mapping, &plan)?;
    Ok(RustDnsRuntimePatchCandidate {
        plan,
        patch_mapping,
        patch,
    })
}

fn build_rust_dns_runtime_parity_report(
    plan: DnsResolverPlan,
    probe: Option<DnsResolverRuntimeProbeReport>,
    patch_mapping: Mapping,
    patch: RustDnsRuntimePatchPlan,
    explicit_opt_in: bool,
    apply_runtime: bool,
) -> RustDnsRuntimeParityReport {
    let mut blockers = Vec::new();
    let mut warnings = plan.warnings.clone();
    let leak_checks = rust_dns_runtime_leak_checks(&plan, &patch, probe.as_ref());

    if plan.status != DnsResolverPlanStatus::Ready {
        blockers.push(plan.reason.clone());
    }
    if apply_runtime && !explicit_opt_in {
        blockers.push("runtime DNS parity apply requires explicit opt-in".into());
    }
    if apply_runtime && !patch.unsupported_nameservers.is_empty() {
        blockers.push(
            format!(
                "unsupported nameservers remain on Mihomo fallback: {}",
                patch.unsupported_nameservers.join(", ")
            )
            .into(),
        );
    }
    if apply_runtime && !patch.blocked_features.is_empty() {
        blockers.push(
            format!(
                "DNS features are not implemented by the Rust runtime subset: {}",
                patch.blocked_features.join(", ")
            )
            .into(),
        );
    }
    if apply_runtime && probe.as_ref().is_some_and(|probe| probe.summary.healthy_targets == 0) {
        blockers.push("controlled Rust DNS probe did not produce a healthy target".into());
    }
    for check in leak_checks.iter().filter(|check| !check.passed) {
        if apply_runtime {
            blockers.push(check.message.clone());
        } else {
            warnings.push(check.message.clone());
        }
    }

    let status = if blockers.is_empty() {
        RustDnsRuntimeParityStatus::Ready
    } else {
        RustDnsRuntimeParityStatus::Blocked
    };
    let reason = match status {
        RustDnsRuntimeParityStatus::Ready if apply_runtime => {
            "Rust DNS runtime parity patch is ready for explicit opt-in apply"
        }
        RustDnsRuntimeParityStatus::Ready => "Rust DNS runtime parity patch is ready for shadow comparison",
        RustDnsRuntimeParityStatus::Blocked => "Rust DNS runtime parity is blocked",
        RustDnsRuntimeParityStatus::Applied | RustDnsRuntimeParityStatus::Restored => {
            "Rust DNS runtime parity completed"
        }
    }
    .into();
    let mut facts = vec![
        "Rust synthesizes the dns/hosts runtime patch before Mihomo receives it".into(),
        "Rust resolver probes supported nameservers before opt-in apply".into(),
        "Mihomo fallback remains retained for unsupported DNS features and rollback".into(),
        format!(
            "dns patch keys={}",
            mapping_keys(&patch_mapping).into_iter().collect::<Vec<_>>().join(", ")
        )
        .into(),
    ];
    facts.extend(
        leak_checks
            .into_iter()
            .map(|check| format!("{}: {}", check.check_id, check.message).into()),
    );

    RustDnsRuntimeParityReport {
        status,
        reason,
        plan,
        probe,
        patch,
        previous_patch_yaml: None,
        rollback_record_path: None,
        explicit_opt_in,
        apply_runtime,
        mutates_runtime: false,
        reload_mihomo: false,
        mihomo_fallback_retained: true,
        blockers,
        warnings,
        facts,
    }
}

fn rust_dns_runtime_leak_checks(
    plan: &DnsResolverPlan,
    patch: &RustDnsRuntimePatchPlan,
    probe: Option<&DnsResolverRuntimeProbeReport>,
) -> Vec<RustDnsRuntimeLeakCheck> {
    vec![
        RustDnsRuntimeLeakCheck {
            check_id: "runtimeSupportedNameserver".into(),
            passed: !patch.supported_nameservers.is_empty(),
            message: if patch.supported_nameservers.is_empty() {
                "no runtime-supported Rust DNS nameserver is available".into()
            } else {
                format!(
                    "{} runtime-supported DNS nameserver(s) available",
                    patch.supported_nameservers.len()
                )
                .into()
            },
        },
        RustDnsRuntimeLeakCheck {
            check_id: "unsupportedFeatureFallback".into(),
            passed: patch.blocked_features.is_empty(),
            message: if patch.blocked_features.is_empty() {
                "configured DNS features fit the supported Rust runtime subset".into()
            } else {
                format!(
                    "unsupported DNS feature(s) require Mihomo fallback: {}",
                    patch.blocked_features.join(", ")
                )
                .into()
            },
        },
        RustDnsRuntimeLeakCheck {
            check_id: "controlledProbe".into(),
            passed: probe
                .map(|probe| probe.summary.healthy_targets > 0)
                .unwrap_or(plan.status != DnsResolverPlanStatus::Ready),
            message: probe
                .map(|probe| {
                    format!(
                        "{} healthy Rust DNS probe target(s), {} failed",
                        probe.summary.healthy_targets, probe.summary.failed_targets
                    )
                })
                .unwrap_or_else(|| "controlled probe skipped until resolver plan is ready".into()),
        },
    ]
}

async fn apply_rust_dns_runtime_patch(patch_yaml: &str) -> Result<(String, String)> {
    let patch = parse_patch_yaml(patch_yaml)?;
    let previous_patch = current_runtime_dns_patch().await;
    let previous_patch_yaml = mapping_to_yaml(&previous_patch)?;
    let rollback_record_path = rust_dns_runtime_parity_rollback_record_path()?;
    let record = RustDnsRuntimeRollbackRecord {
        previous_patch_yaml: previous_patch_yaml.clone(),
        applied_patch_yaml: patch_yaml.into(),
        created_at_epoch_seconds: rust_dns_runtime_epoch_seconds(),
    };
    if let Some(parent) = rollback_record_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&rollback_record_path, serde_yaml_ng::to_string(&record)?.as_bytes()).await?;

    Config::runtime().await.edit_draft(|draft| {
        draft.patch_dns_runtime_config(&patch);
    });
    CoreManager::global()
        .update_config_checked()
        .await
        .context("failed to apply Rust DNS runtime patch")?;
    crate::core::handle::Handle::refresh_clash();
    crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
        "rust_dns_runtime_parity_apply",
        true,
        None,
        Some("applied Rust-owned DNS runtime patch".into()),
    );
    Ok((previous_patch_yaml, rollback_record_path.to_string_lossy().to_string()))
}

async fn current_runtime_dns_patch() -> Mapping {
    let runtime = Config::runtime().await;
    let config = runtime.latest_arc().config.clone();
    let mut patch = Mapping::new();
    for key in ["dns", "hosts"] {
        if let Some(value) = config.as_ref().and_then(|config| config.get(key)).cloned() {
            patch.insert(key.into(), value);
        } else {
            patch.insert(key.into(), Value::Null);
        }
    }
    patch
}

fn patch_plan_from_mapping(patch: &Mapping) -> Result<RustDnsRuntimePatchPlan> {
    let yaml = mapping_to_yaml(patch)?;
    let plan = build_dns_resolver_plan(&yaml)
        .unwrap_or_else(|_| rejected_resolver_plan("rollback patch has no active DNS plan"));
    patch_plan_from_mapping_with_plan(patch, &plan)
}

fn patch_plan_from_mapping_with_plan(patch: &Mapping, plan: &DnsResolverPlan) -> Result<RustDnsRuntimePatchPlan> {
    let dns = patch
        .get("dns")
        .and_then(Value::as_mapping)
        .cloned()
        .unwrap_or_default();
    let hosts_keys = patch
        .get("hosts")
        .and_then(Value::as_mapping)
        .map(mapping_keys)
        .unwrap_or_default();
    let nameservers = server_values_from_dns(&dns);
    let supported_nameservers = plan
        .nameservers
        .iter()
        .filter(|nameserver| nameserver.runtime_supported)
        .map(|nameserver| nameserver.server.clone())
        .collect::<Vec<_>>();
    let unsupported_nameservers = plan
        .nameservers
        .iter()
        .filter(|nameserver| !nameserver.runtime_supported)
        .map(|nameserver| nameserver.server.clone())
        .collect::<Vec<_>>();
    let blocked_features = blocked_dns_runtime_features(&dns);
    let mut preserved_features = Vec::new();
    if !hosts_keys.is_empty() {
        preserved_features.push("hosts".into());
    }
    if dns.contains_key("default-nameserver") {
        preserved_features.push("default-nameserver".into());
    }

    Ok(RustDnsRuntimePatchPlan {
        patch_yaml: mapping_to_yaml(patch)?,
        dns_keys: mapping_keys(&dns),
        hosts_keys,
        nameservers,
        supported_nameservers,
        unsupported_nameservers,
        preserved_features,
        blocked_features,
    })
}

fn blocked_dns_runtime_features(dns: &Mapping) -> Vec<String> {
    let mut features = Vec::new();
    let enhanced_mode = dns
        .get("enhanced-mode")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if enhanced_mode == "fake-ip" || dns.contains_key("fake-ip-range") {
        features.push("fake-ip".into());
    }
    if dns.contains_key("fallback-filter") {
        features.push("fallback-filter".into());
    }
    if dns
        .get("nameserver-policy")
        .and_then(Value::as_mapping)
        .is_some_and(|policy| !policy.is_empty())
    {
        features.push("nameserver-policy".into());
    }
    features
}

fn server_values_from_dns(dns: &Mapping) -> Vec<String> {
    match dns.get("nameserver") {
        Some(Value::Sequence(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(Into::into)
            .collect(),
        Some(Value::String(item)) => {
            let item = item.trim();
            if item.is_empty() { Vec::new() } else { vec![item.into()] }
        }
        _ => Vec::new(),
    }
}

fn parse_patch_yaml(yaml: &str) -> Result<Mapping> {
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    value
        .as_mapping()
        .cloned()
        .ok_or_else(|| anyhow!("DNS runtime patch root must be a mapping"))
}

fn mapping_to_yaml(mapping: &Mapping) -> Result<String> {
    Ok(serde_yaml_ng::to_string(&Value::Mapping(mapping.clone()))?.into())
}

fn mapping_keys(mapping: &Mapping) -> Vec<String> {
    mapping.keys().filter_map(Value::as_str).map(Into::into).collect()
}

fn rust_dns_runtime_parity_rollback_record_path() -> Result<std::path::PathBuf> {
    Ok(dirs::app_runtime_dir()?
        .join("rust-dns-runtime-parity")
        .join(RUST_DNS_RUNTIME_PARITY_ROLLBACK_FILE))
}

fn rust_dns_runtime_epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_runtime_patch_with_dns_and_hosts() {
        let yaml = r#"
dns:
  enable: true
  nameserver:
    - 1.1.1.1
    - https://dns.google/dns-query
hosts:
  example.test: 127.0.0.1
"#;

        let candidate = build_rust_dns_runtime_patch_plan(yaml).unwrap();

        assert_eq!(candidate.plan.status, DnsResolverPlanStatus::Ready);
        assert_eq!(candidate.patch.supported_nameservers.len(), 2);
        assert!(candidate.patch.hosts_keys.contains(&"example.test".into()));
        assert!(candidate.patch.patch_yaml.contains("dns:"));
        assert!(candidate.patch.patch_yaml.contains("hosts:"));
    }

    #[test]
    fn blocks_fake_ip_from_supported_runtime_subset() {
        let yaml = r#"
dns:
  enable: true
  enhanced-mode: fake-ip
  fake-ip-range: 198.18.0.1/16
  nameserver:
    - 1.1.1.1
"#;

        let candidate = build_rust_dns_runtime_patch_plan(yaml).unwrap();

        assert!(candidate.patch.blocked_features.contains(&"fake-ip".into()));
    }

    #[test]
    fn detects_unsupported_nameserver_fallback() {
        let yaml = r#"
dns:
  enable: true
  nameserver:
    - system://default
"#;

        let candidate = build_rust_dns_runtime_patch_plan(yaml).unwrap();

        assert_eq!(candidate.plan.status, DnsResolverPlanStatus::Rejected);
        assert_eq!(candidate.patch.unsupported_nameservers, vec!["system://default"]);
    }
}
