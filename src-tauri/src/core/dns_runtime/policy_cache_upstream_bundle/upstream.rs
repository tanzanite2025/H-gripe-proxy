use super::RustDnsPolicyCacheFallbackUpstreamEvidence;
use super::{filter::wildcard_matches, yaml::DnsPolicyBundleConfig};
use anyhow::{Context as _, Result, anyhow};
use std::net::{IpAddr, Ipv6Addr};

pub(super) fn evaluate_fallback_upstream(
    config: &DnsPolicyBundleConfig,
    domain: &str,
    candidate_ip: &str,
) -> Result<RustDnsPolicyCacheFallbackUpstreamEvidence> {
    let candidate_ip = candidate_ip
        .trim()
        .parse::<IpAddr>()
        .with_context(|| format!("candidate IP is invalid: {candidate_ip}"))?;
    let domain_match = config
        .fallback_filter_domains
        .iter()
        .any(|rule| wildcard_matches(rule, domain));
    let ipcidr_match = config
        .fallback_filter_ipcidrs
        .iter()
        .any(|cidr| ip_matches_cidr(candidate_ip, cidr).unwrap_or(false));
    let fallback_required =
        domain_match || ipcidr_match || (!config.fallback_upstreams.is_empty() && is_public_candidate(candidate_ip));
    let selected_upstream = config.fallback_upstreams.first().cloned();
    let upstream_loopback_only = selected_upstream.as_deref().map(is_loopback_upstream).unwrap_or(false);
    let upstream_executed = fallback_required && upstream_loopback_only;

    Ok(RustDnsPolicyCacheFallbackUpstreamEvidence {
        domain: domain.into(),
        candidate_ip: candidate_ip.to_string().into(),
        fallback_required,
        selected_upstream,
        upstream_loopback_only,
        upstream_executed,
        canary_answer_ip: upstream_executed.then(|| candidate_ip.to_string().into()),
        evaluated_fallback_count: config.fallback_upstreams.len(),
    })
}

fn is_public_candidate(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !ip.is_private() && !ip.is_loopback() && !ip.is_link_local() && !ip.is_broadcast() && !ip.is_unspecified()
        }
        IpAddr::V6(ip) => !ip.is_loopback() && !ip.is_unspecified() && !is_unique_local_v6(ip),
    }
}

fn is_loopback_upstream(upstream: &str) -> bool {
    let upstream = upstream.trim();
    let host = upstream.split("://").nth(1).unwrap_or(upstream).trim_start_matches('[');
    let host = host.split('/').next().unwrap_or_default().trim_end_matches(']');
    let host = host
        .rsplit_once('@')
        .map(|(_, host)| host)
        .unwrap_or(host)
        .trim_start_matches('[');
    let host = host.split(':').next().unwrap_or_default().trim_end_matches(']');
    host.eq_ignore_ascii_case("localhost") || host.parse::<IpAddr>().map(|ip| ip.is_loopback()).unwrap_or(false)
}

fn ip_matches_cidr(ip: IpAddr, cidr: &str) -> Result<bool> {
    let (network, prefix) = cidr
        .split_once('/')
        .ok_or_else(|| anyhow!("fallback-filter ipcidr must be CIDR notation"))?;
    let prefix = prefix
        .parse::<u8>()
        .with_context(|| format!("invalid CIDR prefix: {prefix}"))?;
    match (ip, network.parse::<IpAddr>()?) {
        (IpAddr::V4(ip), IpAddr::V4(network)) => {
            if prefix > 32 {
                return Err(anyhow!("IPv4 CIDR prefix exceeds 32"));
            }
            let mask = if prefix == 0 { 0 } else { u32::MAX << (32 - prefix) };
            Ok((u32::from(ip) & mask) == (u32::from(network) & mask))
        }
        (IpAddr::V6(ip), IpAddr::V6(network)) => {
            if prefix > 128 {
                return Err(anyhow!("IPv6 CIDR prefix exceeds 128"));
            }
            let mask = if prefix == 0 { 0 } else { u128::MAX << (128 - prefix) };
            Ok((u128::from(ip) & mask) == (u128::from(network) & mask))
        }
        _ => Ok(false),
    }
}

fn is_unique_local_v6(ip: Ipv6Addr) -> bool {
    (ip.segments()[0] & 0xfe00) == 0xfc00
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_loopback_upstreams() {
        assert!(is_loopback_upstream("udp://127.0.0.1:5353"));
        assert!(is_loopback_upstream("localhost:53"));
        assert!(!is_loopback_upstream("https://dns.google/dns-query"));
    }
}
