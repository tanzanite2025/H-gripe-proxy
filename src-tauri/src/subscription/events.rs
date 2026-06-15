use crate::subscription::{
    model::{
        SubscriptionAttemptRecord, SubscriptionUpdateAttempt, UpdateFinalStatus, UpdateStage,
        UpdateTrigger,
    },
    transport::TransportKind,
};
use chrono::Local;
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
        trigger: UpdateTrigger,
        started_at: i64,
    },
    StageChanged {
        source_id: String,
        attempt_id: String,
        stage: UpdateStage,
        changed_at: i64,
        transport: Option<TransportKind>,
    },
    UpdateFinished {
        source_id: String,
        attempt_id: String,
        trigger: UpdateTrigger,
        finished_at: i64,
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
            changed_at: stage_changed_at(attempt, stage, transport),
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
            finished_at: Local::now().timestamp_millis(),
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
            finished_at: Local::now().timestamp_millis(),
            final_status: UpdateFinalStatus::Failed,
            stage,
            transport,
            artifact_version,
            runtime_activated: false,
            active_artifact_unchanged,
            error: Some(UpdateErrorView { message: error.into() }),
        }
    }
}

pub fn events_from_attempt_record(
    source_id: &str,
    attempt: &SubscriptionAttemptRecord,
) -> Vec<SubscriptionEvent> {
    let mut events = Vec::with_capacity(attempt.stage_history.len() + 2);
    events.push(SubscriptionEvent::AttemptStarted {
        source_id: source_id.into(),
        attempt_id: attempt.attempt_id.clone(),
        trigger: attempt.trigger,
        started_at: attempt.started_at,
    });

    for record in &attempt.stage_history {
        events.push(SubscriptionEvent::StageChanged {
            source_id: source_id.into(),
            attempt_id: attempt.attempt_id.clone(),
            stage: record.stage,
            changed_at: record.changed_at,
            transport: record.transport,
        });
    }

    events.push(SubscriptionEvent::UpdateFinished {
        source_id: source_id.into(),
        attempt_id: attempt.attempt_id.clone(),
        trigger: attempt.trigger,
        finished_at: attempt.finished_at,
        final_status: attempt.final_status,
        stage: attempt.stage,
        transport: attempt.transport,
        artifact_version: attempt.artifact_version.clone(),
        runtime_activated: attempt.runtime_activated,
        active_artifact_unchanged: attempt.active_artifact_unchanged,
        error: attempt
            .error
            .clone()
            .map(|message| UpdateErrorView { message }),
    });

    events
}

fn stage_changed_at(
    attempt: &SubscriptionUpdateAttempt,
    stage: UpdateStage,
    transport: Option<TransportKind>,
) -> i64 {
    attempt
        .stage_history
        .iter()
        .rev()
        .find(|record| record.stage == stage && record.transport == transport)
        .map(|record| record.changed_at)
        .unwrap_or_else(|| Local::now().timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::subscription::model::SubscriptionStageRecord;

    #[test]
    fn builds_event_timeline_from_finished_attempt_record() {
        let attempt = SubscriptionAttemptRecord {
            attempt_id: "attempt-a".into(),
            trigger: UpdateTrigger::Manual,
            started_at: 100,
            finished_at: 300,
            final_status: UpdateFinalStatus::Succeeded,
            stage: UpdateStage::EmitFinalResult,
            transport: Some(TransportKind::Direct),
            artifact_version: Some("artifact-a".into()),
            error: None,
            runtime_activated: true,
            active_artifact_unchanged: false,
            stage_history: vec![
                SubscriptionStageRecord {
                    stage: UpdateStage::FetchPayload,
                    changed_at: 200,
                    transport: Some(TransportKind::Direct),
                },
                SubscriptionStageRecord {
                    stage: UpdateStage::EmitFinalResult,
                    changed_at: 250,
                    transport: Some(TransportKind::Direct),
                },
            ],
        };

        let events = events_from_attempt_record("source-a", &attempt);

        assert_eq!(events.len(), 4);
        match &events[0] {
            SubscriptionEvent::AttemptStarted {
                source_id,
                attempt_id,
                started_at,
                ..
            } => {
                assert_eq!(source_id, "source-a");
                assert_eq!(attempt_id, "attempt-a");
                assert_eq!(*started_at, 100);
            }
            _ => panic!("expected attempt_started event"),
        }
        match &events[1] {
            SubscriptionEvent::StageChanged {
                stage, changed_at, ..
            } => {
                assert_eq!(*stage, UpdateStage::FetchPayload);
                assert_eq!(*changed_at, 200);
            }
            _ => panic!("expected stage_changed event"),
        }
        match &events[3] {
            SubscriptionEvent::UpdateFinished {
                final_status,
                finished_at,
                artifact_version,
                ..
            } => {
                assert_eq!(*final_status, UpdateFinalStatus::Succeeded);
                assert_eq!(*finished_at, 300);
                assert_eq!(artifact_version.as_deref(), Some("artifact-a"));
            }
            _ => panic!("expected update_finished event"),
        }
    }

    #[test]
    fn realtime_stage_event_uses_recorded_stage_timestamp() {
        let mut attempt =
            SubscriptionUpdateAttempt::new("source-a", UpdateTrigger::Automatic);
        attempt.record_stage_changed(UpdateStage::FetchPayload, Some(TransportKind::LocalProxy));
        let changed_at = attempt.stage_history[0].changed_at;

        let event = SubscriptionEvent::stage_changed(
            &attempt,
            UpdateStage::FetchPayload,
            Some(TransportKind::LocalProxy),
        );

        match event {
            SubscriptionEvent::StageChanged {
                changed_at: event_at,
                ..
            } => assert_eq!(event_at, changed_at),
            _ => panic!("expected stage_changed event"),
        }
    }
}
