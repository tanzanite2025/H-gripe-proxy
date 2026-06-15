use anyhow::{Result, bail};
use base64::{Engine as _, engine::general_purpose};
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Mapping;
use smartstring::alias::String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionFormat {
    ClashYaml,
    Base64Links,
    SingBox,
    Html,
    UnknownText,
}

impl SubscriptionFormat {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ClashYaml => "Clash YAML",
            Self::Base64Links => "base64 link subscription",
            Self::SingBox => "sing-box config",
            Self::Html => "HTML",
            Self::UnknownText => "unknown text",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionFormatDetection {
    pub format: SubscriptionFormat,
    pub reason: String,
    pub preview: String,
    pub top_level_keys: Vec<String>,
}

pub fn response_content_type(headers: &HeaderMap) -> std::string::String {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn payload_preview(data: &str, max_chars: usize) -> std::string::String {
    let first_line = data
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();

    let preview = first_line.chars().take(max_chars).collect::<std::string::String>();
    if preview.is_empty() {
        "<empty>".to_string()
    } else {
        preview
    }
}

pub fn detect_subscription_format(data: &str, content_type: Option<&str>) -> SubscriptionFormatDetection {
    let data = data.trim_start_matches('\u{feff}');
    let preview = payload_preview(data, 160).into();
    let content_type = content_type.unwrap_or_default().to_ascii_lowercase();

    if data.trim().is_empty() {
        return detection(
            SubscriptionFormat::UnknownText,
            "response body is empty",
            preview,
            Vec::new(),
        );
    }

    if content_type.contains("html") || looks_like_html_payload(data) {
        return detection(
            SubscriptionFormat::Html,
            "payload starts with HTML markup or was served as HTML",
            preview,
            Vec::new(),
        );
    }

    if let Ok(mapping) = serde_yaml_ng::from_str::<Mapping>(data) {
        let top_level_keys = yaml_top_level_keys(&mapping);
        if has_any_key(&mapping, &["proxies", "proxy-providers"]) {
            return detection(
                SubscriptionFormat::ClashYaml,
                "YAML contains Clash proxy definitions",
                preview,
                top_level_keys,
            );
        }

        if has_any_key(&mapping, &["outbounds", "inbounds"]) {
            return detection(
                SubscriptionFormat::SingBox,
                "mapping resembles a sing-box configuration instead of Clash YAML",
                preview,
                top_level_keys,
            );
        }

        return detection(
            SubscriptionFormat::UnknownText,
            "YAML mapping is missing `proxies` and `proxy-providers`",
            preview,
            top_level_keys,
        );
    }

    if looks_like_link_subscription(data) || decoded_base64_link_payload(data).is_some() {
        return detection(
            SubscriptionFormat::Base64Links,
            "payload contains proxy URI links instead of Clash YAML",
            preview,
            Vec::new(),
        );
    }

    detection(
        SubscriptionFormat::UnknownText,
        "payload is not valid YAML and did not match a supported subscription format",
        preview,
        Vec::new(),
    )
}

pub fn parse_clash_yaml_subscription(
    data: &str,
    content_type: Option<&str>,
) -> Result<(Mapping, SubscriptionFormatDetection)> {
    let data = data.trim_start_matches('\u{feff}');
    let detection = detect_subscription_format(data, content_type);

    if detection.format != SubscriptionFormat::ClashYaml {
        bail!(
            "subscription server returned {} instead of Clash YAML: {} (preview: {})",
            detection.format.label(),
            detection.reason,
            detection.preview
        );
    }

    let mapping = serde_yaml_ng::from_str::<Mapping>(data)?;
    Ok((mapping, detection))
}

fn detection(
    format: SubscriptionFormat,
    reason: &'static str,
    preview: String,
    top_level_keys: Vec<String>,
) -> SubscriptionFormatDetection {
    SubscriptionFormatDetection {
        format,
        reason: reason.into(),
        preview,
        top_level_keys,
    }
}

fn has_any_key(mapping: &Mapping, keys: &[&str]) -> bool {
    keys.iter().any(|key| mapping.contains_key(*key))
}

fn yaml_top_level_keys(mapping: &Mapping) -> Vec<String> {
    mapping.keys().filter_map(|key| key.as_str().map(Into::into)).collect()
}

fn looks_like_html_payload(data: &str) -> bool {
    let normalized = data.trim_start().chars().take(256).collect::<std::string::String>();
    let normalized = normalized.to_ascii_lowercase();

    normalized.starts_with("<!doctype html")
        || normalized.starts_with("<html")
        || normalized.contains("<head")
        || normalized.contains("<body")
        || normalized.contains("<script")
}

fn looks_like_link_subscription(data: &str) -> bool {
    data.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .take(8)
        .any(looks_like_proxy_uri)
}

fn looks_like_proxy_uri(line: &str) -> bool {
    let line = line.to_ascii_lowercase();
    [
        "ss://",
        "ssr://",
        "vmess://",
        "vless://",
        "trojan://",
        "hysteria://",
        "hysteria2://",
        "tuic://",
    ]
    .iter()
    .any(|scheme| line.starts_with(scheme))
}

fn decoded_base64_link_payload(data: &str) -> Option<std::string::String> {
    let mut normalized = data
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<std::string::String>();

    if normalized.len() < 16
        || !normalized
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '-' | '_' | '='))
    {
        return None;
    }

