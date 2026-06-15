use crate::{
    config::{PrfItem, PrfOption},
    core::mihomo_runtime_guard,
    subscription::{
        artifact::build_clash_yaml_artifact_candidate,
        control_plane::{
            fetch_subscription_update_via_control_plane,
            subscription_update_uses_dedicated_control_plane,
        },
        fetch::fetch_remote_profile,
        model::{
            SubscriptionArtifactRecord, SubscriptionUpdateAttempt, UpdateStage,
            UpdateTrigger,
        },
        persist::persist_artifact_candidate,
        transport::{
            TransportKind, TransportPlan, apply_transport_to_option,
            transport_kind_from_option,
        },
    },
    utils::help::mask_err,
};
use anyhow::anyhow;
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;

#[derive(Debug, Clone)]
pub struct SubscriptionUpdateExecutor {
    source_id: String,
    url: String,
    source_option: Option<PrfOption>,
    request_option: Option<PrfOption>,
    trigger: UpdateTrigger,
}

#[derive(Debug, Clone)]
pub struct SubscriptionUpdateExecution {
    pub attempt: SubscriptionUpdateAttempt,
    pub transport: TransportKind,
    pub artifact: SubscriptionArtifactRecord,
    pub legacy_profile_item: PrfItem,
}

#[derive(Debug, Clone)]
pub struct SubscriptionUpdateFailure {
    pub attempt: SubscriptionUpdateAttempt,
    pub stage: UpdateStage,
    pub transport: Option<TransportKind>,
    pub artifact: Option<SubscriptionArtifactRecord>,
    pub error: String,
}

#[derive(Debug)]
struct TransportAttemptFailure {
    stage: UpdateStage,
    transport: Option<TransportKind>,
    artifact: Option<SubscriptionArtifactRecord>,
    error: anyhow::Error,
}

