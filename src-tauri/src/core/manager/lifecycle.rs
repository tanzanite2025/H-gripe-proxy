use super::outbound_select;
use super::tun_inbound::TunInbound;
use super::{CoreManager, RunningMode};
use crate::config::Config;
use crate::core::geo_update;
use crate::core::handle::Handle;
use crate::core::manager::CLASH_LOGGER;
use crate::core::provider_update;
use crate::core::rule_engine::{RuleProviderConfig, RuleSetData};
use crate::core::rule_geodata::RuleGeoData;
use crate::core::service::{SERVICE_MANAGER, ServiceStatus};
use anyhow::{Context as _, Result, anyhow};
use clash_verge_logging::{Type, logging};
use learn_gripe::{GeoLookup, GripeConfig, GripeKernel, OutboundMode, ProcessLookup, RuleSetLookup};
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

        // Obfuscation counters are process-global; clear them so the stats
        // track only the current kernel run.
        learn_gripe::reset_obfuscation_stats();

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

    /// Snapshot the in-process connection table from the running kernel.
    /// Returns `None` when the kernel is not running. Replaces the Mihomo
    /// controller `/connections` query.
    pub async fn runtime_connections(&self) -> Option<learn_gripe::ConnTableSnapshot> {
        self.gripe.lock().await.as_ref().map(|handle| handle.connections())
    }

    /// Snapshot the in-stack DNS answerer's counters. Returns `None` unless a
    /// TUN inbound is running, because the fake-IP answerer on the TUN datapath
    /// is the only resolver the Rust kernel answers itself — there is no honest
    /// DNS source outside TUN mode (queries are forwarded verbatim).
    pub async fn runtime_dns_stats(&self) -> Option<learn_gripe::DnsStatsSnapshot> {
        self.tun.lock().await.as_ref().map(|tun| tun.dns_stats())
    }

    /// Subscribe to the running kernel's connection-table change signal. Returns
    /// `None` when the kernel is not running. Drives the live-connections stream
    /// (push a fresh snapshot on every membership change) and lets the stream
    /// detect a stopped kernel (the receiver errors once the kernel is dropped).
    /// Replaces the Mihomo controller `/connections` WebSocket subscription.
    pub async fn watch_runtime_connections(&self) -> Option<tokio::sync::watch::Receiver<u64>> {
        self.gripe
            .lock()
            .await
            .as_ref()
            .map(|handle| handle.watch_connections())
    }

    /// Signal the kernel to close the connection with `id`. Returns `true` if it
    /// was live. Replaces the Mihomo controller `close_connection` call.
    pub async fn close_runtime_connection(&self, id: u64) -> bool {
        match self.gripe.lock().await.as_ref() {
            Some(handle) => handle.close_connection(id),
            None => false,
        }
    }

    /// Signal the kernel to close every live connection, returning the number
    /// signalled. Replaces iterating the Mihomo controller `close_connection`.
    pub async fn close_all_runtime_connections(&self) -> usize {
        match self.gripe.lock().await.as_ref() {
            Some(handle) => handle.close_all_connections(),
            None => 0,
        }
    }

    /// Snapshot the kernel's in-process client-obfuscation counters — outbound
    /// TLS ClientHello fingerprint shaping (the kernel's only client-side
    /// obfuscation). Returns `None` when the kernel is not running, so the
    /// bridge reports empty stats. Replaces the Mihomo controller
    /// `/engine/obfuscation/stats` query against the external Go kernel.
    pub async fn runtime_obfuscation_stats(&self) -> Option<learn_gripe::ObfuscationSnapshot> {
        self.gripe
            .lock()
            .await
            .as_ref()
            .map(|_| learn_gripe::snapshot_obfuscation_stats())
    }

    /// Reset the in-process obfuscation counters. The counters are
    /// process-global, so this succeeds whether or not the kernel is running.
    /// Replaces the Mihomo controller `reset_obfuscation_stats` call.
    pub async fn reset_runtime_obfuscation_stats(&self) {
        learn_gripe::reset_obfuscation_stats();
    }

    /// Record an operator-requested TLS fingerprint rotation and return the
    /// active fingerprint label. learn-gripe re-rolls `random` / `randomized`
    /// fingerprints per dial and pins concrete ones to per-proxy config, so a
    /// forced rotation has no on-the-wire effect; it is counted for telemetry
    /// parity with the former Mihomo controller `/engine/obfuscation/tls/rotate`
    /// call. The counter is process-global, so this succeeds whether or not the
    /// kernel is running.
    pub async fn force_runtime_tls_rotation(&self) -> String {
        learn_gripe::force_obfuscation_tls_rotation()
    }

    /// Measure the delay (RTT) of dialing `test_url` through the outbound for
    /// `proxy_name` — a `proxies:` node, or a proxy-group followed to its
    /// selected node — capped at `timeout` milliseconds. Returns the delay in
    /// milliseconds. Replaces the Mihomo controller `/proxies/{name}/delay`
    /// call: the probe dials the node's own outbound in-process and times the
    /// handshake plus a minimal HTTP request, so the figure includes the
    /// proxy's protocol/TLS setup exactly as a real connection pays it.
    ///
    /// Errors when no runtime config exists, the name has no usable outbound,
    /// or the probe fails (timeout / refused). The caller maps a failed probe
    /// to the UI's `delay == 0` timeout sentinel.
    pub async fn measure_runtime_proxy_delay(&self, proxy_name: &str, test_url: &str, timeout: u32) -> Result<u32> {
        let mode = Self::outbound_for_named(proxy_name).await?;
        learn_gripe::measure_delay(&mode, test_url, Duration::from_millis(u64::from(timeout)))
            .await
            .map_err(|err| anyhow!("delay probe for {proxy_name:?} failed: {err:#}"))
    }

    /// Measure the delay of every measurable member of `group_name`, probing
    /// them concurrently, and return `{ member_name -> delay_ms }`. Replaces
    /// the Mihomo controller `/group/{name}/delay` call. A member whose probe
    /// times out or fails is reported as `0` (the UI timeout sentinel) rather
    /// than dropped, so the UI still shows every node. Errors only when no
    /// runtime config exists or the group is missing/empty.
    pub async fn measure_runtime_group_delay(
        &self,
        group_name: &str,
        test_url: &str,
        timeout: u32,
    ) -> Result<HashMap<String, u32>> {
        let members = Self::group_member_outbounds_for(group_name).await?;
        let timeout = Duration::from_millis(u64::from(timeout));

        let mut probes = tokio::task::JoinSet::new();
        for (name, mode) in members {
            let test_url = test_url.to_string();
            probes.spawn(async move {
                let delay = learn_gripe::measure_delay(&mode, &test_url, timeout).await.unwrap_or(0);
                (name, delay)
            });
        }

        let mut delays = HashMap::new();
        while let Some(joined) = probes.join_next().await {
            if let Ok((name, delay)) = joined {
                delays.insert(name, delay);
            }
        }
        Ok(delays)
    }

    /// Resolve one policy name to the outbound a delay probe should dial, using
    /// the current runtime config plus persisted selection.
    async fn outbound_for_named(name: &str) -> Result<OutboundMode> {
        let runtime = Config::runtime().await.latest_arc();
        let config = runtime
            .config
            .as_ref()
            .ok_or_else(|| anyhow!("no runtime config available for delay test"))?;
        let selection = Self::current_group_selection().await;
        outbound_select::outbound_for_proxy(config, &selection, name)
    }

    /// Resolve every measurable member of a proxy-group to `(name, outbound)`
    /// pairs, using the current runtime config plus persisted selection.
    async fn group_member_outbounds_for(group_name: &str) -> Result<Vec<(String, OutboundMode)>> {
        let runtime = Config::runtime().await.latest_arc();
        let config = runtime
            .config
            .as_ref()
            .ok_or_else(|| anyhow!("no runtime config available for delay test"))?;
        let selection = Self::current_group_selection().await;
        outbound_select::group_member_outbounds(config, &selection, group_name)
    }

    pub async fn restart_core(&self) -> Result<()> {
        logging!(info, Type::Core, "Restarting core");
        self.stop_core().await?;

        tokio::time::sleep(Duration::from_millis(350)).await;

        self.start_core().await
    }

    /// Update the local GeoIP/GeoSite/ASN databases in process.
    ///
    /// Downloads the upstream files (honouring any custom `geox-url` source),
    /// validates them, and atomically replaces the local copies. When the
    /// kernel is running it is restarted so the router reloads the refreshed
    /// geo data through [`GeoLookup`] — the same boundary config changes use.
    pub async fn update_geo(&self) -> Result<()> {
        let updated = geo_update::update_geo_files().await?;
        logging!(info, Type::Core, "Updated geo databases: {}", updated.join(", "));

        if matches!(self.get_running_mode().as_ref(), RunningMode::Gripe) {
            self.restart_core()
                .await
                .context("failed to reload kernel after geo update")?;
        }
        Ok(())
    }

    /// Refresh a proxy provider's local node list in process, then reload the
    /// kernel so the new nodes take effect. Replaces the Mihomo controller
    /// `/providers/proxies/{name}` update call: an HTTP provider is downloaded,
    /// validated, and atomically swapped in; file/inline providers are no-ops.
    pub async fn update_proxy_provider(&self, name: &str) -> Result<()> {
        provider_update::update_proxy_provider(name).await?;
        self.reload_after_provider_update().await
    }

    /// Refresh a rule provider's local file in process and reload the kernel so
    /// the rule engine re-parses it. Replaces the Mihomo controller
    /// `/providers/rules/{name}` update call.
    pub async fn update_rule_provider(&self, name: &str) -> Result<()> {
        provider_update::update_rule_provider(name).await?;
        self.reload_after_provider_update().await
    }

    /// Probe every measurable node of a proxy provider in process and persist
    /// the per-node delays. Replaces the Mihomo controller
    /// `/providers/proxies/{name}/healthcheck` call; no reload is needed since
    /// the snapshot reads the recorded delays directly.
    pub async fn healthcheck_proxy_provider(&self, name: &str) -> Result<()> {
        let probed = provider_update::healthcheck_proxy_provider(name).await?;
        logging!(
            info,
            Type::Core,
            "Health-checked {probed} node(s) in proxy provider {name:?}"
        );
        Ok(())
    }

    async fn reload_after_provider_update(&self) -> Result<()> {
        if matches!(self.get_running_mode().as_ref(), RunningMode::Gripe) {
            self.restart_core()
                .await
                .context("failed to reload kernel after provider update")?;
        }
        Ok(())
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
        let process = Self::load_process_lookup();
        Self::resolve_with(move |config, selection| {
            let rule_sets = Self::load_rule_sets(config);
            outbound_select::routed_outbound(config, selection, geo.clone(), rule_sets, process.clone())
        })
        .await
    }

    /// Build the OS-level process lookup so the rule router can evaluate
    /// `PROCESS-NAME` / `PROCESS-PATH` rules. The kernel never performs the
    /// socket→process resolution itself — it only queries the owning local
    /// process of a connection's source socket through [`ProcessLookup`].
    /// There is no config to load; the lookup is always available and simply
    /// resolves nothing (so those rules never match) on platforms or
    /// connections where the owning process cannot be determined.
    fn load_process_lookup() -> Option<Arc<dyn ProcessLookup>> {
        Some(Arc::new(crate::core::process_lookup::ProcessData) as Arc<dyn ProcessLookup>)
    }

    /// Build the locally-loaded rule-set providers (`rule-providers:`) from the
    /// runtime config so the rule router can evaluate `RULE-SET` rules. The
    /// kernel never fetches or owns this data — it only queries it through
    /// [`RuleSetLookup`]. When the config declares no providers, or none can be
    /// loaded, the lookup is absent and `RULE-SET` rules are simply skipped.
    fn load_rule_sets(config: &serde_yaml_ng::Mapping) -> Option<Arc<dyn RuleSetLookup>> {
        let providers = config.get("rule-providers")?.clone();
        let providers: HashMap<String, RuleProviderConfig> = match serde_yaml_ng::from_value(providers) {
            Ok(providers) => providers,
            Err(err) => {
                logging!(warn, Type::Core, "failed to parse rule-providers: {err:#}");
                return None;
            }
        };
        if providers.is_empty() {
            return None;
        }
        match RuleSetData::from_rule_providers(providers) {
            Ok(data) => Some(Arc::new(data) as Arc<dyn RuleSetLookup>),
            Err(err) => {
                logging!(warn, Type::Core, "failed to load rule providers: {err:#}");
                None
            }
        }
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
        self.wait_for_service_if_needed().await;

        self.enforce_tun_fail_closed_if_needed().await?;

        self.set_running_mode(RunningMode::NotRunning);
        Ok(())
    }

    fn after_core_process(&self) {
        let app_handle = Handle::app_handle();
        tauri_plugin_clash_verge_sysinfo::set_app_core_mode(app_handle, self.get_running_mode().to_string());
    }

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
        OutboundMode::Http(_) => "http",
        OutboundMode::Ssh(_) => "ssh",
        OutboundMode::Hysteria(_) => "hysteria",
        OutboundMode::GostRelay(_) => "gost-relay",
        OutboundMode::Mieru(_) => "mieru",
        OutboundMode::Vless(_) => "vless",
        OutboundMode::Trojan(_) => "trojan",
        OutboundMode::Vmess(_) => "vmess",
        OutboundMode::Shadowsocks(_) => "shadowsocks",
        OutboundMode::Tuic(_) => "tuic",
        OutboundMode::Hysteria2(_) => "hysteria2",
        OutboundMode::Masque(_) => "masque",
        OutboundMode::AnyTls(_) => "anytls",
        OutboundMode::Snell(_) => "snell",
        OutboundMode::Ssr(_) => "ssr",
        OutboundMode::WireGuard(_) => "wireguard",
        OutboundMode::Routed(_) => "routed",
    }
}
