use crate::{
    APP_HANDLE,
    config::{Config, IClashTemp},
    singleton,
    subscription::{
        events::SubscriptionEvent,
        model::{SubscriptionUpdateAttempt, UpdateStage},
        transport::TransportKind,
    },
};
use anyhow::{Result, anyhow};
use smartstring::alias::String;
use std::{
    net::SocketAddr,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::AppHandle;
use tauri_plugin_mihomo::{Mihomo, MihomoExt as _, models::Protocol};
use tokio::sync::RwLockReadGuard;

use super::connection_metrics::ConnectionMetricsSnapshot;
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

    pub async fn mihomo() -> RwLockReadGuard<'static, Mihomo> {
        Self::app_handle().mihomo().read().await
    }

    pub async fn sync_mihomo_controller_state() -> Result<()> {
        let client_info = Config::clash().await.latest_arc().get_client_info();
        #[cfg(target_os = "windows")]
        let http_controller_enabled = {
            let verge = Config::verge().await;
            let verge_arc = verge.latest_arc();
            let mut enabled = verge_arc.enable_external_controller.unwrap_or(false);
            enabled |= verge_arc.enable_tun_mode.unwrap_or(false);
            drop(verge_arc);
            drop(verge);
            enabled
        };
        let controller = client_info
            .server
            .parse::<SocketAddr>()
            .map_err(|err| anyhow!("invalid external controller '{}': {err}", client_info.server))?;
        let socket_path = IClashTemp::guard_external_controller_ipc();
        let host = controller.ip().to_string();
        let port = controller.port();
        let secret = client_info.secret.clone();

        let mut mihomo = Self::app_handle().mihomo().write().await;

        #[cfg(target_os = "windows")]
        {
            if !matches!(mihomo.protocol, Protocol::LocalSocket) {
                mihomo.update_protocol(Protocol::LocalSocket);
            }
        }

        #[cfg(not(target_os = "windows"))]
        if !matches!(mihomo.protocol, Protocol::LocalSocket) {
            mihomo.update_protocol(Protocol::LocalSocket);
        }

        #[cfg(target_os = "windows")]
        {
            if http_controller_enabled {
                if mihomo.external_host.as_deref() != Some(host.as_str()) {
                    mihomo.update_external_host(Some(host.clone()));
                }

                if mihomo.external_port != Some(port) {
                    mihomo.update_external_port(Some(port));
                }
            } else {
                if mihomo.external_host.is_some() {
                    mihomo.update_external_host(None);
                }

                if mihomo.external_port.is_some() {
                    mihomo.update_external_port(None);
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            if mihomo.external_host.as_deref() != Some(host.as_str()) {
                mihomo.update_external_host(Some(host));
            }

            if mihomo.external_port != Some(port) {
                mihomo.update_external_port(Some(port));
            }
        }

        if mihomo.secret != secret {
            mihomo.update_secret(secret);
        }

        if mihomo.socket_path.as_deref() != Some(socket_path.as_str()) {
            mihomo.update_socket_path(socket_path)?;
        }

        Ok(())
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

    pub fn send_connection_metrics(snapshot: ConnectionMetricsSnapshot) {
        Self::send_event(FrontendEvent::ConnectionMetrics { snapshot });
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