impl SubscriptionUpdateExecutor {
    pub fn new(
        source_id: impl Into<String>,
        url: impl Into<String>,
        source_option: Option<PrfOption>,
        request_option: Option<PrfOption>,
        trigger: UpdateTrigger,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            url: url.into(),
            source_option,
            request_option,
            trigger,
        }
    }

    pub async fn execute<OnAttemptStarted, OnStageChanged>(
        self,
        mut on_attempt_started: OnAttemptStarted,
        mut on_stage_changed: OnStageChanged,
    ) -> std::result::Result<SubscriptionUpdateExecution, SubscriptionUpdateFailure>
    where
        OnAttemptStarted: FnMut(&SubscriptionUpdateAttempt),
        OnStageChanged: FnMut(&SubscriptionUpdateAttempt, UpdateStage, Option<TransportKind>),
    {
        logging!(
            info,
            Type::Config,
            "[Subscription Update] start downloading remote subscription"
        );

        let mut attempt = SubscriptionUpdateAttempt::new(self.source_id.clone(), self.trigger);
        on_attempt_started(&attempt);
        record_stage(
            &mut attempt,
            UpdateStage::ResolveSource,
            None,
            &mut on_stage_changed,
        );

        let merged_option =
            PrfOption::merge(self.source_option.as_ref(), self.request_option.as_ref());
        let persisted_option = self.source_option.clone();
        record_stage(
            &mut attempt,
            UpdateStage::ResolveTransportPlan,
            None,
            &mut on_stage_changed,
        );

        let transport_plan =
            TransportPlan::for_subscription_update(Some(transport_kind_from_option(
                merged_option.as_ref(),
            )))
            .await;
        if let Some(note) = transport_plan.note.as_ref() {
            logging!(
                info,
                Type::Config,
                "[Subscription Update] transport plan note: {}",
                note
            );
        }

        let use_dedicated_control_plane = subscription_update_uses_dedicated_control_plane().await;
        let transport_plan_note = transport_plan.note.clone();
        let mut last_failure = None;

        for candidate in transport_plan.ordered_candidates {
            let transport = candidate.kind;
            let attempt_option = apply_transport_to_option(merged_option.as_ref(), transport);

            if matches!(transport, TransportKind::LocalProxy)
                && let Err(err) = mihomo_runtime_guard::ensure_mihomo_core_ready().await
            {
                logging!(
                    warn,
                    Type::Config,
                    "Warning: [Subscription Update] {} skipped because Mihomo core is not ready: {}",
                    transport.label(),
                    format_subscription_update_error(&err)
                );
                log_subscription_update_error("ensure mihomo core ready", &err);
                last_failure = Some(TransportAttemptFailure {
                    stage: UpdateStage::FetchPayload,
                    transport: Some(transport),
                    artifact: None,
                    error: err,
                });
                continue;
            }

            record_stage(
                &mut attempt,
                UpdateStage::FetchPayload,
                Some(transport),
                &mut on_stage_changed,
            );

            let fetched = match if use_dedicated_control_plane
                && matches!(transport, TransportKind::LocalProxy)
            {
                fetch_subscription_update_via_control_plane(self.url.as_str(), &attempt_option)
                    .await
            } else {
                fetch_remote_profile(self.url.as_str(), Some(&attempt_option)).await
            } {
                Ok(fetched) => fetched,
                Err(err) => {
                    logging!(
                        warn,
                        Type::Config,
                        "Warning: [Subscription Update] {} failed: {}",
                        transport.label(),
                        format_subscription_update_error(&err)
                    );
                    log_subscription_update_error(transport.label(), &err);
                    last_failure = Some(TransportAttemptFailure {
                        stage: UpdateStage::FetchPayload,
                        transport: Some(transport),
                        artifact: None,
                        error: err,
                    });
                    continue;
                }
            };

            record_stage(
                &mut attempt,
                UpdateStage::DecodePayload,
                Some(transport),
                &mut on_stage_changed,
            );

            let artifact_candidate = match build_clash_yaml_artifact_candidate(
                &fetched,
                chrono::Local::now().timestamp_millis(),
            ) {
                Ok(candidate) => candidate,
                Err(err) => {
                    logging!(
                        warn,
                        Type::Config,
                        "Warning: [Subscription Update] {} returned an unsupported payload format: {}",
                        transport.label(),
                        format_subscription_update_error(&err)
                    );
                    log_subscription_update_error("decode payload", &err);
                    last_failure = Some(TransportAttemptFailure {
                        stage: UpdateStage::DecodePayload,
                        transport: Some(transport),
                        artifact: None,
                        error: err,
                    });
                    continue;
                }
            };

            record_stage(
                &mut attempt,
                UpdateStage::MaterializeArtifact,
                Some(transport),
                &mut on_stage_changed,
            );

            let artifact = artifact_candidate.record.clone();
            if let Err(err) =
                persist_artifact_candidate(self.source_id.as_str(), &artifact_candidate).await
            {
                return Err(SubscriptionUpdateFailure {
                    attempt,
                    stage: UpdateStage::MaterializeArtifact,
                    transport: Some(transport),
                    artifact: Some(artifact),
                    error: format!("failed to persist subscription artifact: {err}").into(),
                });
            }

            let legacy_profile_item = match PrfItem::from_fetched_payload(
                self.url.as_str(),
                fetched,
                None,
                None,
                persisted_option.as_ref(),
            )
            .await
            {
                Ok(item) => item,
                Err(err) => {
                    logging!(
                        warn,
                        Type::Config,
                        "Warning: [Subscription Update] {} returned an invalid payload: {}",
                        transport.label(),
                        format_subscription_update_error(&err)
                    );
                    log_subscription_update_error("materialize artifact", &err);
                    last_failure = Some(TransportAttemptFailure {
                        stage: UpdateStage::MaterializeArtifact,
                        transport: Some(transport),
                        artifact: Some(artifact),
                        error: err,
                    });
                    continue;
                }
            };

            logging!(
                info,
                Type::Config,
                "[Subscription Update] subscription fetch succeeded via {}",
                transport.label()
            );
            return Ok(SubscriptionUpdateExecution {
                attempt,
                transport,
                artifact,
                legacy_profile_item,
            });
        }

        Err(finish_transport_plan_failure(
            attempt,
            last_failure,
            transport_plan_note.as_ref(),
        ))
    }
}

