use super::{constants::DEFAULT_FAKE_IP_RANGE, range::FakeIpRange};
use anyhow::{Context as _, Result, anyhow};
use serde_yaml_ng::Value;

pub(super) fn fake_ip_range_from_yaml(yaml: &str) -> Result<FakeIpRange> {
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    let root = value
        .as_mapping()
        .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
    let dns = root
        .get("dns")
        .and_then(Value::as_mapping)
        .ok_or_else(|| anyhow!("dns config is missing"))?;
    let enhanced_mode = dns
        .get("enhanced-mode")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if enhanced_mode != "fake-ip" && !dns.contains_key("fake-ip-range") {
        return Err(anyhow!("dns config does not enable a fake-ip cache bounded scope"));
    }
    let range = dns
        .get("fake-ip-range")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_FAKE_IP_RANGE);
    FakeIpRange::parse(range.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fake_ip_range_from_dns_config() {
        let range = fake_ip_range_from_yaml(
            r#"
dns:
  enhanced-mode: fake-ip
  fake-ip-range: 198.18.0.1/16
"#,
        )
        .unwrap();

        assert!(range.contains(std::net::Ipv4Addr::new(198, 18, 1, 1)));
    }
}
