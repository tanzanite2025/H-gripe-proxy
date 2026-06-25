use super::outbound_select;
use super::tun_inbound::TunInbound;
use super::{CoreManager, RunningMode};
use crate::config::Config;
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use crate::core::rule_geodata::RuleGeoData;
use crate::core::service::{SERVICE_MANAGER, ServiceStatus};
use anyhow::{Result, anyhow};
use clash_verge_logging::{Type, logging};
use learn_gripe::{GeoLookup, GripeConfig, GripeKernel, OutboundMode};
use scopeguard::defer;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;
use tauri_plugin_clash_verge_sysinfo;

impl CoreManager {
    pub async fn start_core(&self) -> Result<()> {
        self.prepare_startup().await?;
        defer! {
            self.after_core_process();
        }

        let listen_port = Self::mixed_listen_port().await;
        let outbound = Self::resolve_outbound().await;
        logging!(
            info,
            Type::Core,
            "learn-gripe outbound resolved to {}",
            outbound_label(&outbound)
        );
        let config = GripeConfig {
            socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, listen_port)),
            outbound,
        };

        let handle = GripeKernel::start(config)
            .await
            .map_err(|err| anyhow!("failed to start learn-gripe kernel: {err:#}"))?;

        logging!(
            info,
            Type::Core,
            "learn-gripe kernel started on {}",
            handle.local_addr()
        );
        *self.gripe.lock().await = Some(handle);
        self.set_running_mode(RunningMode::Gripe);

        self.start_tun_if_enabled().await;
        Ok(())
    }

    /// Start the OS TUN inbound when `enable_tun_mode` is set. Off by default, so
    /// this is a no-op for the normal path. A failure to bind the device is
    /// logged but does not fail core startup — the mixed inbound stays up.
    ///
    /// The TUN device uses the *single global egress* ([`resolve_tun_outbound`])
    /// rather than the mixed inbound's rule router: a global default-route
    /// capture is only sound for a single fixed-server proxy
    /// (`OutboundMode::supports_global_capture`), and per-flow routing through
    /// the TUN (with a `Direct` bypass) is tracked as later TUN work.
    async fn start_tun_if_enabled(&self) {
        let tun_enabled = Config::verge().await.latest_arc().enable_tun_mode.unwrap_or(false);
        if !tun_enabled {
            return;
        }

        let outbound = Self::resolve_tun_outbound().await;
        match TunInbound::start(outbound).await {
            Ok(tun) => {
                *self.tun.lock().await = Some(tun);
            }
            Err(err) => {
                logging!(
                    warn,
                    Type::Core,
                    "TUN mode enabled but the OS TUN device could not be started: {err:#}"
                );
            }
        }
    }

    pub async fn stop_core(&self) -> Result<()> {
        CLASH_LOGGER.clear_logs().await;
        defer! {
            self.after_core_process();
        }

        if let Some(tun) = self.tun.lock().await.take() {
            tun.stop().await;
        }

        if let Some(handle) = self.gripe.lock().await.take() {
            handle.shutdown().await;
            logging!(info, Type::Core, "learn-gripe kernel stopped");
        }
        self.set_running_mode(RunningMode::NotRunning);
        Ok(())
    }

    pub async fn restart_core(&self) -> Result<()> {
        logging!(info, Type::Core, "Restarting core");
        self.stop_core().await?;

        #[cfg(target_os = "windows")]
        tokio::time::sleep(Duration::from_millis(350)).await;

        self.start_core().await
    }

    /// TCP port the kernel's mixed inbound binds on. This is the same port the
    /// OS system proxy and the PAC script target — `verge_mixed_port`, falling
    /// back to the clash `mixed-port` — so enabling the system proxy actually
    /// routes traffic through learn-gripe instead of a dead port.
    async fn mixed_listen_port() -> u16 {
        match Config::verge().await.latest_arc().verge_mixed_port {
            Some(port) => port,
            None => Config::clash().await.latest_arc().get_mixed_port(),
        }
    }

    /// Resolve the outbound for the mixed inbound from the generated runtime
    /// config plus the persisted per-group selection. In `rule` mode this is a
    /// per-connection rule [`OutboundMode::Routed`]; otherwise it is the single
    /// global egress. Falls back to [`OutboundMode::Direct`] when the runtime
    /// config is missing.
    async fn resolve_outbound() -> OutboundMode {
        let geo = Self::load_geo_lookup();
        Self::resolve_with(move |config, selection| outbound_select::routed_outbound(config, selection, geo.clone()))
            .await
    }

    /// Load the *local*, user-maintained geo database (Country.mmdb / GeoIP.dat
    /// / GeoSite.dat from the app home + resources dirs) so the rule router can
    /// evaluate `GEOIP` / `GEOSITE` rules. The kernel never fetches or owns this
    /// data — it only queries it through [`GeoLookup`]. When no files are
    /// present the lookups simply never match, so those rules are skipped.
    fn load_geo_lookup() -> Option<Arc<dyn GeoLookup>> {
        let geo: Arc<dyn GeoLookup> = Arc::new(RuleGeoData::load_default());
        Some(geo)
    }

    /// Resolve the *single global egress* for the TUN device (see
    /// [`start_tun_if_enabled`] for why TUN does not use the rule router).
    async fn resolve_tun_outbound() -> OutboundMode {
        Self::resolve_with(outbound_select::selected_outbound).await
    }

    /// Run `resolve` against the current runtime config + persisted selection,
    /// falling back to [`OutboundMode::Direct`] when no runtime config exists.
    async fn resolve_with(
        resolve: impl Fn(&serde_yaml_ng::Mapping, &HashMap<String, String>) -> OutboundMode,
    ) -> OutboundMode {
        let runtime = Config::runtime().await.latest_arc();
        let Some(config) = runtime.config.as_ref() else {
            logging!(
                info,
                Type::Core,
                "no runtime config yet; learn-gripe uses Direct outbound"
            );
            return OutboundMode::Direct;
        };
        let selection = Self::current_group_selection().await;
        resolve(config, &selection)
    }

    /// The persisted `{ group -> selected node }` map for the current profile.
    async fn current_group_selection() -> HashMap<String, String> {
        let profiles = Config::profiles().await.latest_arc();
        let Some(uid) = profiles.current_primary_uid() else {
            return HashMap::new();
        };
        let Ok(item) = profiles.get_item(&uid) else {
            return HashMap::new();
        };
        item.selected
            .as_ref()
            .map(|selected| {
                selected
                    .iter()
                    .filter_map(|s| Some((s.name.as_deref()?.to_string(), s.now.as_deref()?.to_string())))
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn prepare_startup(&self) -> Result<()> {
        #[cfg(target_os = "windows")]
        self.wait_for_service_if_needed().await;

        #[cfg(target_os = "windows")]
        self.enforce_tun_fail_closed_if_needed().await?;

        self.set_running_mode(RunningMode::NotRunning);
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

/// Short human-readable label for a resolved outbound, for startup logs.
fn outbound_label(outbound: &OutboundMode) -> &'static str {
    match outbound {
        OutboundMode::Direct => "direct",
        OutboundMode::Reject => "reject",
        OutboundMode::Socks5Upstream { .. } => "socks5",
        OutboundMode::Vless(_) => "vless",
        OutboundMode::Trojan(_) => "trojan",
        OutboundMode::Vmess(_) => "vmess",
        OutboundMode::Shadowsocks(_) => "shadowsocks",
        OutboundMode::Routed(_) => "routed",
    }
}
