use crate::utils::help;
use chrono::Local;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateTrigger {
    Manual,
    Automatic,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateStage {
    ResolveSource,
    ResolveTransportPlan,
    FetchPayload,
    MaterializeArtifact,
    ActivateRuntime,
    EmitFinalResult,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UpdateFinalStatus {
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionUpdateAttempt {
    pub attempt_id: String,
    pub source_id: String,
    pub trigger: UpdateTrigger,
    pub started_at: i64,
}

impl SubscriptionUpdateAttempt {
    pub fn new(source_id: impl Into<String>, trigger: UpdateTrigger) -> Self {
        Self {
            attempt_id: help::get_uid("ua").into(),
            source_id: source_id.into(),
            trigger,
            started_at: Local::now().timestamp_millis(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionArtifactRecord {
    pub version: String,
    pub content_hash: String,
    pub fetched_at: i64,
    pub content_length: usize,
    pub content_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionAttemptRecord {
    pub attempt_id: String,
    pub trigger: UpdateTrigger,
    pub started_at: i64,
    pub finished_at: i64,
    pub final_status: UpdateFinalStatus,
    pub stage: UpdateStage,
    pub transport: Option<crate::subscription::transport::TransportKind>,
    pub artifact_version: Option<String>,
    pub error: Option<String>,
    pub runtime_activated: bool,
    pub active_artifact_unchanged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionSourceState {
    pub source_id: String,
    pub active_artifact_version: Option<String>,
    pub latest_artifact: Option<SubscriptionArtifactRecord>,
    pub latest_attempt: Option<SubscriptionAttemptRecord>,
    pub latest_success: Option<SubscriptionAttemptRecord>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubscriptionStateDocument {
    pub sources: Vec<SubscriptionSourceState>,
}
