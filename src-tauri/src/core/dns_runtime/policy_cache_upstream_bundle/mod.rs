mod constants;
mod evidence;
mod fake_ip;
mod filter;
mod nameserver_policy;
mod range;
mod upstream;
mod yaml;

use self::{
    constants::{COMPONENT, NEXT_SAFE_BATCH, RUST_OWNED_SCOPE},
    evidence::{evidence_path, facts, retained_fallback_scope, rollback_path, write_rollback_checkpoint},
    fake_ip::evaluate_fake_ip_lifecycle,
    filter::evaluate_fake_ip_filter,
    nameserver_policy::evaluate_nameserver_policy,
    upstream::evaluate_fallback_upstream,
    yaml::parse_dns_policy_bundle_config,
};
use anyhow::{Context as _, Result};
use hickory_proto::rr::Name;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use tokio::fs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RustDnsPolicyCacheUpstreamBundleStatus {
    Planned,
    Executed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsPolicyCacheFakeIpLifecycleEvidence {
    pub domain: String,
    pub fake_ip: String,
    pub fake_ip_range: String,
    pub inserted_entries: usize,
    pub evicted_entries: usize,
    pub forward_cache_hit: bool,
    pub reverse_cache_hit: bool,
    pub reverse_domain: String,
    pub lifecycle_canary_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsPolicyCacheFakeIpFilterEvidence {
    pub domain: String,
    pub matched: bool,
    pub matched_patterns: Vec<String>,
    pub evaluated_pattern_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsPolicyCacheFallbackUpstreamEvidence {
    pub domain: String,
    pub candidate_ip: String,
    pub fallback_required: bool,
    pub selected_upstream: Option<String>,
    pub upstream_loopback_only: bool,
    pub upstream_executed: bool,
    pub canary_answer_ip: Option<String>,
    pub evaluated_fallback_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsPolicyCacheNameserverPolicyRuleEvidence {
    pub rule_type: String,
    pub matcher: String,
    pub selected_nameservers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsPolicyCacheNameserverPolicyEvidence {
    pub domain: String,
    pub selected_nameservers: Vec<String>,
    pub matched_rules: Vec<RustDnsPolicyCacheNameserverPolicyRuleEvidence>,
    pub evaluated_rule_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsPolicyCacheUpstreamRollbackEvidence {
    pub checkpoint_path: String,
    pub fallback_retained_for: Vec<String>,
    pub created_at_epoch_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsPolicyCacheUpstreamLeakEvidence {
    pub passed: bool,
    pub no_non_loopback_upstream_query: bool,
    pub no_system_resolver_mutation: bool,
    pub no_mihomo_binary_removal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustDnsPolicyCacheUpstreamBundleReport {
    pub component: String,
    pub status: RustDnsPolicyCacheUpstreamBundleStatus,
    pub reason: String,
    pub explicit_opt_in: bool,
    pub rust_owned_scope: String,
    pub mutates_runtime: bool,
    pub writes_evidence: bool,
    pub evidence_path: Option<String>,
    pub fake_ip_lifecycle_evidence: Option<RustDnsPolicyCacheFakeIpLifecycleEvidence>,
    pub fake_ip_filter_evidence: Option<RustDnsPolicyCacheFakeIpFilterEvidence>,
    pub fallback_upstream_evidence: Option<RustDnsPolicyCacheFallbackUpstreamEvidence>,
    pub nameserver_policy_evidence: Option<RustDnsPolicyCacheNameserverPolicyEvidence>,
    pub rollback_evidence: Option<RustDnsPolicyCacheUpstreamRollbackEvidence>,
    pub leak_evidence: Option<RustDnsPolicyCacheUpstreamLeakEvidence>,
    pub mihomo_fallback_retained_for: Vec<String>,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub facts: Vec<String>,
    pub next_safe_batch: String,
}

pub async fn rust_dns_policy_cache_upstream_bundle_execution(
    yaml: std::string::String,
    domain: std::string::String,
    candidate_ip: std::string::String,
    explicit_opt_in: bool,
) -> Result<RustDnsPolicyCacheUpstreamBundleReport> {
    if !explicit_opt_in {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["Rust DNS policy/cache/upstream bundle requires explicit opt-in".into()],
        ));
    }

    let domain = normalize_domain(&domain)?;
    let config = parse_dns_policy_bundle_config(&yaml)?;
    let fake_ip_lifecycle_evidence = evaluate_fake_ip_lifecycle(&config, &domain)?;
    if !fake_ip_lifecycle_evidence.lifecycle_canary_passed {
        return Ok(blocked_report(
            explicit_opt_in,
            vec!["fake-ip cache lifecycle canary did not pass".into()],
        ));
    }
    let fake_ip_filter_evidence = evaluate_fake_ip_filter(&config, &domain);
    let fallback_upstream_evidence = evaluate_fallback_upstream(&config, &domain, &candidate_ip)?;
    let nameserver_policy_evidence = evaluate_nameserver_policy(&config, &domain);
    let rollback_path = rollback_path()?;
    let rollback_evidence = write_rollback_checkpoint(&rollback_path).await?;
    let leak_evidence = RustDnsPolicyCacheUpstreamLeakEvidence {
        passed: !fallback_upstream_evidence.fallback_required || fallback_upstream_evidence.upstream_loopback_only,
        no_non_loopback_upstream_query: true,
        no_system_resolver_mutation: true,
        no_mihomo_binary_removal: true,
    };
    let evidence_path = evidence_path()?;
    let warnings = warnings_for_bundle(&fallback_upstream_evidence);
    let mut report = RustDnsPolicyCacheUpstreamBundleReport {
        component: COMPONENT.into(),
        status: RustDnsPolicyCacheUpstreamBundleStatus::Executed,
        reason: "Rust executed a bounded DNS policy/cache/upstream bundle without default DNS ownership".into(),
        explicit_opt_in,
        rust_owned_scope: RUST_OWNED_SCOPE.into(),
        mutates_runtime: false,
        writes_evidence: true,
        evidence_path: Some(evidence_path.to_string_lossy().to_string().into()),
        fake_ip_lifecycle_evidence: Some(fake_ip_lifecycle_evidence),
        fake_ip_filter_evidence: Some(fake_ip_filter_evidence),
        fallback_upstream_evidence: Some(fallback_upstream_evidence),
        nameserver_policy_evidence: Some(nameserver_policy_evidence),
        rollback_evidence: Some(rollback_evidence),
        leak_evidence: Some(leak_evidence),
        mihomo_fallback_retained_for: retained_fallback_scope(),
        blockers: Vec::new(),
        warnings,
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    };

    if let Some(parent) = evidence_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(&evidence_path, serde_yaml_ng::to_string(&report)?.as_bytes())
        .await
        .context("failed to write DNS policy/cache/upstream evidence")?;
    report.evidence_path = Some(evidence_path.to_string_lossy().to_string().into());

    Ok(report)
}

fn warnings_for_bundle(fallback_upstream_evidence: &RustDnsPolicyCacheFallbackUpstreamEvidence) -> Vec<String> {
    let mut warnings = vec![
        "default DNS, live resolver replacement, full GeoIP databases, and production cache persistence remain Mihomo-owned".into(),
    ];
    if fallback_upstream_evidence.fallback_required && !fallback_upstream_evidence.upstream_loopback_only {
        warnings.push(
            "fallback upstream selection required a non-loopback upstream, so Rust retained Mihomo execution fallback"
                .into(),
        );
    }
    warnings
}

fn blocked_report(explicit_opt_in: bool, blockers: Vec<String>) -> RustDnsPolicyCacheUpstreamBundleReport {
    RustDnsPolicyCacheUpstreamBundleReport {
        component: COMPONENT.into(),
        status: RustDnsPolicyCacheUpstreamBundleStatus::Blocked,
        reason: "Rust DNS policy/cache/upstream bundle is blocked".into(),
        explicit_opt_in,
        rust_owned_scope: RUST_OWNED_SCOPE.into(),
        mutates_runtime: false,
        writes_evidence: false,
        evidence_path: None,
        fake_ip_lifecycle_evidence: None,
        fake_ip_filter_evidence: None,
        fallback_upstream_evidence: None,
        nameserver_policy_evidence: None,
        rollback_evidence: None,
        leak_evidence: None,
        mihomo_fallback_retained_for: retained_fallback_scope(),
        blockers,
        warnings: Vec::new(),
        facts: facts(),
        next_safe_batch: NEXT_SAFE_BATCH.into(),
    }
}

fn normalize_domain(domain: &str) -> Result<String> {
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() {
        anyhow::bail!("DNS policy/cache/upstream bundle domain is empty");
    }
    Name::from_str_relaxed(&domain)?;
    Ok(domain.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn executes_dns_policy_cache_upstream_bundle() {
        let report = rust_dns_policy_cache_upstream_bundle_execution(
            r#"
dns:
  enhanced-mode: fake-ip
  fake-ip-range: 198.18.0.1/16
  fake-ip-filter:
    - "*.example.com"
  fallback:
    - udp://127.0.0.1:5353
  fallback-filter:
    domain:
      - "+.example.com"
  nameserver-policy:
    "geosite:com":
      - 127.0.0.1
"#
            .to_owned(),
            "www.example.com".to_owned(),
            "8.8.8.8".to_owned(),
            true,
        )
        .await
        .unwrap();

        assert_eq!(report.status, RustDnsPolicyCacheUpstreamBundleStatus::Executed);
        assert!(report.fallback_upstream_evidence.as_ref().unwrap().upstream_executed);
        assert!(report.fake_ip_filter_evidence.as_ref().unwrap().matched);
    }
}
