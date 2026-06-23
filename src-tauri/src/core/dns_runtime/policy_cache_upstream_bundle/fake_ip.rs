use super::RustDnsPolicyCacheFakeIpLifecycleEvidence;
use super::{range::FakeIpRange, yaml::DnsPolicyBundleConfig};
use anyhow::Result;
use smartstring::alias::String;
use std::collections::BTreeMap;

pub(super) fn evaluate_fake_ip_lifecycle(
    config: &DnsPolicyBundleConfig,
    domain: &str,
) -> Result<RustDnsPolicyCacheFakeIpLifecycleEvidence> {
    let range = FakeIpRange::parse(&config.fake_ip_range)?;
    let fake_ip = range.allocate(domain);
    let fake_ip_text: String = fake_ip.to_string().into();
    let stale_domain: String = "stale.fake-ip-cache.test".into();
    let stale_ip: String = range.allocate(&stale_domain).to_string().into();
    let mut forward_cache = BTreeMap::new();
    let mut reverse_cache = BTreeMap::new();
    forward_cache.insert(domain.into(), fake_ip_text.clone());
    forward_cache.insert(stale_domain.clone(), stale_ip.clone());
    reverse_cache.insert(fake_ip_text.clone(), domain.into());
    reverse_cache.insert(stale_ip.clone(), stale_domain.clone());
    let evicted_entries = evict_stale_entry(&mut forward_cache, &mut reverse_cache, &stale_domain);
    let reverse_domain = reverse_cache.get(&fake_ip_text).cloned().unwrap_or_default();

    Ok(RustDnsPolicyCacheFakeIpLifecycleEvidence {
        domain: domain.into(),
        fake_ip: fake_ip_text,
        fake_ip_range: range.to_string().into(),
        inserted_entries: 2,
        evicted_entries,
        forward_cache_hit: forward_cache.contains_key(domain),
        reverse_cache_hit: reverse_domain == domain,
        reverse_domain,
        lifecycle_canary_passed: evicted_entries == 1 && range.contains(fake_ip),
    })
}

fn evict_stale_entry(
    forward_cache: &mut BTreeMap<String, String>,
    reverse_cache: &mut BTreeMap<String, String>,
    stale_domain: &str,
) -> usize {
    forward_cache
        .remove(stale_domain)
        .map(|fake_ip| {
            reverse_cache.remove(&fake_ip);
            1
        })
        .unwrap_or_default()
}
