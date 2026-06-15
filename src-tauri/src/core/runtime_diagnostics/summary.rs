use super::runtime_state::build_dns_runtime_status;
use crate::core::runtime_status::DnsRuntimeStatus;
use anyhow::Result;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RuntimeDiagnosticLevel {
    Ok,
    Unknown,
    Warning,
    Danger,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDiagnosticCheck {
    pub name: String,
    pub level: RuntimeDiagnosticLevel,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDiagnosticsSummary {
    pub healthy: bool,
    pub level: RuntimeDiagnosticLevel,
    pub explanation: String,
    pub checks: Vec<RuntimeDiagnosticCheck>,
}

pub async fn build_runtime_diagnostics_summary() -> Result<RuntimeDiagnosticsSummary> {
    let dns_status = build_dns_runtime_status().await?;
    Ok(build_runtime_diagnostics_summary_from_dns_status(&dns_status))
}

pub fn build_runtime_diagnostics_summary_from_dns_status(dns_status: &DnsRuntimeStatus) -> RuntimeDiagnosticsSummary {
    let checks = vec![
        dns_config_check(dns_status),
        runtime_dns_check(dns_status),
        runtime_sync_check(dns_status),
        leak_protection_check(dns_status),
        nameserver_check(dns_status),
    ];
    let level = checks
        .iter()
        .map(|check| check.level)
        .max()
        .unwrap_or(RuntimeDiagnosticLevel::Unknown);
    let healthy = !matches!(level, RuntimeDiagnosticLevel::Warning | RuntimeDiagnosticLevel::Danger);
    let problem_count = checks
        .iter()
        .filter(|check| {
            matches!(
                check.level,
                RuntimeDiagnosticLevel::Warning | RuntimeDiagnosticLevel::Danger
            )
        })
        .count();
    let explanation = match level {
        RuntimeDiagnosticLevel::Ok => "runtime diagnostics look healthy".to_owned(),
        RuntimeDiagnosticLevel::Unknown => "runtime diagnostics are incomplete".to_owned(),
        RuntimeDiagnosticLevel::Warning => format!("runtime diagnostics found {problem_count} warning(s)"),
        RuntimeDiagnosticLevel::Danger => format!("runtime diagnostics found {problem_count} critical issue(s)"),
    };

    RuntimeDiagnosticsSummary {
        healthy,
        level,
        explanation,
        checks,
    }
}

fn dns_config_check(status: &DnsRuntimeStatus) -> RuntimeDiagnosticCheck {
    if !status.enable_dns_settings {
        return check(
            "dns-config",
            RuntimeDiagnosticLevel::Unknown,
            "DNS settings are disabled in Verge config",
        );
    }
    if !status.dns_config_exists {
        return check(
            "dns-config",
            RuntimeDiagnosticLevel::Warning,
            "DNS settings are enabled but the generated DNS config file is missing",
        );
    }
    if !status.dns_config_valid {
        return check(
            "dns-config",
            RuntimeDiagnosticLevel::Danger,
            "generated DNS config exists but cannot be parsed",
        );
    }
    check(
        "dns-config",
        RuntimeDiagnosticLevel::Ok,
        "generated DNS config exists and is parseable",
    )
}

fn runtime_dns_check(status: &DnsRuntimeStatus) -> RuntimeDiagnosticCheck {
    if status.runtime_has_dns {
        check(
            "runtime-dns",
            RuntimeDiagnosticLevel::Ok,
            "runtime config contains DNS settings",
        )
    } else {
        check(
            "runtime-dns",
            RuntimeDiagnosticLevel::Warning,
            "runtime config does not contain DNS settings",
        )
    }
}

fn runtime_sync_check(status: &DnsRuntimeStatus) -> RuntimeDiagnosticCheck {
    if status.runtime_matches_saved {
        check(
            "runtime-sync",
            RuntimeDiagnosticLevel::Ok,
            "runtime DNS and hosts sections match generated config",
        )
    } else {
        check(
            "runtime-sync",
            RuntimeDiagnosticLevel::Warning,
            "runtime DNS or hosts sections differ from generated config",
        )
    }
}

fn leak_protection_check(status: &DnsRuntimeStatus) -> RuntimeDiagnosticCheck {
    match status.derived.leak_protection_safe {
        Some(true) => check(
            "leak-protection",
            RuntimeDiagnosticLevel::Ok,
            "DNS leak protection is enabled",
        ),
        Some(false) => check(
            "leak-protection",
            RuntimeDiagnosticLevel::Warning,
            "DNS leak protection is disabled",
        ),
        None => check(
            "leak-protection",
            RuntimeDiagnosticLevel::Unknown,
            "DNS leak protection level could not be inferred",
        ),
    }
}

fn nameserver_check(status: &DnsRuntimeStatus) -> RuntimeDiagnosticCheck {
    if status.snapshot.nameserver_count > 0 {
        check(
            "nameservers",
            RuntimeDiagnosticLevel::Ok,
            format!("runtime has {} DNS nameserver(s)", status.snapshot.nameserver_count),
        )
    } else {
        check(
            "nameservers",
            RuntimeDiagnosticLevel::Warning,
            "runtime has no DNS nameservers",
        )
    }
}

fn check(name: impl Into<String>, level: RuntimeDiagnosticLevel, message: impl Into<String>) -> RuntimeDiagnosticCheck {
    RuntimeDiagnosticCheck {
        name: name.into(),
        level,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime_status::{DnsRuntimeDerivedState, DnsRuntimeSnapshot};
    use smartstring::alias::String;

    fn dns_status() -> DnsRuntimeStatus {
        DnsRuntimeStatus {
            enable_dns_settings: true,
            dns_config_exists: true,
            dns_config_valid: true,
            runtime_has_dns: true,
            runtime_has_hosts: true,
            runtime_dns_matches_saved: true,
            runtime_hosts_matches_saved: true,
            runtime_matches_saved: true,
            snapshot: DnsRuntimeSnapshot {
                enhanced_mode: Some(String::from("fake-ip")),
                ipv6: Some(false),
                nameserver_count: 2,
                fallback_count: 1,
                nameserver_policy_count: 1,
                use_hosts: Some(true),
                use_system_hosts: Some(false),
                respect_rules: Some(true),
            },
            derived: DnsRuntimeDerivedState {
                routing_mode: Some(String::from("split")),
                domestic_dns: vec![String::from("https://dns.alidns.com/dns-query")],
                foreign_dns: vec![String::from("https://cloudflare-dns.com/dns-query")],
                default_nameserver_count: 1,
                default_nameserver_plain_count: 1,
                prefer_h3: Some(false),
                leak_protection_level: Some(String::from("strict")),
                leak_protection_security: Some(String::from("high")),
                leak_protection_safe: Some(true),
            },
        }
    }

    #[test]
    fn summary_is_healthy_when_all_checks_pass() {
        let summary = build_runtime_diagnostics_summary_from_dns_status(&dns_status());

        assert!(summary.healthy);
        assert_eq!(summary.level, RuntimeDiagnosticLevel::Ok);
        assert!(
            summary
                .checks
                .iter()
                .all(|check| check.level == RuntimeDiagnosticLevel::Ok)
        );
    }

    #[test]
    fn summary_escalates_to_danger_for_invalid_generated_dns_config() {
        let mut status = dns_status();
        status.dns_config_valid = false;

        let summary = build_runtime_diagnostics_summary_from_dns_status(&status);

        assert!(!summary.healthy);
        assert_eq!(summary.level, RuntimeDiagnosticLevel::Danger);
        assert!(summary.explanation.contains("critical"));
    }

    #[test]
    fn summary_flags_runtime_drift_as_warning() {
        let mut status = dns_status();
        status.runtime_matches_saved = false;

        let summary = build_runtime_diagnostics_summary_from_dns_status(&status);

        assert!(!summary.healthy);
        assert_eq!(summary.level, RuntimeDiagnosticLevel::Warning);
        assert!(summary.checks.iter().any(|check| check.name == "runtime-sync"));
    }
}
