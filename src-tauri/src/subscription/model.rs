use crate::{
    subscription::{format::SubscriptionFormat, transport::TransportKind},
    utils::help,
};
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
    DecodePayload,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionStageRecord {
    pub stage: UpdateStage,
    pub changed_at: i64,
    pub transport: Option<TransportKind>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubscriptionUpdateAttempt {
    pub attempt_id: String,
    pub source_id: String,
    pub trigger: UpdateTrigger,
    pub started_at: i64,
    pub stage_history: Vec<SubscriptionStageRecord>,
}

impl SubscriptionUpdateAttempt {
    pub fn new(source_id: impl Into<String>, trigger: UpdateTrigger) -> Self {
        Self {
            attempt_id: help::get_uid("ua").into(),
            source_id: source_id.into(),
            trigger,
            started_at: Local::now().timestamp_millis(),
            stage_history: Vec::new(),
        }
    }

    pub fn record_stage_changed(&mut self, stage: UpdateStage, transport: Option<TransportKind>) {
        self.stage_history.push(SubscriptionStageRecord {
            stage,
            changed_at: Local::now().timestamp_millis(),
            transport,
        });
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionArtifactRecord {
    pub version: String,
    pub content_hash: String,
    pub fetched_at: i64,
    pub content_length: usize,
    pub content_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_format: Option<SubscriptionFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionAttemptRecord {
    pub attempt_id: String,
    pub trigger: UpdateTrigger,
    pub started_at: i64,
    pub finished_at: i64,
    pub final_status: UpdateFinalStatus,
    pub stage: UpdateStage,
    pub transport: Option<TransportKind>,
    pub artifact_version: Option<String>,
    pub error: Option<String>,
    pub runtime_activated: bool,
    pub active_artifact_unchanged: bool,
    #[serde(default)]
    pub stage_history: Vec<SubscriptionStageRecord>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_attempt_records_stage_changes_in_order() {
        let mut attempt = SubscriptionUpdateAttempt::new("source-a", UpdateTrigger::Automatic);

        attempt.record_stage_changed(UpdateStage::ResolveTransportPlan, None);
        attempt.record_stage_changed(UpdateStage::FetchPayload, Some(TransportKind::LocalProxy));

        assert_eq!(attempt.stage_history.len(), 2);
        assert_eq!(attempt.stage_history[0].stage, UpdateStage::ResolveTransportPlan);
        assert_eq!(attempt.stage_history[1].stage, UpdateStage::FetchPayload);
        assert_eq!(attempt.stage_history[1].transport, Some(TransportKind::LocalProxy));
    }
}
