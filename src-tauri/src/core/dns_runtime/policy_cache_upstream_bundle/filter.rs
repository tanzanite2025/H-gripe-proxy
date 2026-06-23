use super::RustDnsPolicyCacheFakeIpFilterEvidence;
use super::yaml::DnsPolicyBundleConfig;

pub(super) fn evaluate_fake_ip_filter(
    config: &DnsPolicyBundleConfig,
    domain: &str,
) -> RustDnsPolicyCacheFakeIpFilterEvidence {
    let matched_patterns = config
        .fake_ip_filters
        .iter()
        .filter(|pattern| wildcard_matches(pattern, domain))
        .cloned()
        .collect::<Vec<_>>();

    RustDnsPolicyCacheFakeIpFilterEvidence {
        domain: domain.into(),
        matched: !matched_patterns.is_empty(),
        matched_patterns,
        evaluated_pattern_count: config.fake_ip_filters.len(),
    }
}

pub(super) fn wildcard_matches(pattern: &str, domain: &str) -> bool {
    let pattern = pattern.trim().trim_end_matches('.').to_ascii_lowercase();
    let domain = domain.trim().trim_end_matches('.').to_ascii_lowercase();
    if pattern == "*" || pattern == domain {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("+.") {
        return domain == suffix || domain.ends_with(&format!(".{suffix}"));
    }
    if let Some(suffix) = pattern.strip_prefix("*.").or_else(|| pattern.strip_prefix('.')) {
        return domain.ends_with(&format!(".{suffix}"));
    }
    if !pattern.contains('*') {
        return false;
    }

    let mut remainder = domain.as_str();
    let anchored_start = !pattern.starts_with('*');
    let anchored_end = !pattern.ends_with('*');
    let parts = pattern.split('*').filter(|part| !part.is_empty());
    for (index, part) in parts.enumerate() {
        if index == 0 && anchored_start {
            if !remainder.starts_with(part) {
                return false;
            }
            remainder = &remainder[part.len()..];
            continue;
        }
        if let Some(position) = remainder.find(part) {
            remainder = &remainder[position + part.len()..];
        } else {
            return false;
        }
    }
    !anchored_end || remainder.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_supported_wildcards() {
        assert!(wildcard_matches("*.example.com", "www.example.com"));
        assert!(wildcard_matches("+.example.com", "example.com"));
        assert!(wildcard_matches("api.*.test", "api.dev.test"));
        assert!(!wildcard_matches("*.example.com", "example.com"));
    }
}