    let padding = (4 - normalized.len() % 4) % 4;
    normalized.extend(std::iter::repeat_n('=', padding));

    let decoded = general_purpose::STANDARD
        .decode(normalized.as_bytes())
        .or_else(|_| general_purpose::URL_SAFE.decode(normalized.as_bytes()))
        .ok()?;
    let decoded = std::string::String::from_utf8(decoded).ok()?;

    looks_like_link_subscription(&decoded).then_some(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_clash_yaml_subscription() {
        let detection = detect_subscription_format(
            r#"
proxies:
  - name: node-a
    type: ss
proxy-groups: []
"#,
            Some("application/yaml"),
        );

        assert_eq!(detection.format, SubscriptionFormat::ClashYaml);
        assert!(detection.top_level_keys.contains(&"proxies".into()));
    }

    #[test]
    fn detects_html_payload_from_content_and_header() {
        let detection = detect_subscription_format("<html><body>login</body></html>", Some("text/html"));

        assert_eq!(detection.format, SubscriptionFormat::Html);
        assert!(detection.reason.contains("HTML"));
    }

    #[test]
    fn detects_base64_link_subscription() {
        let encoded = general_purpose::STANDARD.encode("vmess://example\nss://example");
        let detection = detect_subscription_format(&encoded, Some("text/plain"));

        assert_eq!(detection.format, SubscriptionFormat::Base64Links);
    }

    #[test]
    fn detects_sing_box_mapping() {
        let detection = detect_subscription_format(
            r#"
outbounds:
  - type: direct
route:
  rules: []
"#,
            Some("application/json"),
        );

        assert_eq!(detection.format, SubscriptionFormat::SingBox);
        assert!(detection.top_level_keys.contains(&"outbounds".into()));
    }

    #[test]
    fn rejects_yaml_without_clash_proxy_keys() {
        let err = parse_clash_yaml_subscription(
            r#"
mixed-port: 7890
rules:
  - MATCH,DIRECT
"#,
            Some("application/yaml"),
        )
        .expect_err("runtime config without proxies should not be accepted");

        assert!(err.to_string().contains("instead of Clash YAML"));
    }

    #[test]
    fn payload_preview_uses_first_non_empty_line() {
        assert_eq!(payload_preview("\n\n  proxies:\n  - a", 16), "proxies:");
        assert_eq!(payload_preview("   \n", 16), "<empty>");
    }
}
