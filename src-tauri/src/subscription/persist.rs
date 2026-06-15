use crate::{
    subscription::{
        artifact::{SubscriptionArtifactCandidate, SubscriptionArtifactDiagnostics},
        events::{events_from_attempt_record, SubscriptionEvent},
        model::{
            SubscriptionArtifactRecord, SubscriptionAttemptRecord, SubscriptionSourceState,
            SubscriptionStageRecord, SubscriptionStateDocument, UpdateFinalStatus,
        },
    },
    utils::{dirs, help},
};
use anyhow::Result;
use chrono::Local;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;
use tokio::fs;

const ARTIFACT_DIAGNOSTICS_FILE: &str = "diagnostics.yaml";
const ARTIFACT_METADATA_FILE: &str = "metadata.yaml";
const ARTIFACT_NORMALIZED_FILE: &str = "normalized.yaml";
const ARTIFACT_RAW_FILE: &str = "raw.body";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionArtifactContentKind {
    RawBody,
    NormalizedYaml,
}

impl SubscriptionArtifactContentKind {
    fn file_name(self) -> &'static str {
        match self {
            Self::RawBody => ARTIFACT_RAW_FILE,
            Self::NormalizedYaml => ARTIFACT_NORMALIZED_FILE,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionArtifactContent {
    pub source_id: String,
    pub version: String,
    pub content_kind: SubscriptionArtifactContentKind,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionArtifactMetadata {
    pub source_id: String,
    pub artifact: SubscriptionArtifactRecord,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionArtifactSummary {
    pub source_id: String,
    pub artifact: SubscriptionArtifactRecord,
    pub has_diagnostics: bool,
    pub has_raw_body: bool,
    pub has_normalized_yaml: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionArtifactPublishResult {
    pub source_id: String,
    pub artifact_version: String,
    pub previous_active_artifact_version: Option<String>,
    pub published_at: i64,
}

async fn load_state_document() -> Result<SubscriptionStateDocument> {
    let path = dirs::subscription_state_path()?;
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(SubscriptionStateDocument::default());
    }

    help::read_yaml(&path).await
}

pub async fn read_subscription_state_document() -> Result<SubscriptionStateDocument> {
    load_state_document().await
}

pub fn find_subscription_source_state(
    state: &SubscriptionStateDocument,
    source_id: &str,
) -> Option<SubscriptionSourceState> {
    state
        .sources
        .iter()
        .find(|source_state| source_state.source_id == source_id)
        .cloned()
}

pub async fn read_subscription_source_state(
    source_id: &str,
) -> Result<Option<SubscriptionSourceState>> {
    let state = read_subscription_state_document().await?;
    Ok(find_subscription_source_state(&state, source_id))
}

pub async fn read_subscription_source_update_events(
    source_id: &str,
) -> Result<Vec<SubscriptionEvent>> {
    if !is_safe_subscription_artifact_path_segment(source_id) {
        anyhow::bail!("invalid subscription artifact path segment");
    }

    let Some(source_state) = read_subscription_source_state(source_id).await? else {
        return Ok(Vec::new());
    };
    let Some(attempt) = source_state.latest_attempt else {
        return Ok(Vec::new());
    };

    Ok(events_from_attempt_record(source_id, &attempt))
}

pub fn is_safe_subscription_artifact_path_segment(value: &str) -> bool {
    !value.is_empty()
        && value != "."
        && value != ".."
        && !value.contains('/')
        && !value.contains('\\')
}

pub async fn read_subscription_artifact_diagnostics(
    source_id: &str,
    version: &str,
) -> Result<Option<SubscriptionArtifactDiagnostics>> {
    ensure_safe_subscription_artifact_path(source_id, version)?;

    let path = dirs::subscription_artifact_version_dir(source_id, version)?
        .join(ARTIFACT_DIAGNOSTICS_FILE);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(None);
    }

    help::read_yaml(&path).await.map(Some)
}

pub async fn read_subscription_artifact_metadata(
    source_id: &str,
    version: &str,
) -> Result<Option<SubscriptionArtifactMetadata>> {
    ensure_safe_subscription_artifact_path(source_id, version)?;

    let path = dirs::subscription_artifact_version_dir(source_id, version)?
        .join(ARTIFACT_METADATA_FILE);
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(None);
    }

    let metadata: SubscriptionArtifactMetadata = help::read_yaml(&path).await?;
    validate_artifact_metadata(source_id, version, &metadata)?;
    Ok(Some(metadata))
}

pub async fn read_subscription_artifact_content(
    source_id: &str,
    version: &str,
    content_kind: SubscriptionArtifactContentKind,
) -> Result<Option<SubscriptionArtifactContent>> {
    ensure_safe_subscription_artifact_path(source_id, version)?;

    let path = dirs::subscription_artifact_version_dir(source_id, version)?
        .join(content_kind.file_name());
    if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
        return Ok(None);
    }

    let content = tokio::fs::read_to_string(&path).await?;
    Ok(Some(SubscriptionArtifactContent {
        source_id: source_id.into(),
        version: version.into(),
        content_kind,
        content: content.into(),
    }))
}

pub async fn list_subscription_artifact_metadata(
    source_id: &str,
) -> Result<Vec<SubscriptionArtifactMetadata>> {
    if !is_safe_subscription_artifact_path_segment(source_id) {
        anyhow::bail!("invalid subscription artifact path segment");
    }

    let dir = dirs::subscription_artifacts_dir(source_id)?;
    if !tokio::fs::try_exists(&dir).await.unwrap_or(false) {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(dir).await?;
    let mut artifacts = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        if !entry.file_type().await?.is_dir() {
            continue;
        }

        let version = entry.file_name().to_string_lossy().into_owned();
        if !is_safe_subscription_artifact_path_segment(version.as_str()) {
            continue;
        }

        let path = entry.path().join(ARTIFACT_METADATA_FILE);
        if !tokio::fs::try_exists(&path).await.unwrap_or(false) {
            continue;
        }

        let metadata: SubscriptionArtifactMetadata = help::read_yaml(&path).await?;
        validate_artifact_metadata(source_id, version.as_str(), &metadata)?;
        artifacts.push(metadata);
    }

    sort_artifact_metadata_newest_first(&mut artifacts);
    Ok(artifacts)
}

pub async fn list_subscription_artifact_summaries(
    source_id: &str,
) -> Result<Vec<SubscriptionArtifactSummary>> {
    if !is_safe_subscription_artifact_path_segment(source_id) {
        anyhow::bail!("invalid subscription artifact path segment");
    }

    let active_version = read_subscription_source_state(source_id)
        .await?
        .and_then(|source_state| source_state.active_artifact_version);
    let metadata = list_subscription_artifact_metadata(source_id).await?;
    let mut summaries = Vec::with_capacity(metadata.len());

    for item in metadata {
        let version = item.artifact.version.as_str();
        ensure_safe_subscription_artifact_path(source_id, version)?;
        let dir = dirs::subscription_artifact_version_dir(source_id, version)?;
        let has_diagnostics = tokio::fs::try_exists(dir.join(ARTIFACT_DIAGNOSTICS_FILE))
            .await
            .unwrap_or(false);
        let has_raw_body = tokio::fs::try_exists(dir.join(ARTIFACT_RAW_FILE))
            .await
            .unwrap_or(false);
        let has_normalized_yaml = tokio::fs::try_exists(dir.join(ARTIFACT_NORMALIZED_FILE))
            .await
            .unwrap_or(false);
        let is_active = active_version.as_deref() == Some(version);

        summaries.push(SubscriptionArtifactSummary {
            source_id: item.source_id,
            artifact: item.artifact,
            has_diagnostics,
            has_raw_body,
            has_normalized_yaml,
            is_active,
        });
    }

    Ok(summaries)
}

const DEFAULT_ARTIFACT_RETENTION: usize = 10;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionArtifactCleanupResult {
    pub source_id: String,
    pub retain_count: usize,
    pub removed_versions: Vec<String>,
    pub kept_versions: Vec<String>,
    pub active_version_preserved: bool,
}

pub async fn cleanup_subscription_artifacts(
    source_id: &str,
    retain_count: Option<usize>,
) -> Result<SubscriptionArtifactCleanupResult> {
    if !is_safe_subscription_artifact_path_segment(source_id) {
        anyhow::bail!("invalid subscription artifact path segment");
    }

    let retain = retain_count.unwrap_or(DEFAULT_ARTIFACT_RETENTION);
    if retain == 0 {
        anyhow::bail!("retain_count must be at least 1");
    }

    let active_version = read_subscription_source_state(source_id)
        .await?
        .and_then(|source_state| source_state.active_artifact_version);
    let artifacts = list_subscription_artifact_metadata(source_id).await?;
    let kept_versions =
        retained_artifact_versions(&artifacts, retain, active_version.as_deref());
    let mut removed_versions = Vec::new();

    for item in artifacts {
        let version = item.artifact.version;
        if kept_versions.iter().any(|kept| kept == &version) {
            continue;
        }

        ensure_safe_subscription_artifact_path(source_id, version.as_str())?;
        let dir = dirs::subscription_artifact_version_dir(source_id, version.as_str())?;
        if tokio::fs::try_exists(&dir).await.unwrap_or(false) {
            fs::remove_dir_all(&dir).await?;
        }
        removed_versions.push(version);
    }

    let active_version_preserved = active_version
        .as_deref()
        .is_some_and(|active| {
            kept_versions
                .iter()
                .any(|kept| kept.as_str() == active)
        });

    Ok(SubscriptionArtifactCleanupResult {
        source_id: source_id.into(),
        retain_count: retain,
        removed_versions,
        kept_versions,
        active_version_preserved,
    })
}

pub fn retained_artifact_versions(
    artifacts: &[SubscriptionArtifactMetadata],
    retain_count: usize,
    active_version: Option<&str>,
) -> Vec<String> {
    let mut versions: Vec<String> = artifacts
        .iter()
        .take(retain_count)
        .map(|item| item.artifact.version.clone())
        .collect();

    if let Some(active_version) = active_version {
        if artifacts
            .iter()
            .any(|item| item.artifact.version.as_str() == active_version)
            && !versions.iter().any(|version| version.as_str() == active_version)
        {
            versions.push(active_version.into());
        }
    }

    versions
}

pub fn sort_artifact_metadata_newest_first(artifacts: &mut [SubscriptionArtifactMetadata]) {
    artifacts.sort_by(|left, right| {
        right
            .artifact
            .fetched_at
            .cmp(&left.artifact.fetched_at)
            .then_with(|| right.artifact.version.cmp(&left.artifact.version))
    });
}

fn ensure_safe_subscription_artifact_path(source_id: &str, version: &str) -> Result<()> {
    if !is_safe_subscription_artifact_path_segment(source_id)
        || !is_safe_subscription_artifact_path_segment(version)
    {
        anyhow::bail!("invalid subscription artifact path segment");
    }
    Ok(())
}

fn validate_artifact_metadata(
    source_id: &str,
    version: &str,
    metadata: &SubscriptionArtifactMetadata,
) -> Result<()> {
    if metadata.source_id != source_id {
        anyhow::bail!("subscription artifact metadata source mismatch");
    }
    if metadata.artifact.version != version {
        anyhow::bail!("subscription artifact metadata version mismatch");
    }
    if !is_safe_subscription_artifact_path_segment(metadata.artifact.version.as_str()) {
        anyhow::bail!("invalid subscription artifact path segment");
    }
    Ok(())
}

async fn save_state_document(state: &SubscriptionStateDocument) -> Result<()> {
    let subscriptions_dir = dirs::subscriptions_dir()?;
    fs::create_dir_all(&subscriptions_dir).await?;

    let path = dirs::subscription_state_path()?;
    help::save_yaml(&path, state, Some("# Subscription State for Clash Verge Optimized")).await
}

pub async fn persist_artifact_candidate(
    source_id: &str,
    candidate: &SubscriptionArtifactCandidate,
) -> Result<()> {
    let dir = dirs::subscription_artifact_version_dir(source_id, candidate.record.version.as_str())?;
    fs::create_dir_all(&dir).await?;

    fs::write(dir.join("raw.body"), candidate.raw_body.as_bytes()).await?;
    fs::write(dir.join("normalized.yaml"), candidate.normalized_yaml.as_bytes())
        .await?;

    let diagnostics_path = dir.join(ARTIFACT_DIAGNOSTICS_FILE);
    help::save_yaml(
        &diagnostics_path,
        &candidate.diagnostics,
        Some("# Subscription Artifact Diagnostics"),
    )
    .await?;

    let metadata = SubscriptionArtifactMetadata {
        source_id: source_id.into(),
        artifact: candidate.record.clone(),
    };
    let metadata_path = dir.join(ARTIFACT_METADATA_FILE);
    help::save_yaml(&metadata_path, &metadata, Some("# Subscription Artifact Metadata")).await
}

pub async fn publish_subscription_artifact(
    source_id: &String,
    artifact: &SubscriptionArtifactRecord,
) -> Result<SubscriptionArtifactPublishResult> {
    if read_subscription_artifact_metadata(source_id.as_str(), artifact.version.as_str())
        .await?
        .is_none()
    {
        anyhow::bail!(
            "subscription artifact metadata is missing for version {}",
            artifact.version
        );
    }

    let mut state = load_state_document().await?;
    let source_state = ensure_subscription_source_state(&mut state, source_id);
    let previous_active_artifact_version = source_state.active_artifact_version.clone();

    source_state.latest_artifact = Some(artifact.clone());
    source_state.active_artifact_version = Some(artifact.version.clone());

    save_state_document(&state).await?;

    Ok(SubscriptionArtifactPublishResult {
        source_id: source_id.clone(),
        artifact_version: artifact.version.clone(),
        previous_active_artifact_version,
        published_at: Local::now().timestamp_millis(),
    })
}

pub async fn restore_subscription_active_artifact(
    source_id: &String,
    active_artifact_version: Option<String>,
) -> Result<()> {
    let mut state = load_state_document().await?;
    let source_state = ensure_subscription_source_state(&mut state, source_id);
    source_state.active_artifact_version = active_artifact_version;
    save_state_document(&state).await
}

pub async fn persist_attempt_result(
    source_id: &String,
    artifact: Option<&SubscriptionArtifactRecord>,
    attempt: &SubscriptionAttemptRecord,
) -> Result<()> {
    let mut state = load_state_document().await?;
    let source_state = ensure_subscription_source_state(&mut state, source_id);

    if let Some(artifact) = artifact {
        source_state.latest_artifact = Some(artifact.clone());
        if attempt.runtime_activated {
            source_state.active_artifact_version = Some(artifact.version.clone());
        }
    }

    source_state.latest_attempt = Some(attempt.clone());
    if attempt.final_status == UpdateFinalStatus::Succeeded {
        source_state.latest_success = Some(attempt.clone());
    }

    save_state_document(&state).await
}

fn ensure_subscription_source_state<'a>(
    state: &'a mut SubscriptionStateDocument,
    source_id: &String,
) -> &'a mut SubscriptionSourceState {
    if let Some(index) = state
        .sources
        .iter()
        .position(|source_state| source_state.source_id == *source_id)
    {
        return &mut state.sources[index];
    }

    state.sources.push(SubscriptionSourceState {
        source_id: source_id.clone(),
        active_artifact_version: None,
        latest_artifact: None,
        latest_attempt: None,
        latest_success: None,
    });
    state
        .sources
        .last_mut()
        .expect("just inserted subscription source state")
}

