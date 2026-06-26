use crate::{
    APP_HANDLE, singleton,
    subscription::{
        events::SubscriptionEvent,
        model::{SubscriptionUpdateAttempt, UpdateStage},
        transport::TransportKind,
    },
};
use smartstring::alias::String;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

use super::connection_metrics::ConnectionMetricsEventPayload;
use super::notification::{FrontendEvent, NotificationSystem};

#[derive(Debug)]
pub struct Handle {
    is_exiting: AtomicBool,
}

impl Default for Handle {
    fn default() -> Self {
        Self {
            is_exiting: AtomicBool::new(false),
        }
    }
}

singleton!(Handle, HANDLE);

impl Handle {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn app_handle() -> &'static AppHandle {
        #[allow(clippy::expect_used)]
        APP_HANDLE.get().expect("App handle not initialized")
    }

    pub fn refresh_clash() {
        Self::send_event(FrontendEvent::RefreshClash);
    }

    pub fn refresh_verge() {
        Self::send_event(FrontendEvent::RefreshVerge);
    }

    pub fn notify_profile_changed(profile_id: &String) {
        Self::send_event(FrontendEvent::ProfileChanged {
            current_profile_id: profile_id,
        });
    }

    pub fn notify_timer_updated(profile_index: &String) {
        Self::send_event(FrontendEvent::TimerUpdated { profile_index });
    }

    pub fn notice_message<S: AsRef<str>, M: Into<String>>(status: S, msg: M) {
        let status_str = status.as_ref();
        let msg_str = msg.into();

        Self::send_event(FrontendEvent::NoticeMessage {
            status: status_str,
            message: msg_str,
        });
    }

    pub fn notify_subscription_attempt_started(attempt: &SubscriptionUpdateAttempt) {
        Self::send_event(FrontendEvent::SubscriptionUpdate {
            event: SubscriptionEvent::attempt_started(attempt),
        });
    }

    pub fn notify_subscription_stage_changed(
        attempt: &SubscriptionUpdateAttempt,
        stage: UpdateStage,
        transport: Option<TransportKind>,
    ) {
        Self::send_event(FrontendEvent::SubscriptionUpdate {
            event: SubscriptionEvent::stage_changed(attempt, stage, transport),
        });
    }

    pub fn notify_subscription_update_succeeded(
        attempt: &SubscriptionUpdateAttempt,
        transport: TransportKind,
        stage: UpdateStage,
        artifact_version: String,
        runtime_activated: bool,
        active_artifact_unchanged: bool,
    ) {
        Self::send_event(FrontendEvent::SubscriptionUpdate {
            event: SubscriptionEvent::succeeded(
                attempt,
                transport,
                stage,
                artifact_version,
                runtime_activated,
                active_artifact_unchanged,
            ),
        });
    }

    pub fn notify_subscription_update_failed(
        attempt: &SubscriptionUpdateAttempt,
        stage: UpdateStage,
        transport: Option<TransportKind>,
        artifact_version: Option<String>,
        error: impl Into<String>,
        active_artifact_unchanged: bool,
    ) {
        Self::send_event(FrontendEvent::SubscriptionUpdate {
            event: SubscriptionEvent::failed(
                attempt,
                stage,
                transport,
                artifact_version,
                error,
                active_artifact_unchanged,
            ),
        });
    }

    pub fn set_is_exiting(&self) {
        self.is_exiting.store(true, Ordering::Release);
    }

    pub fn is_exiting(&self) -> bool {
        self.is_exiting.load(Ordering::Acquire)
    }

    pub fn send_connection_metrics(payload: ConnectionMetricsEventPayload) {
        Self::send_event(FrontendEvent::ConnectionMetrics { payload });
    }

    pub fn send_core_log(payload: serde_json::Value) {
        Self::send_event(FrontendEvent::CoreLog { payload });
    }

    fn send_event(event: FrontendEvent) {
        let handle = Self::global();
        if handle.is_exiting() {
            return;
        }

        NotificationSystem::send_event(event);
    }
}

#[cfg(target_os = "macos")]
impl Handle {
    pub fn set_activation_policy(&self, policy: tauri::ActivationPolicy) -> Result<(), String> {
        Self::app_handle()
            .set_activation_policy(policy)
            .map_err(|e| e.to_string().into())
    }

    pub fn set_activation_policy_regular(&self) {
        let _ = self.set_activation_policy(tauri::ActivationPolicy::Regular);
    }

    pub fn set_activation_policy_accessory(&self) {
        let _ = self.set_activation_policy(tauri::ActivationPolicy::Accessory);
    }
}
