use anyhow::{Context as _, Result, anyhow};
use serde_yaml_ng::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct FallbackFilterGeoipConfig {
    pub(super) geoip_enabled: bool,
    pub(super) geoip_code: &'static str,
}

pub(super) fn parse_geoip_filter(yaml: &str) -> Result<FallbackFilterGeoipConfig> {
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    let root = value
        .as_mapping()
        .ok_or_else(|| anyhow!("config root must be a YAML mapping"))?;
    let dns = root
        .get("dns")
        .and_then(Value::as_mapping)
        .ok_or_else(|| anyhow!("dns config is missing"))?;
    let fallback_filter = dns
        .get("fallback-filter")
        .and_then(Value::as_mapping)
        .ok_or_else(|| anyhow!("dns.fallback-filter is missing"))?;
    let geoip_enabled = fallback_filter
        .get("geoip")
        .and_then(Value::as_bool)
        .unwrap_or_default();
    if !geoip_enabled {
        return Err(anyhow!("fallback-filter geoip must be enabled for this bounded path"));
    }

    let geoip_code = fallback_filter
        .get("geoip-code")
        .and_then(Value::as_str)
        .unwrap_or("CN")
        .trim()
        .to_ascii_uppercase();
    match geoip_code.as_str() {
        "CN" => Ok(FallbackFilterGeoipConfig {
            geoip_enabled,
            geoip_code: "CN",
        }),
        "US" => Ok(FallbackFilterGeoipConfig {
            geoip_enabled,
            geoip_code: "US",
        }),
        _ => Err(anyhow!(
            "fallback-filter geoip-code {geoip_code} is outside the bounded Rust canary set"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cn_geoip_filter() {
        let config = parse_geoip_filter(
            r#"
dns:
  fallback-filter:
    geoip: true
    geoip-code: cn
"#,
        )
        .unwrap();

        assert_eq!(config.geoip_code, "CN");
    }
}