pub fn build_finished_attempt_record(
    attempt: &crate::subscription::model::SubscriptionUpdateAttempt,
    final_status: UpdateFinalStatus,
    stage: crate::subscription::model::UpdateStage,
    transport: Option<crate::subscription::transport::TransportKind>,
    artifact_version: Option<String>,
    error: Option<String>,
    runtime_activated: bool,
    active_artifact_unchanged: bool,
) -> SubscriptionAttemptRecord {
    let finished_at = Local::now().timestamp_millis();
    let mut stage_history = attempt.stage_history.clone();
    if stage_history
        .last()
        .is_none_or(|record| record.stage != stage || record.transport != transport)
    {
        stage_history.push(SubscriptionStageRecord {
            stage,
            changed_at: finished_at,
            transport,
        });
    }

    SubscriptionAttemptRecord {
        attempt_id: attempt.attempt_id.clone(),
        trigger: attempt.trigger,
        started_at: attempt.started_at,
        finished_at,
        final_status,
        stage,
        transport,
        artifact_version,
        error,
        runtime_activated,
        active_artifact_unchanged,
        stage_history,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::{
        model::{SubscriptionUpdateAttempt, UpdateStage, UpdateTrigger},
        transport::TransportKind,
    };

    #[test]
    fn finished_attempt_record_copies_stage_history() {
        let mut attempt = SubscriptionUpdateAttempt::new("source-a", UpdateTrigger::Manual);
        attempt.record_stage_changed(UpdateStage::FetchPayload, Some(TransportKind::Direct));

        let record = build_finished_attempt_record(
            &attempt,
            UpdateFinalStatus::Succeeded,
            UpdateStage::EmitFinalResult,
            Some(TransportKind::Direct),
            Some("artifact-a".into()),
            None,
            true,
            false,
        );

        assert_eq!(record.stage_history.len(), 2);
        assert_eq!(record.stage_history[0].stage, UpdateStage::FetchPayload);
        assert_eq!(record.stage_history[1].stage, UpdateStage::EmitFinalResult);
    }

    #[test]
    fn finds_subscription_source_state_by_source_id() {
        let state = SubscriptionStateDocument {
            sources: vec![SubscriptionSourceState {
                source_id: "source-a".into(),
                active_artifact_version: Some("artifact-a".into()),
                latest_artifact: None,
                latest_attempt: None,
                latest_success: None,
            }],
        };

        let found =
            find_subscription_source_state(&state, "source-a").expect("source should exist");

        assert_eq!(found.source_id, "source-a");
        assert_eq!(found.active_artifact_version.as_deref(), Some("artifact-a"));
        assert!(find_subscription_source_state(&state, "source-b").is_none());
    }

    #[test]
    fn validates_subscription_artifact_path_segments() {
        assert!(is_safe_subscription_artifact_path_segment("source-a"));
        assert!(is_safe_subscription_artifact_path_segment("1234567890-abcdef"));
        assert!(!is_safe_subscription_artifact_path_segment(""));
        assert!(!is_safe_subscription_artifact_path_segment("."));
        assert!(!is_safe_subscription_artifact_path_segment(".."));
        assert!(!is_safe_subscription_artifact_path_segment("../source-a"));
        assert!(!is_safe_subscription_artifact_path_segment("source\\a"));
    }

    #[test]
    fn maps_artifact_content_kinds_to_files() {
        assert_eq!(SubscriptionArtifactContentKind::RawBody.file_name(), "raw.body");
        assert_eq!(
            SubscriptionArtifactContentKind::NormalizedYaml.file_name(),
            "normalized.yaml"
        );
    }

    #[test]
    fn validates_artifact_metadata_identity() {
        let metadata = artifact_metadata("source-a", "artifact-a", 100);

        assert!(validate_artifact_metadata("source-a", "artifact-a", &metadata).is_ok());
        assert!(validate_artifact_metadata("source-b", "artifact-a", &metadata).is_err());
        assert!(validate_artifact_metadata("source-a", "artifact-b", &metadata).is_err());

        let unsafe_version = artifact_metadata("source-a", "../artifact-a", 100);
        assert!(
            validate_artifact_metadata("source-a", "../artifact-a", &unsafe_version).is_err()
        );
    }

    #[test]
    fn sorts_artifact_metadata_newest_first() {
        let mut artifacts = vec![
            artifact_metadata("source-a", "artifact-a", 100),
            artifact_metadata("source-a", "artifact-c", 200),
            artifact_metadata("source-a", "artifact-b", 200),
        ];

        sort_artifact_metadata_newest_first(&mut artifacts);

        assert_eq!(artifacts[0].artifact.version, "artifact-c");
        assert_eq!(artifacts[1].artifact.version, "artifact-b");
        assert_eq!(artifacts[2].artifact.version, "artifact-a");
    }

    #[test]
    fn retains_newest_artifacts_for_cleanup() {
        let artifacts = vec![
            artifact_metadata("source-a", "artifact-c", 300),
            artifact_metadata("source-a", "artifact-b", 200),
            artifact_metadata("source-a", "artifact-a", 100),
        ];

        let retained = retained_artifact_versions(&artifacts, 2, None);

        assert_eq!(artifact_versions(&retained), vec!["artifact-c", "artifact-b"]);
    }

    #[test]
    fn retains_active_artifact_even_when_old() {
        let artifacts = vec![
            artifact_metadata("source-a", "artifact-c", 300),
            artifact_metadata("source-a", "artifact-b", 200),
            artifact_metadata("source-a", "artifact-a", 100),
        ];

        let retained =
            retained_artifact_versions(&artifacts, 1, Some("artifact-a"));

        assert_eq!(artifact_versions(&retained), vec!["artifact-c", "artifact-a"]);
    }

    fn artifact_versions(versions: &[String]) -> Vec<&str> {
        versions.iter().map(String::as_str).collect()
    }

    fn artifact_metadata(
        source_id: &str,
        version: &str,
        fetched_at: i64,
    ) -> SubscriptionArtifactMetadata {
        SubscriptionArtifactMetadata {
            source_id: source_id.into(),
            artifact: SubscriptionArtifactRecord {
                version: version.into(),
                content_hash: format!("hash-{version}").into(),
                fetched_at,
                content_length: 0,
                content_type: None,
                detected_format: None,
            },
        }
    }
}
