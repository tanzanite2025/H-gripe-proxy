use super::{CoreManager, RunningMode};
use crate::config::Config;
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use crate::core::service::{SERVICE_MANAGER, ServiceStatus};
use anyhow::{Result, anyhow};
use clash_verge_logging::{Type, logging};
use scopeguard::defer;
use std::time::Duration;
use tauri_plugin_clash_verge_sysinfo;

impl CoreManager {
    pub async fn start_core(&self) -> Result<()> {
        self.prepare_startup().await?;
        defer! {
            self.after_core_process();
        }

        match *self.get_running_mode() {
            RunningMode::Service => self.start_core_by_service().await,
            RunningMode::Sidecar => self.start_core_by_sidecar().await,
            RunningMode::NotRunning => Err(anyhow!(
                "Mihomo sidecar startup is retired and no service/Rust runtime is ready"
            )),
        }
    }

    pub async fn stop_core(&self) -> Result<()> {
        CLASH_LOGGER.clear_logs().await;
        defer! {
            self.after_core_process();
        }

        match *self.get_running_mode() {
            RunningMode::Service => self.stop_core_by_service().await,
            RunningMode::Sidecar => {
                self.stop_core_by_sidecar();
                Ok(())
            }
            RunningMode::NotRunning => Ok(()),
        }
    }

    pub async fn restart_core(&self) -> Result<()> {
        logging!(info, Type::Core, "Restarting core");
        self.stop_core().await?;

        #[cfg(target_os = "windows")]
        tokio::time::sleep(Duration::from_millis(350)).await;

        self.start_core().await
    }

    async fn prepare_startup(&self) -> Result<()> {
        #[cfg(target_os = "windows")]
        self.wait_for_service_if_needed().await;

        #[cfg(target_os = "windows")]
        self.enforce_tun_fail_closed_if_needed().await?;

        #[cfg(target_os = "windows")]
        let tun_enabled = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);
        let value = SERVICE_MANAGER.lock().await.current();
        let mode = match value {
            #[cfg(target_os = "windows")]
            ServiceStatus::Ready if tun_enabled => RunningMode::NotRunning,
            ServiceStatus::Ready => RunningMode::NotRunning,
            _ => RunningMode::NotRunning,
        };

        self.set_running_mode(mode);
        Ok(())
    }

    fn after_core_process(&self) {
        let app_handle = Handle::app_handle();
        tauri_plugin_clash_verge_sysinfo::set_app_core_mode(app_handle, self.get_running_mode().to_string());
    }

    #[cfg(target_os = "windows")]
    async fn enforce_tun_fail_closed_if_needed(&self) -> Result<()> {
        use tauri_plugin_clash_verge_sysinfo::is_current_app_handle_admin;

        let tun_enabled = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);

        if !tun_enabled || is_current_app_handle_admin(Handle::app_handle()) {
            return Ok(());
        }

        let service_ready = matches!(SERVICE_MANAGER.lock().await.current(), ServiceStatus::Ready);

        if service_ready {
            let message = "TUN protection unavailable: Mihomo service core startup is retired. Use the Rust runtime startup path.";
            logging!(warn, Type::Core, "{}", message);
            self.set_running_mode(RunningMode::NotRunning);
            Handle::notice_message("update_failed", message);
            return Err(anyhow!(message));
        }

        let message = "TUN protection unavailable: the privileged service is not ready. Core start blocked to avoid traffic leaks. Repair the service or run as administrator.";
        logging!(warn, Type::Core, "{}", message);
        self.set_running_mode(RunningMode::NotRunning);
        Handle::notice_message("update_failed", message);
        Err(anyhow!(message))
    }

    #[cfg(target_os = "windows")]
    async fn wait_for_service_if_needed(&self) {
        use crate::{config::Config, core::service};
        use backon::{ConstantBuilder, Retryable as _};

        let needs_service = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);

        if !needs_service {
            return;
        }

        let service_config = service::ServiceManager::config();
        let backoff = ConstantBuilder::default()
            .with_delay(service_config.retry_delay)
            .with_max_times(service_config.max_retries);

        let _ = (|| async {
            let mut manager = SERVICE_MANAGER.lock().await;

            if matches!(manager.current(), ServiceStatus::Ready) {
                return Ok(());
            }

            // If the service IPC path is not ready yet, treat it as transient and retry.
            // Running init/refresh too early can mark service state unavailable and break later config reloads.
            if !service::is_service_ipc_path_exists() {
                return Err(anyhow::anyhow!("Service IPC not ready"));
            }

            manager.init().await?;
            let _ = manager.refresh().await;

            if matches!(manager.current(), ServiceStatus::Ready) {
                Ok(())
            } else {
                Err(anyhow::anyhow!("Service not ready"))
            }
        })
        .retry(backoff)
        .await;
    }
}
