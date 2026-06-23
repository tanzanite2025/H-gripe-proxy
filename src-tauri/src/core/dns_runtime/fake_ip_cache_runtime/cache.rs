use super::{RustDnsFakeIpCacheMappingEvidence, range::FakeIpRange};
use anyhow::Result;
use hickory_proto::rr::Name;
use smartstring::alias::String;
use std::collections::BTreeMap;

pub(super) fn build_fake_ip_cache_evidence(
    range: &FakeIpRange,
    domain: &str,
) -> Result<RustDnsFakeIpCacheMappingEvidence> {
    let domain = normalize_fake_ip_domain(domain)?;
    let fake_ip = range.allocate(&domain);
    let fake_ip_text: String = fake_ip.to_string().into();
    let mut forward_cache = BTreeMap::new();
    let mut reverse_cache = BTreeMap::new();
    forward_cache.insert(domain.clone(), fake_ip_text.clone());
    reverse_cache.insert(fake_ip_text.clone(), domain.clone());
    let cached_forward = forward_cache.get(&domain).cloned();
    let cached_reverse = cached_forward
        .as_ref()
        .and_then(|cached_fake_ip| reverse_cache.get(cached_fake_ip))
        .cloned()
        .unwrap_or_default();

    Ok(RustDnsFakeIpCacheMappingEvidence {
        domain: domain.clone(),
        fake_ip: fake_ip_text,
        fake_ip_range: range.to_string().into(),
        forward_cache_hit: cached_forward.is_some(),
        reverse_cache_hit: cached_reverse == domain,
        reverse_domain: cached_reverse,
        cache_entry_count: forward_cache.len(),
        deterministic: range.allocate(&domain) == fake_ip,
        range_member: range.contains(fake_ip),
    })
}

fn normalize_fake_ip_domain(domain: &str) -> Result<String> {
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if domain.is_empty() {
        anyhow::bail!("fake-ip cache domain is empty");
    }
    Name::from_str_relaxed(&domain)?;
    Ok(domain.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::dns_runtime::fake_ip_cache_runtime::range::FakeIpRange;

    #[test]
    fn builds_forward_and_reverse_cache_evidence() {
        let range = FakeIpRange::parse("198.18.0.1/16").unwrap();
        let evidence = build_fake_ip_cache_evidence(&range, "example.com").unwrap();

        assert!(evidence.forward_cache_hit);
        assert!(evidence.reverse_cache_hit);
        assert_eq!(evidence.reverse_domain, "example.com");
        assert!(evidence.range_member);
    }
}
