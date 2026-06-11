use crate::subscription::{
    model::{SubscriptionUpdateAttempt, UpdateFinalStatus, UpdateStage, UpdateTrigger},
    transport::TransportKind,
};
use serde::Serialize;
use smartstring::alias::String;

#[derive(Debug, Clone, Serialize)]
pub struct UpdateErrorView {
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SubscriptionEvent {
    AttemptStarted {
        source_id: String,
        attempt_id: String,
        trigger: crate::subscription::model::UpdateTrigger,
        started_at: i64,
    },
    StageChanged {
        source_id: String,
        attempt_id: String,
        stage: UpdateStage,
        transport: Option<TransportKind>,
    },
    UpdateFinished {
        source_id: String,
        attempt_id: String,
        trigger: UpdateTrigger,
        final_status: UpdateFinalStatus,
        stage: UpdateStage,
        transport: Option<TransportKind>,
        artifact_version: Option<String>,
        runtime_activated: bool,
        active_artifact_unchanged: bool,
        error: Option<UpdateErrorView>,
    },
}

impl SubscriptionEvent {
    pub fn attempt_started(attempt: &SubscriptionUpdateAttempt) -> Self {
        Self::AttemptStarted {
            source_id: attempt.source_id.clone(),
            attempt_id: attempt.attempt_id.clone(),
            trigger: attempt.trigger,
            started_at: attempt.started_at,
        }
    }

    pub fn stage_changed(
        attempt: &SubscriptionUpdateAttempt,
        stage: UpdateStage,
        transport: Option<TransportKind>,
    ) -> Self {
        Self::StageChanged {
            source_id: attempt.source_id.clone(),
            attempt_id: attempt.attempt_id.clone(),
            stage,
            transport,
        }
    }

    pub fn succeeded(
        attempt: &SubscriptionUpdateAttempt,
        transport: TransportKind,
        stage: UpdateStage,
        artifact_version: String,
        runtime_activated: bool,
        active_artifact_unchanged: bool,
    ) -> Self {
        Self::UpdateFinished {
            source_id: attempt.source_id.clone(),
            attempt_id: attempt.attempt_id.clone(),
            trigger: attempt.trigger,
            final_status: UpdateFinalStatus::Succeeded,
            stage,
            transport: Some(transport),
            artifact_version: Some(artifact_version),
            runtime_activated,
            active_artifact_unchanged,
            error: None,
        }
    }

    pub fn failed(
        attempt: &SubscriptionUpdateAttempt,
        stage: UpdateStage,
        transport: Option<TransportKind>,
        artifact_version: Option<String>,
        error: impl Into<String>,
        active_artifact_unchanged: bool,
    ) -> Self {
        Self::UpdateFinished {
            source_id: attempt.source_id.clone(),
            attempt_id: attempt.attempt_id.clone(),
            trigger: attempt.trigger,
            final_status: UpdateFinalStatus::Failed,
            stage,
            transport,
            artifact_version,
            runtime_activated: false,
            active_artifact_unchanged,
            error: Some(UpdateErrorView {
                message: error.into(),
            }),
        }
    }
}