fn record_stage<OnStageChanged>(
    attempt: &mut SubscriptionUpdateAttempt,
    stage: UpdateStage,
    transport: Option<TransportKind>,
    on_stage_changed: &mut OnStageChanged,
) where
    OnStageChanged: FnMut(&SubscriptionUpdateAttempt, UpdateStage, Option<TransportKind>),
{
    attempt.record_stage_changed(stage, transport);
    on_stage_changed(attempt, stage, transport);
}

fn finish_transport_plan_failure(
    attempt: SubscriptionUpdateAttempt,
    last_failure: Option<TransportAttemptFailure>,
    transport_plan_note: Option<&String>,
) -> SubscriptionUpdateFailure {
    let failure = last_failure.unwrap_or_else(|| TransportAttemptFailure {
        stage: UpdateStage::FetchPayload,
        transport: None,
        artifact: None,
        error: anyhow!("subscription update transport plan produced no attempts"),
    });
    SubscriptionUpdateFailure {
        attempt,
        stage: failure.stage,
        transport: failure.transport,
        artifact: failure.artifact,
        error: append_subscription_update_note(
            format!(
                "failed to update profile after all transport attempts: {}",
                format_subscription_update_error(&failure.error)
            ),
            transport_plan_note,
        ),
    }
}

pub fn format_subscription_update_error(err: &anyhow::Error) -> String {
    mask_err(&format!("{err:#}"))
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(": ")
        .into()
}

pub fn append_subscription_update_note(
    message: impl Into<std::string::String>,
    note: Option<&String>,
) -> String {
    let mut message = message.into();

    if let Some(note) = note.map(|note| note.trim()).filter(|note| !note.is_empty()) {
        message.push_str(" Note: ");
        message.push_str(note);
    }

    message.into()
}

fn log_subscription_update_error(stage: &str, err: &anyhow::Error) {
    logging!(
        debug,
        Type::Config,
        "[Subscription Update] {} detailed error chain: {}",
        stage,
        mask_err(&format!("{err:#}"))
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_stage_updates_attempt_and_notifies_observer() {
        let mut attempt = SubscriptionUpdateAttempt::new("source-a", UpdateTrigger::Manual);
        let mut observed = Vec::new();

        record_stage(
            &mut attempt,
            UpdateStage::FetchPayload,
            Some(TransportKind::Direct),
            &mut |_, stage, transport| observed.push((stage, transport)),
        );

        assert_eq!(attempt.stage_history.len(), 1);
        assert_eq!(attempt.stage_history[0].stage, UpdateStage::FetchPayload);
        assert_eq!(
            observed,
            vec![(UpdateStage::FetchPayload, Some(TransportKind::Direct))]
        );
    }

    #[test]
    fn finish_transport_plan_failure_uses_last_failure_with_note() {
        let attempt = SubscriptionUpdateAttempt::new("source-a", UpdateTrigger::Automatic);
        let note = String::from("dedicated control-plane route was selected");
        let failure = finish_transport_plan_failure(
            attempt,
            Some(TransportAttemptFailure {
                stage: UpdateStage::DecodePayload,
                transport: Some(TransportKind::LocalProxy),
                artifact: None,
                error: anyhow!("unsupported payload"),
            }),
            Some(&note),
        );

        assert_eq!(failure.stage, UpdateStage::DecodePayload);
        assert_eq!(failure.transport, Some(TransportKind::LocalProxy));
        assert!(failure.error.contains("unsupported payload"));
        assert!(failure.error.contains("dedicated control-plane route"));
    }

    #[test]
    fn finish_transport_plan_failure_handles_empty_plan() {
        let attempt = SubscriptionUpdateAttempt::new("source-a", UpdateTrigger::Automatic);
        let failure = finish_transport_plan_failure(attempt, None, None);

        assert_eq!(failure.stage, UpdateStage::FetchPayload);
        assert!(failure.transport.is_none());
        assert!(
            failure
                .error
                .contains("subscription update transport plan produced no attempts")
        );
    }
}
