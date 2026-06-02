use serde::{Deserialize, Serialize};

use crate::core::current_egress_identity::{CurrentEgressIdentity, CurrentEgressIdentitySource};
use crate::core::ip_reputation::{IpReputation, IpType, ResidentialVerificationState, RiskLevel};
use crate::core::runtime_status::{DnsLeakTestResult, DnsRuntimeStatus};
use crate::tls_fingerprint::TlsFingerprint;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IdentityConsistencyLevel {
    Good,
    Warning,
    Danger,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum IdentityConsistencyIssueKind {
    MissingPublicEgress,
    LowEgressConfidence,
    HighIpRisk,
    DnsLeak,
    DnsRuntimeRisk,
    RandomTlsFingerprint,
    MissingTlsFingerprint,
    ObservationIncomplete,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityConsistencyIssue {
    pub kind: IdentityConsistencyIssueKind,
    pub severity: IdentityConsistencyLevel,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IdentityConsistencyReport {
    pub score: u8,
    pub level: IdentityConsistencyLevel,
    pub issues: Vec<IdentityConsistencyIssue>,
    pub public_egress_ip: Option<String>,
    pub proxy_chain: Vec<String>,
    pub ip_type: Option<IpType>,
    pub residential_state: Option<ResidentialVerificationState>,
    pub egress_source: Option<String>,
    pub egress_confidence: Option<i64>,
    pub tls_fingerprint: Option<String>,
    pub dns_assessment: Option<String>,
}

pub struct IdentityConsistencyInput<'a> {
    pub current_identity: &'a CurrentEgressIdentity,
    pub dns_runtime: Option<&'a DnsRuntimeStatus>,
    pub dns_leak: Option<&'a DnsLeakTestResult>,
    pub tls_fingerprint: Option<&'a TlsFingerprint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConsistencySnapshot {
    pub observed_at: String,
    pub report: IdentityConsistencyReport,
}

pub fn build_identity_consistency_report(input: IdentityConsistencyInput<'_>) -> IdentityConsistencyReport {
    let mut score: i32 = 100;
    let mut issues = Vec::new();
    let identity = input.current_identity;
    let reputation = identity.reputation.as_ref();
    let public_egress_ip = identity
        .public_egress_ip
        .clone()
        .or_else(|| identity.egress_ip.clone());

    if public_egress_ip.is_none() {
        push_issue(
            &mut issues,
            &mut score,
            IdentityConsistencyIssueKind::MissingPublicEgress,
            IdentityConsistencyLevel::Danger,
            35,
            "缺少已确认的公网出口 IP，当前节点类型判断可信度不足。",
        );
    }

    if matches!(identity.source, CurrentEgressIdentitySource::MihomoConnectionMetadata) {
        push_issue(
            &mut issues,
            &mut score,
            IdentityConsistencyIssueKind::ObservationIncomplete,
            IdentityConsistencyLevel::Warning,
            15,
            "当前仅有连接链路元数据，还没有公网出口观测。",
        );
    }

    if identity.confidence.unwrap_or(0) > 0 && identity.confidence.unwrap_or(0) < 70 {
        push_issue(
            &mut issues,
            &mut score,
            IdentityConsistencyIssueKind::LowEgressConfidence,
            IdentityConsistencyLevel::Warning,
            15,
            "公网出口观测置信度偏低。",
        );
    }

    apply_reputation_score(reputation, &mut issues, &mut score);
    apply_dns_score(input.dns_runtime, input.dns_leak, &mut issues, &mut score);
    apply_tls_score(input.tls_fingerprint, &mut issues, &mut score);

    let score = score.clamp(0, 100) as u8;
    let level = level_from_score_and_issues(score, &issues);

    IdentityConsistencyReport {
        score,
        level,
        issues,
        public_egress_ip,
        proxy_chain: identity.proxy_chain.clone(),
        ip_type: reputation.map(|item| item.ip_type.clone()),
        residential_state: reputation.map(|item| item.residential_state.clone()),
        egress_source: identity.egress_source.clone(),
        egress_confidence: identity.confidence,
        tls_fingerprint: input.tls_fingerprint.map(|item| item.name.clone()),
        dns_assessment: input.dns_leak.map(|item| item.assessment.to_string()),
    }
}

pub fn append_identity_consistency_snapshot(
    mut history: Vec<IdentityConsistencySnapshot>,
    report: IdentityConsistencyReport,
    observed_at: String,
    limit: usize,
) -> Vec<IdentityConsistencySnapshot> {
    if limit == 0 {
        return Vec::new();
    }

    if let Some(current) = history.first_mut()
        && current.report == report
    {
        current.observed_at = observed_at;
        history.truncate(limit);
        return history;
    }

    history.insert(0, IdentityConsistencySnapshot { observed_at, report });
    history.truncate(limit);
    history
}

fn apply_reputation_score(
    reputation: Option<&IpReputation>,
    issues: &mut Vec<IdentityConsistencyIssue>,
    score: &mut i32,
) {
    let Some(reputation) = reputation else {
        return;
    };

    if matches!(reputation.risk_level, RiskLevel::High | RiskLevel::VeryHigh) || reputation.fraud_score >= 70 {
        push_issue(
            issues,
            score,
            IdentityConsistencyIssueKind::HighIpRisk,
            IdentityConsistencyLevel::Danger,
            30,
            "当前公网出口 IP 风险评分偏高。",
        );
    }
}

fn apply_dns_score(
    dns_runtime: Option<&DnsRuntimeStatus>,
    dns_leak: Option<&DnsLeakTestResult>,
    issues: &mut Vec<IdentityConsistencyIssue>,
    score: &mut i32,
) {
    if let Some(leak) = dns_leak {
        if leak.has_leak || leak.observed_leak {
            push_issue(
                issues,
                score,
                IdentityConsistencyIssueKind::DnsLeak,
                IdentityConsistencyLevel::Danger,
                30,
                "DNS 观测存在泄漏或与当前出口不一致。",
            );
        }

        if leak.observation_incomplete {
            push_issue(
                issues,
                score,
                IdentityConsistencyIssueKind::ObservationIncomplete,
                IdentityConsistencyLevel::Warning,
                10,
                "DNS 泄漏观测不完整。",
            );
        }
    }

    if let Some(runtime) = dns_runtime {
        if runtime.derived.leak_protection_safe == Some(false) || !runtime.runtime_matches_saved {
            push_issue(
                issues,
                score,
                IdentityConsistencyIssueKind::DnsRuntimeRisk,
                IdentityConsistencyLevel::Warning,
                20,
                "DNS 运行时配置存在不一致或防护不足。",
            );
        }
    }
}

fn apply_tls_score(
    tls_fingerprint: Option<&TlsFingerprint>,
    issues: &mut Vec<IdentityConsistencyIssue>,
    score: &mut i32,
) {
    match tls_fingerprint {
        Some(fp) if fp.category == "random" || fp.name == "random" || fp.name == "randomized" => {
            push_issue(
                issues,
                score,
                IdentityConsistencyIssueKind::RandomTlsFingerprint,
                IdentityConsistencyLevel::Warning,
                20,
                "TLS 指纹使用随机策略，严格会话中可能造成身份漂移。",
            );
        }
        Some(_) => {}
        None => push_issue(
            issues,
            score,
            IdentityConsistencyIssueKind::MissingTlsFingerprint,
            IdentityConsistencyLevel::Warning,
            10,
            "未设置稳定 TLS 指纹。",
        ),
    }
}

fn push_issue(
    issues: &mut Vec<IdentityConsistencyIssue>,
    score: &mut i32,
    kind: IdentityConsistencyIssueKind,
    severity: IdentityConsistencyLevel,
    penalty: i32,
    message: &str,
) {
    *score -= penalty;
    issues.push(IdentityConsistencyIssue {
        kind,
        severity,
        message: message.to_string(),
    });
}

fn level_from_score_and_issues(score: u8, issues: &[IdentityConsistencyIssue]) -> IdentityConsistencyLevel {
    if issues
        .iter()
        .any(|issue| issue.severity == IdentityConsistencyLevel::Danger)
    {
        return IdentityConsistencyLevel::Danger;
    }

    match score {
        80..=100 => IdentityConsistencyLevel::Good,
        50..=79 => IdentityConsistencyLevel::Warning,
        1..=49 => IdentityConsistencyLevel::Danger,
        _ => IdentityConsistencyLevel::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::SystemTime;

    fn base_identity() -> CurrentEgressIdentity {
        CurrentEgressIdentity {
            source: CurrentEgressIdentitySource::MihomoEgressStatus,
            proxy_name: Some("proxy-a".to_string()),
            proxy_chain: vec!["proxy-a".to_string()],
            egress_ip: Some("198.51.100.20".to_string()),
            public_egress_ip: Some("198.51.100.20".to_string()),
            proxy_endpoint: Some("203.0.113.10:443".to_string()),
            destination_asn: None,
            asn_org: None,
            rule: Some("MATCH".to_string()),
            rule_payload: None,
            egress_source: Some("publicProbe".to_string()),
            confidence: Some(90),
            sample_count: Some(2),
            last_verified_at: Some("2026-06-02T02:00:00Z".to_string()),
            updated_at: Some("2026-06-02T02:00:00Z".to_string()),
            reputation: Some(base_reputation()),
            message: "test".to_string(),
        }
    }

    fn base_reputation() -> IpReputation {
        IpReputation {
            ip: "198.51.100.20".to_string(),
            ip_type: IpType::Residential,
            asn: "AS7922".to_string(),
            asn_org: "Comcast Cable Communications, LLC".to_string(),
            fraud_score: 20,
            risk_level: RiskLevel::Low,
            confidence: 90,
            evidence: Vec::new(),
            residential_state: ResidentialVerificationState::VerifiedResidential,
            is_proxy: false,
            is_vpn: false,
            is_tor: false,
            country_code: "US".to_string(),
            city: Some("Philadelphia".to_string()),
            checked_at: SystemTime::UNIX_EPOCH,
        }
    }

    fn stable_tls() -> TlsFingerprint {
        TlsFingerprint {
            name: "chrome".to_string(),
            description: "Chrome".to_string(),
            category: "browser".to_string(),
        }
    }

    fn random_tls() -> TlsFingerprint {
        TlsFingerprint {
            name: "randomized".to_string(),
            description: "Randomized".to_string(),
            category: "random".to_string(),
        }
    }

    #[test]
    fn high_confidence_residential_identity_scores_good() {
        let identity = base_identity();
        let tls = stable_tls();

        let report = build_identity_consistency_report(IdentityConsistencyInput {
            current_identity: &identity,
            dns_runtime: None,
            dns_leak: None,
            tls_fingerprint: Some(&tls),
        });

        assert_eq!(report.level, IdentityConsistencyLevel::Good);
        assert!(report.score >= 80);
        assert!(report.issues.is_empty());
    }

    #[test]
    fn missing_public_egress_and_random_tls_are_reported() {
        let mut identity = base_identity();
        identity.public_egress_ip = None;
        identity.egress_ip = None;
        identity.confidence = Some(0);
        let tls = random_tls();

        let report = build_identity_consistency_report(IdentityConsistencyInput {
            current_identity: &identity,
            dns_runtime: None,
            dns_leak: None,
            tls_fingerprint: Some(&tls),
        });

        assert_eq!(report.level, IdentityConsistencyLevel::Danger);
        assert!(report.score <= 45);
        assert!(report.issues.iter().any(|issue| issue.kind == IdentityConsistencyIssueKind::MissingPublicEgress));
        assert!(report.issues.iter().any(|issue| issue.kind == IdentityConsistencyIssueKind::RandomTlsFingerprint));
    }

    #[test]
    fn snapshot_history_coalesces_unchanged_reports_and_limits_size() {
        let mut report = build_identity_consistency_report(IdentityConsistencyInput {
            current_identity: &base_identity(),
            dns_runtime: None,
            dns_leak: None,
            tls_fingerprint: Some(&stable_tls()),
        });

        let history = append_identity_consistency_snapshot(Vec::new(), report.clone(), "t1".to_string(), 2);
        let history = append_identity_consistency_snapshot(history, report.clone(), "t2".to_string(), 2);

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].observed_at, "t2");

        report.public_egress_ip = Some("198.51.100.21".to_string());
        let history = append_identity_consistency_snapshot(history, report.clone(), "t3".to_string(), 2);

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].report.public_egress_ip.as_deref(), Some("198.51.100.21"));

        report.public_egress_ip = Some("198.51.100.22".to_string());
        let history = append_identity_consistency_snapshot(history, report, "t4".to_string(), 2);

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].report.public_egress_ip.as_deref(), Some("198.51.100.22"));
        assert_eq!(history[1].report.public_egress_ip.as_deref(), Some("198.51.100.21"));
    }
}
