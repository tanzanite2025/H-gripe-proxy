use crate::subscription::{
    fetch::FetchedSubscriptionPayload,
    format::{SubscriptionFormat, SubscriptionFormatDetection, parse_clash_yaml_subscription},
    model::SubscriptionArtifactRecord,
};
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use smartstring::alias::String;

#[derive(Debug, Clone)]
pub struct SubscriptionArtifactCandidate {
    pub record: SubscriptionArtifactRecord,
    pub raw_body: String,
    pub normalized_yaml: String,
    pub diagnostics: SubscriptionArtifactDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionArtifactDiagnostics {
    pub format_detection: SubscriptionFormatDetection,
    pub response: SubscriptionResponseDiagnostics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionResponseDiagnostics {
    pub status_code: u16,
    pub content_type: Option<String>,
    pub content_length: usize,
}

pub fn build_clash_yaml_artifact_candidate(
    fetched: &FetchedSubscriptionPayload,
    fetched_at: i64,
) -> Result<SubscriptionArtifactCandidate> {
    let status_code = fetched.metadata.status_code;
    if !(200..300).contains(&status_code) {
        bail!("failed to fetch remote profile with status {status_code}");
    }

    let raw_body = fetched.body.clone();
    let (mapping, format_detection) =
        parse_clash_yaml_subscription(raw_body.as_str(), fetched.metadata.content_type.as_deref())?;
    let normalized_yaml = serde_yaml_ng::to_string(&mapping)?;
    let record = build_artifact_record(
        raw_body.as_str(),
        fetched_at,
        fetched.metadata.content_type.clone(),
        Some(SubscriptionFormat::ClashYaml),
    );
    let diagnostics = SubscriptionArtifactDiagnostics {
        format_detection,
        response: SubscriptionResponseDiagnostics {
            status_code: fetched.metadata.status_code,
            content_type: fetched.metadata.content_type.clone(),
            content_length: fetched.metadata.content_length,
        },
    };

    Ok(SubscriptionArtifactCandidate {
        record,
        raw_body,
        normalized_yaml: normalized_yaml.into(),
        diagnostics,
    })
}

fn build_artifact_record(
    raw_body: &str,
    fetched_at: i64,
    content_type: Option<String>,
    detected_format: Option<SubscriptionFormat>,
) -> SubscriptionArtifactRecord {
    let content_hash: String = hex::encode(Sha256::digest(raw_body.as_bytes())).into();
    let suffix_len = content_hash.len().min(12);
    let version = format!("{fetched_at}-{}", &content_hash[..suffix_len]).into();

    SubscriptionArtifactRecord {
        version,
        content_hash,
        fetched_at,
        content_length: raw_body.len(),
        content_type,
        detected_format,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::{
        fetch::{FetchedSubscriptionPayload, ResponseMetadata},
        transport::TransportKind,
    };
    use reqwest::header::HeaderMap;

    fn fetched_payload(body: &str, content_type: Option<&str>) -> FetchedSubscriptionPayload {
        fetched_payload_with_status(body, content_type, 200)
    }

    fn fetched_payload_with_status(
        body: &str,
        content_type: Option<&str>,
        status_code: u16,
    ) -> FetchedSubscriptionPayload {
        FetchedSubscriptionPayload {
            body: body.into(),
            headers: HeaderMap::new(),
            metadata: ResponseMetadata {
                status_code,
                content_type: content_type.map(Into::into),
                content_length: body.len(),
                transport: TransportKind::Direct,
            },
        }
    }

    #[test]
    fn builds_clash_yaml_artifact_candidate() {
        let fetched = fetched_payload(
            r#"
proxies:
  - name: node-a
    type: ss
proxy-groups: []
"#,
            Some("application/yaml"),
        );

        let candidate =
            build_clash_yaml_artifact_candidate(&fetched, 123).expect("candidate should build");

        assert_eq!(candidate.record.detected_format, Some(SubscriptionFormat::ClashYaml));
        assert!(candidate.record.version.starts_with("123-"));
        assert!(candidate.normalized_yaml.contains("proxies"));
        assert_eq!(
            candidate.diagnostics.format_detection.format,
            SubscriptionFormat::ClashYaml
        );
    }

    #[test]
    fn rejects_non_clash_payload_before_candidate_creation() {
        let fetched = fetched_payload("<html><body>login</body></html>", Some("text/html"));

        let err = build_clash_yaml_artifact_candidate(&fetched, 123)
            .expect_err("html payload should not create artifact candidate");

        assert!(err.to_string().contains("instead of Clash YAML"));
    }

    #[test]
    fn rejects_non_successful_response_before_candidate_creation() {
        let fetched = fetched_payload_with_status("proxies: []", Some("application/yaml"), 404);

        let err = build_clash_yaml_artifact_candidate(&fetched, 123)
            .expect_err("non-2xx response should not create artifact candidate");

        assert!(err.to_string().contains("status 404"));
    }
}
