use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::RwLock,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    config::Config,
    core::{CoreManager, manager::RunningMode},
};
use anyhow::Result;
use clash_dtos::{
    BaseConfig, BufferPoolStats, ClashMode, Connection, ConnectionMetaData, ConnectionType, Connections, DNSMode,
    DelayHistory, DnsCacheStats, DnsMetrics, DnsPollutionStats, DnsQueryEvent, DnsQueryStats, DnsServerClassification,
    DnsServerStats, DnsTrustSummary, EgressStatus, EngineStats, Extra, FindProcessMode, HotReloadStatus, LogLevel,
    MihomoVersion, Network, PerfStats, ProviderType, Proxies, Proxy, ProxyProvider, ProxyProviders, ProxyType, Rule,
    RuleBehavior, RuleFormat, RuleProvider, RuleProviders, RuleTrafficSnapshot, RuleType, Rules, SubScriptionInfo,
    TLSFingerprintStats, TunConfig, TunStack, VehicleType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::Value;

#[derive(Debug, Default)]
pub struct RuntimeSnapshot {
    pub core_running: bool,
    pub version: Option<MihomoVersion>,
    pub base_config: Option<BaseConfig>,
    pub proxies: Option<Proxies>,
    pub dns_metrics: Option<DnsMetrics>,
    pub engine_stats: Option<EngineStats>,
    pub perf_stats: Option<PerfStats>,
    pub buffer_pool_stats: Option<BufferPoolStats>,
    pub hot_reload_status: Option<HotReloadStatus>,
    pub rule_traffic: Option<HashMap<std::string::String, RuleTrafficSnapshot>>,
    pub tls_fingerprint_stats: Option<TLSFingerprintStats>,
    pub connections: Option<Connections>,
    pub rules: Option<Rules>,
    pub egress_status: Option<EgressStatus>,
    pub proxies_from_runtime_config: bool,
}

impl RuntimeSnapshot {
    pub fn stable_group_selected_nodes(&self) -> HashMap<String, String> {
        self.proxies
            .as_ref()
            .map(|proxies| {
                proxies
                    .proxies
                    .iter()
                    .filter_map(|(group_name, group_data)| {
                        if !group_name.starts_with("VERGE-STABLE-") {
                            return None;
                        }

                        group_data
                            .now
                            .as_ref()
                            .map(|value| value.trim())
                            .filter(|value| !value.is_empty())
                            .map(|value| (group_name.clone(), value.to_string()))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

static RUNTIME_SNAPSHOT_SERVICE: Lazy<RuntimeSnapshotService> = Lazy::new(RuntimeSnapshotService::new);
static RUNTIME_PROXY_SELECTION_STATE: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| RwLock::new(HashMap::new()));
static RUNTIME_PROXY_DELAY_STATE: Lazy<RwLock<RuntimeProxyDelayState>> =
    Lazy::new(|| RwLock::new(RuntimeProxyDelayState::default()));
static RUNTIME_PROVIDER_HEALTH_STATE: Lazy<RwLock<RuntimeProviderHealthState>> =
    Lazy::new(|| RwLock::new(RuntimeProviderHealthState::default()));
static RUNTIME_LIFECYCLE_STATE: Lazy<RwLock<RuntimeLifecycleState>> =
    Lazy::new(|| RwLock::new(RuntimeLifecycleState::default()));
static RUNTIME_UPGRADE_HISTORY_STATE: Lazy<RwLock<RuntimeUpgradeHistoryState>> =
    Lazy::new(|| RwLock::new(RuntimeUpgradeHistoryState::default()));
const RUNTIME_PROXY_SELECTIONS_FILE: &str = "proxy-selections.yaml";
const RUNTIME_PROXY_DELAYS_FILE: &str = "proxy-delays.yaml";
const RUNTIME_PROVIDER_HEALTH_FILE: &str = "provider-health.yaml";
const RUNTIME_LIFECYCLE_EVENTS_FILE: &str = "lifecycle-events.yaml";
const RUNTIME_LIFECYCLE_EVENTS_CAP: usize = 100;
const RUNTIME_UPGRADE_HISTORY_FILE: &str = "core-upgrade-history.yaml";
const RUNTIME_UPGRADE_HISTORY_CAP: usize = 50;

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct RuntimeProxySelectionState {
    pub groups: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProxyDelayRecord {
    pub group_name: String,
    pub proxy_name: String,
    pub delay: u32,
    pub test_url: String,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RuntimeProxyDelayState {
    pub records: Vec<RuntimeProxyDelayRecord>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProviderHealthRecord {
    pub provider_name: String,
    pub success: bool,
    pub error: Option<String>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RuntimeProviderHealthState {
    pub records: Vec<RuntimeProviderHealthRecord>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLifecycleRecord {
    pub kind: String,
    pub success: bool,
    pub error: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RuntimeUpgradeHistoryState {
    pub records: Vec<RuntimeLifecycleRecord>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct RuntimeLifecycleState {
    pub records: Vec<RuntimeLifecycleRecord>,
}

#[derive(Debug, Default)]
pub struct RuntimeSnapshotService;

impl RuntimeSnapshotService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn global() -> &'static Self {
        &RUNTIME_SNAPSHOT_SERVICE
    }

    pub async fn refresh_dns_metrics(&self) -> RuntimeSnapshot {
        let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
        let mut snapshot = RuntimeSnapshot {
            core_running,
            ..RuntimeSnapshot::default()
        };

        if core_running {
            snapshot.dns_metrics = Some(DnsMetrics::default());
        }

        snapshot
    }

    pub async fn refresh_proxies(&self) -> RuntimeSnapshot {
        let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
        let mut snapshot = RuntimeSnapshot {
            core_running,
            ..RuntimeSnapshot::default()
        };

        if core_running {
            snapshot.proxies = self.proxies_from_runtime_config().await;
            snapshot.proxies_from_runtime_config = snapshot.proxies.is_some();
        }

        snapshot
    }

    pub async fn refresh_proxies_result(&self) -> Result<RuntimeSnapshot> {
        let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
        let mut snapshot = RuntimeSnapshot {
            core_running,
            ..RuntimeSnapshot::default()
        };

        if core_running {
            snapshot.proxies = Some(self.require_proxies_from_runtime_config().await?);
            snapshot.proxies_from_runtime_config = true;
        }

        Ok(snapshot)
    }

    pub async fn refresh_proxy_topology_from_runtime_config(&self) -> Result<RuntimeSnapshot> {
        let core_running = *CoreManager::global().get_running_mode() != RunningMode::NotRunning;
        let runtime = Config::runtime().await;
        let runtime = runtime.latest_arc();
        let config = runtime
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
        Ok(RuntimeSnapshot {
            core_running,
            proxies: Some(build_proxies_from_runtime_config(config)),
            proxies_from_runtime_config: true,
            ..RuntimeSnapshot::default()
        })
    }

    pub async fn refresh_runtime_version_result(&self) -> Result<RuntimeSnapshot> {
        let mut snapshot = self.runtime_read_snapshot();
        snapshot.version = Some(MihomoVersion {
            meta: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
        });
        Ok(snapshot)
    }

    pub async fn refresh_runtime_base_config_result(&self) -> Result<RuntimeSnapshot> {
        let mut snapshot = self.runtime_read_snapshot();
        snapshot.base_config = Some(self.require_base_config_from_runtime_config().await?);
        Ok(snapshot)
    }

    pub async fn refresh_runtime_dns_metrics_result(&self) -> Result<RuntimeSnapshot> {
        // The only resolver the Rust kernel answers itself is the in-stack
        // fake-IP answerer on the TUN datapath; outside TUN mode DNS is forwarded
        // verbatim with no instrumentation. `runtime_dns_stats` returns `None`
        // unless a TUN inbound is live, so report unavailable rather than
        // fabricating zeroed counters. The cache/query/per-server/recent-query
        // and resolution-path trust fields carry real data; only pollution
        // analysis has no honest in-process source and stays empty (the panel
        // hides that section).
        let stats = CoreManager::global().runtime_dns_stats().await.ok_or_else(|| {
            anyhow::anyhow!("DNS metrics are not available: the Rust kernel only instruments DNS in TUN mode (in-stack fake-IP), which is not active")
        })?;
        let mut snapshot = self.runtime_read_snapshot();
        snapshot.dns_metrics = Some(dns_metrics_from_stats(&stats));
        Ok(snapshot)
    }

    pub async fn refresh_runtime_engine_stats_result(&self) -> Result<RuntimeSnapshot> {
        let mut snapshot = self.runtime_read_snapshot();
        let live = runtime_live_connection_count().await;
        snapshot.engine_stats = Some(EngineStats {
            active_connections: live,
            tracked_conns: live,
        });
        Ok(snapshot)
    }

    pub async fn refresh_runtime_perf_stats_result(&self) -> Result<RuntimeSnapshot> {
        // PerfStats models the Go runtime (goroutines, GOGC, GC pauses, Go heap),
        // none of which exist in the Rust kernel. Report unavailable rather than
        // surfacing meaningless zeros under Go-specific labels.
        anyhow::bail!("perf stats are not available: the Rust runtime kernel does not expose Go-runtime metrics")
    }

    pub async fn refresh_runtime_buffer_pool_stats_result(&self) -> Result<RuntimeSnapshot> {
        // The Rust kernel relies on tokio's built-in copy buffers and has no
        // custom size-classed buffer pool to report on.
        anyhow::bail!("buffer pool stats are not available: the Rust runtime kernel has no custom buffer pool")
    }

    pub async fn refresh_runtime_hot_reload_status_result(&self) -> Result<RuntimeSnapshot> {
        let mut snapshot = self.runtime_read_snapshot();
        let rule_version = self.rule_version_from_runtime_config().await.unwrap_or_default();
        snapshot.hot_reload_status = Some(HotReloadStatus {
            rule_version,
            protected_conns: runtime_live_connection_count().await,
        });
        Ok(snapshot)
    }

    pub async fn refresh_runtime_rule_traffic_result(&self) -> Result<RuntimeSnapshot> {
        // Each tracked connection records the rule the router matched plus its
        // live byte counters, so the conntrack table already carries a real
        // per-rule traffic breakdown — no extra kernel bookkeeping needed.
        let table = CoreManager::global()
            .runtime_connections()
            .await
            .ok_or_else(|| anyhow::anyhow!("rule traffic is not available: the kernel is not running"))?;
        let mut snapshot = self.runtime_read_snapshot();
        snapshot.rule_traffic = Some(rule_traffic_from_kernel(&table));
        Ok(snapshot)
    }

    async fn rule_version_from_runtime_config(&self) -> Option<String> {
        let runtime = Config::runtime().await;
        let runtime = runtime.latest_arc();
        let config = runtime.config.as_ref()?;
        Some(rule_version_from_runtime_config(config))
    }

    pub async fn refresh_runtime_tls_fingerprint_stats_result(&self) -> Result<RuntimeSnapshot> {
        let mut snapshot = self.runtime_read_snapshot();
        let obfuscation = CoreManager::global()
            .runtime_obfuscation_stats()
            .await
            .unwrap_or_default();
        snapshot.tls_fingerprint_stats = Some(tls_fingerprint_stats_from_obfuscation(obfuscation));
        Ok(snapshot)
    }

    pub async fn refresh_runtime_connections_result(&self) -> Result<RuntimeSnapshot> {
        let mut snapshot = self.runtime_read_snapshot();
        let table = CoreManager::global().runtime_connections().await.unwrap_or_default();
        snapshot.connections = Some(connections_from_kernel(table));
        Ok(snapshot)
    }

    pub async fn refresh_runtime_rules_result(&self) -> Result<RuntimeSnapshot> {
        let mut snapshot = self.runtime_read_snapshot();
        snapshot.rules = Some(self.require_rules_from_runtime_config().await?);
        Ok(snapshot)
    }

    pub async fn refresh_runtime_proxies_result(&self) -> Result<RuntimeSnapshot> {
        let mut snapshot = self.runtime_read_snapshot();
        snapshot.proxies = Some(self.require_proxies_from_runtime_config().await?);
        snapshot.proxies_from_runtime_config = true;
        Ok(snapshot)
    }

    pub async fn refresh_current_egress_status_result(
        &self,
        _app_handle: &tauri::AppHandle,
    ) -> Result<RuntimeSnapshot> {
        let mut snapshot = self.runtime_read_snapshot();
        snapshot.egress_status = Some(egress_status_from_monitor());
        Ok(snapshot)
    }

    async fn proxies_from_runtime_config(&self) -> Option<Proxies> {
        let runtime = Config::runtime().await;
        let runtime = runtime.latest_arc();
        let config = runtime.config.as_ref()?;
        Some(build_proxies_from_runtime_config(config))
    }

    async fn require_proxies_from_runtime_config(&self) -> Result<Proxies> {
        let runtime = Config::runtime().await;
        let runtime = runtime.latest_arc();
        let config = runtime
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
        Ok(build_proxies_from_runtime_config(config))
    }

    async fn require_rules_from_runtime_config(&self) -> Result<Rules> {
        let runtime = Config::runtime().await;
        let runtime = runtime.latest_arc();
        let config = runtime
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
        Ok(build_rules_from_runtime_config(config))
    }

    async fn require_base_config_from_runtime_config(&self) -> Result<BaseConfig> {
        let runtime = Config::runtime().await;
        let runtime = runtime.latest_arc();
        let config = runtime
            .config
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("runtime config is not available"))?;
        Ok(build_base_config_from_runtime_config(config))
    }

    fn runtime_read_snapshot(&self) -> RuntimeSnapshot {
        RuntimeSnapshot {
            core_running: *CoreManager::global().get_running_mode() != RunningMode::NotRunning,
            ..RuntimeSnapshot::default()
        }
    }
}

/// Build the egress-status telemetry payload from the in-process egress monitor
/// (public-IP probe loop) instead of querying the external Mihomo controller's
/// `/engine/egress/status` endpoint.
fn egress_status_from_monitor() -> EgressStatus {
    let stats = crate::core::egress_monitor::egress_monitor().get_stats();
    let egress_ip = stats.last_probe.as_ref().map(|probe| probe.ip.clone());
    EgressStatus {
        stable: stats.ip_change_count == 0,
        change_count: stats.ip_change_count as i64,
        observed_count: Some(stats.successful_probes as i64),
        public_egress_ip: egress_ip.clone(),
        egress_ip,
        egress_source: Some("egressMonitor".to_string()),
        sample_count: Some(stats.successful_probes as i64),
        ..EgressStatus::default()
    }
}

/// Shape the kernel's in-process client-obfuscation snapshot (TLS ClientHello
/// fingerprint shaping) into the Mihomo `TLSFingerprintStats` payload so the
/// telemetry consumer parses it unchanged. Replaces the Mihomo controller
/// `/engine/obfuscation/tls` query against the external Go kernel.
pub(crate) fn tls_fingerprint_stats_from_obfuscation(snap: learn_gripe::ObfuscationSnapshot) -> TLSFingerprintStats {
    TLSFingerprintStats {
        current_fingerprint: snap.current_tls_fingerprint,
        rotation_count: snap.tls_rotation_count as i64,
        usage_snapshot: snap
            .fingerprint_usage
            .into_iter()
            .map(|(label, count)| (label, count as i64))
            .collect(),
    }
}

/// Convert the kernel's in-process connection table into the Mihomo-compatible
/// `Connections` DTO the app and frontend already consume. The kernel owns the
/// data plane, so this replaces the former Mihomo controller `/connections`
/// query. Fields the kernel does not track (process info, GeoIP/ASN, sniffing,
/// DSCP) are left empty/default, as they were with no Mihomo controller.
pub(crate) fn connections_from_kernel(table: learn_gripe::ConnTableSnapshot) -> Connections {
    let connections = table.connections.into_iter().map(connection_from_kernel).collect();
    Connections {
        download_total: table.download_total,
        upload_total: table.upload_total,
        connections: Some(connections),
        memory: 0,
    }
}

/// Aggregate the live conntrack table into per-rule traffic totals. Every
/// tracked connection records the rule type/payload the router matched plus its
/// live upload/download counters, so summing those by `(rule type, payload)`
/// yields a real per-rule traffic breakdown — the same shape the retired Go
/// controller reported over `/engine/rules/traffic`, but sourced in-process.
/// Connections no rule router matched (empty rule, e.g. a non-routed outbound
/// mode) are skipped since they carry no rule attribution. `last_active` is the
/// newest connection start for the rule, the closest signal the table exposes.
pub(crate) fn rule_traffic_from_kernel(
    table: &learn_gripe::ConnTableSnapshot,
) -> HashMap<std::string::String, RuleTrafficSnapshot> {
    let mut map: HashMap<std::string::String, RuleTrafficSnapshot> = HashMap::new();
    for conn in &table.connections {
        if conn.meta.rule.is_empty() {
            continue;
        }
        let key = format!("{}:{}", conn.meta.rule, conn.meta.rule_payload);
        let entry = map.entry(key).or_insert_with(|| RuleTrafficSnapshot {
            rule_type: conn.meta.rule.clone(),
            rule_payload: conn.meta.rule_payload.clone(),
            upload: 0,
            download: 0,
            connections: 0,
            last_active: 0,
        });
        entry.upload = entry.upload.saturating_add(conn.upload as i64);
        entry.download = entry.download.saturating_add(conn.download as i64);
        entry.connections = entry.connections.saturating_add(1);
        entry.last_active = entry.last_active.max(conn.start_unix_ms as i64);
    }
    map
}

/// Shape the in-stack DNS counters into the `DnsMetrics` DTO the telemetry panel
/// renders. Only the cache and query sections carry real data: `A` questions
/// either hit an existing fake-IP mapping or allocate a new one (a miss), and
/// every accepted question counts toward the totals. The kernel answers entirely
/// from the local fake-IP pool, so success == total - errors and there is no
/// per-query latency to report. In fake-IP TUN mode the in-stack answerer is the
/// single DNS server handling every query, so the `servers` section carries one
/// honest entry for it (derived from the same counters). The `trust` section is
/// also honest in this mode: queries are answered locally and never leave the
/// host, while the real name resolution happens at the proxy egress over the
/// encrypted tunnel, so the resolution path carries zero DNS-leak risk. Only
/// pollution analysis has no honest in-process source (it would need to compare
/// answers against a trusted baseline), so it stays empty and the panel hides it.
pub(crate) fn dns_metrics_from_stats(stats: &learn_gripe::DnsStatsSnapshot) -> DnsMetrics {
    // A cache hit is an `A` question whose domain already had a mapping; the
    // remaining `A` questions allocated a new entry (a miss).
    let cache_hits = stats.cache_hits;
    let cache_misses = stats.a_queries.saturating_sub(cache_hits);
    let cache_lookups = stats.a_queries;
    let hit_rate = if cache_lookups > 0 {
        cache_hits as f64 / cache_lookups as f64
    } else {
        0.0
    };

    let total = stats.total_queries;
    let failed = stats.errors;
    let success = total.saturating_sub(failed);

    // The answerer records each question it served; surface them as the recent
    // query list (already newest-first). Routing fields (proxy/rule/egress) are
    // unknown at answer time and stay `None`; there is no upstream round-trip to
    // time, so latency is 0. `server` names the in-stack answerer itself.
    let recent = stats
        .recent
        .iter()
        .map(|q| DnsQueryEvent {
            domain: q.domain.clone(),
            q_type: q.q_type.clone(),
            server: "fake-ip (in-stack)".to_string(),
            protocol: "udp".to_string(),
            proxy_name: None,
            proxy_chain: None,
            egress: None,
            rule: None,
            rule_payload: None,
            success: q.success,
            error: None,
            latency_us: 0,
            timestamp: unix_ms_to_rfc3339(q.unix_ms),
        })
        .collect();

    // In fake-IP TUN mode every query is answered by the one in-stack answerer,
    // so it is the sole DNS "server". Surface a single honest entry derived from
    // the same counters once at least one query has been served. `last_query` is
    // the newest recorded question's timestamp; `last_error` stays `None` because
    // parse/serialize failures are not tied to a specific upstream.
    let servers = if total > 0 {
        let last_query = stats
            .recent
            .first()
            .map(|q| unix_ms_to_rfc3339(q.unix_ms))
            .unwrap_or_default();
        vec![DnsServerStats {
            server: "fake-ip (in-stack)".to_string(),
            queries: total,
            successes: success,
            failures: failed,
            avg_latency_us: 0,
            last_query,
            last_error: None,
        }]
    } else {
        Vec::new()
    };

    // This DTO is only ever shaped from a live in-stack snapshot (the read
    // returns `Err` outside TUN mode), so the resolution path is always the
    // fake-IP answerer: every question is answered locally and no plaintext DNS
    // leaves the host. The real name resolution happens at the proxy egress over
    // the encrypted tunnel, so the path is leak-free (`leak_risk_score = 0`) and
    // classified at maximum trust. Pollution detection has no honest source and
    // stays empty.
    let trust = DnsTrustSummary {
        total: 1,
        encrypted: 1,
        unencrypted: 0,
        by_trust_level: HashMap::from([("maximum".to_string(), 1)]),
        servers: vec![DnsServerClassification {
            address: "fake-ip (in-stack)".to_string(),
            protocol: "fakeip".to_string(),
            trust_level: "maximum".to_string(),
            encrypted: true,
            description: Some("查询在本机 fake-IP 应答，真实解析在代理出口经加密隧道完成，DNS 不出本机".to_string()),
        }],
        leak_risk_score: 0.0,
        last_evaluated: unix_ms_to_rfc3339(now_millis()),
    };

    DnsMetrics {
        cache: DnsCacheStats {
            hit: cache_hits,
            miss: cache_misses,
            size: stats.fake_ip_entries,
            hit_rate,
        },
        queries: DnsQueryStats {
            total,
            success,
            failed,
            // The fake-IP answerer resolves synchronously from an in-memory pool;
            // there is no upstream round-trip to time.
            avg_latency_us: 0,
            max_latency_us: 0,
        },
        servers,
        recent,
        pollution: DnsPollutionStats::default(),
        trust,
    }
}

fn connection_from_kernel(conn: learn_gripe::ConnSnapshot) -> Connection {
    let meta = conn.meta;
    let (source_ip, source_port) = split_socket_addr(meta.source);
    let (inbound_ip, inbound_port) = split_socket_addr(meta.inbound_local);
    let destination_ip = meta.destination_ip.map(|ip| ip.to_string()).unwrap_or_default();
    Connection {
        id: conn.id.to_string(),
        metadata: ConnectionMetaData {
            network: network_from_kernel(meta.network),
            connection_type: ConnectionType::Unknown("Mixed".to_string()),
            source_ip,
            destination_ip,
            source_geo_ip: None,
            destination_geo_ip: None,
            source_ip_asn: String::new(),
            destination_ip_asn: String::new(),
            source_port,
            destination_port: meta.destination_port.to_string(),
            inbound_ip,
            inbound_port,
            inbound_name: "mixed".to_string(),
            inbound_user: String::new(),
            host: meta.host,
            dns_mode: DNSMode::Normal,
            uid: 0,
            process: String::new(),
            process_path: String::new(),
            special_proxy: String::new(),
            special_rules: String::new(),
            remote_destination: String::new(),
            dscp: 0,
            sniff_host: String::new(),
        },
        upload: conn.upload,
        download: conn.download,
        start: unix_ms_to_rfc3339(conn.start_unix_ms),
        chains: meta.chains,
        provider_chains: None,
        rule: meta.rule,
        rule_payload: meta.rule_payload,
    }
}

fn network_from_kernel(network: learn_gripe::ConnNetwork) -> Network {
    match network {
        learn_gripe::ConnNetwork::Tcp => Network::TCP,
        learn_gripe::ConnNetwork::Udp => Network::UDP,
    }
}

fn split_socket_addr(addr: Option<std::net::SocketAddr>) -> (String, String) {
    match addr {
        Some(addr) => (addr.ip().to_string(), addr.port().to_string()),
        None => (String::new(), String::new()),
    }
}

fn unix_ms_to_rfc3339(unix_ms: u64) -> String {
    let time = UNIX_EPOCH + std::time::Duration::from_millis(unix_ms);
    chrono::DateTime::<chrono::Utc>::from(time).to_rfc3339()
}

/// Number of live connections tracked by the in-process kernel conntrack table.
/// This is the real data source backing engine/hot-reload telemetry that the
/// retired Go kernel previously reported over the controller API. Returns 0 when
/// the kernel is not running (no table to snapshot).
async fn runtime_live_connection_count() -> i64 {
    CoreManager::global()
        .runtime_connections()
        .await
        .map(|table| table.connections.len() as i64)
        .unwrap_or(0)
}

/// Derive a stable rule-set version identifier from the loaded runtime config.
/// The retired Go kernel exposed a hot-reload rule version; the Rust runtime
/// reloads rules by restarting the kernel, so we surface a content hash of the
/// active `rules` and `rule-providers` that changes whenever the rule set does.
/// Returns an empty string when the config declares no rules.
fn rule_version_from_runtime_config(config: &serde_yaml_ng::Mapping) -> String {
    use std::hash::{Hash, Hasher};

    let rules = config.get("rules");
    let providers = config.get("rule-providers");
    if rules.is_none() && providers.is_none() {
        return String::new();
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    if let Some(rules) = rules {
        serde_yaml_ng::to_string(rules).unwrap_or_default().hash(&mut hasher);
    }
    if let Some(providers) = providers {
        serde_yaml_ng::to_string(providers)
            .unwrap_or_default()
            .hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}

pub async fn read_runtime_version() -> Result<MihomoVersion> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_version_result()
        .await?;
    runtime_readback(snapshot.version, "version")
}

pub async fn read_runtime_base_config() -> Result<BaseConfig> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_base_config_result()
        .await?;
    runtime_readback(snapshot.base_config, "base config")
}

pub async fn read_runtime_dns_metrics() -> Result<DnsMetrics> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_dns_metrics_result()
        .await?;
    runtime_readback(snapshot.dns_metrics, "DNS metrics")
}

pub async fn read_runtime_engine_stats() -> Result<EngineStats> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_engine_stats_result()
        .await?;
    runtime_readback(snapshot.engine_stats, "engine stats")
}

pub async fn read_runtime_perf_stats() -> Result<PerfStats> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_perf_stats_result()
        .await?;
    runtime_readback(snapshot.perf_stats, "perf stats")
}

pub async fn read_runtime_buffer_pool_stats() -> Result<BufferPoolStats> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_buffer_pool_stats_result()
        .await?;
    runtime_readback(snapshot.buffer_pool_stats, "buffer pool stats")
}

pub async fn read_runtime_hot_reload_status() -> Result<HotReloadStatus> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_hot_reload_status_result()
        .await?;
    runtime_readback(snapshot.hot_reload_status, "hot reload status")
}

pub async fn read_runtime_rule_traffic() -> Result<HashMap<std::string::String, RuleTrafficSnapshot>> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_rule_traffic_result()
        .await?;
    runtime_readback(snapshot.rule_traffic, "rule traffic")
}

pub async fn read_runtime_tls_fingerprint_stats() -> Result<TLSFingerprintStats> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_tls_fingerprint_stats_result()
        .await?;
    runtime_readback(snapshot.tls_fingerprint_stats, "TLS fingerprint stats")
}

pub async fn read_runtime_connections() -> Result<Connections> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_connections_result()
        .await?;
    runtime_readback(snapshot.connections, "connections")
}

pub async fn read_runtime_rules() -> Result<Rules> {
    let snapshot = RuntimeSnapshotService::global().refresh_runtime_rules_result().await?;
    runtime_readback(snapshot.rules, "rules")
}

pub async fn read_runtime_proxies() -> Result<Proxies> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_runtime_proxies_result()
        .await?;
    runtime_readback(snapshot.proxies, "proxies")
}

pub async fn read_current_egress_status(app_handle: &tauri::AppHandle) -> Result<EgressStatus> {
    let snapshot = RuntimeSnapshotService::global()
        .refresh_current_egress_status_result(app_handle)
        .await?;
    runtime_readback(snapshot.egress_status, "egress status")
}

pub async fn read_subscription_control_plane_topology(
    _app_handle: &tauri::AppHandle,
    group_name: &str,
) -> Result<(Proxy, Proxies)> {
    let proxies = RuntimeSnapshotService::global()
        .require_proxies_from_runtime_config()
        .await?;
    let group = proxies
        .proxies
        .get(group_name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("proxy group '{group_name}' not found in runtime config"))?;
    Ok((group, proxies))
}

fn runtime_readback<T>(value: Option<T>, label: &str) -> Result<T> {
    value.ok_or_else(|| anyhow::anyhow!("runtime {label} readback unavailable"))
}

/// Build the `BaseConfig` telemetry payload from the in-process runtime config
/// (the merged Clash mapping) instead of querying the external Mihomo
/// controller's `/configs` endpoint. Scalar fields are read straight from the
/// mapping; fields the merged config does not carry fall back to honest
/// defaults.
pub fn build_base_config_from_runtime_config(config: &serde_yaml_ng::Mapping) -> BaseConfig {
    let get_str = |key: &str| config.get(key).and_then(Value::as_str);
    let get_u16 = |key: &str| config.get(key).and_then(Value::as_u64).map(|value| value as u16);
    let get_bool = |key: &str| config.get(key).and_then(Value::as_bool);

    let mode = match get_str("mode").map(str::to_ascii_lowercase).as_deref() {
        Some("global") => ClashMode::Global,
        Some("direct") => ClashMode::Direct,
        _ => ClashMode::Rule,
    };
    let log_level = match get_str("log-level").map(str::to_ascii_lowercase).as_deref() {
        Some("debug") => LogLevel::DEBUG,
        Some("warning") => LogLevel::WARNING,
        Some("error") => LogLevel::ERROR,
        Some("silent") => LogLevel::SILENT,
        _ => LogLevel::INFO,
    };
    let find_process_mode = match get_str("find-process-mode").map(str::to_ascii_lowercase).as_deref() {
        Some("always") => FindProcessMode::Always,
        Some("strict") => FindProcessMode::Strict,
        _ => FindProcessMode::Off,
    };

    let tun = config
        .get("tun")
        .and_then(Value::as_mapping)
        .map(build_tun_config_from_mapping)
        .unwrap_or_default();

    BaseConfig {
        port: get_u16("port").unwrap_or_default(),
        socks_port: get_u16("socks-port").unwrap_or_default(),
        redir_port: get_u16("redir-port").unwrap_or_default(),
        tproxy_port: get_u16("tproxy-port").unwrap_or_default(),
        mixed_port: get_u16("mixed-port").unwrap_or_default(),
        tun,
        allow_lan: get_bool("allow-lan").unwrap_or_default(),
        bind_address: get_str("bind-address").unwrap_or("*").to_string(),
        mode,
        unified_delay: get_bool("unified-delay").unwrap_or_default(),
        log_level,
        ipv6: get_bool("ipv6").unwrap_or_default(),
        interface_name: get_str("interface-name").unwrap_or_default().to_string(),
        geodata_mode: get_bool("geodata-mode").unwrap_or_default(),
        tcp_concurrent: get_bool("tcp-concurrent").unwrap_or_default(),
        find_process_mode,
        sniffing: get_bool("sniffing").unwrap_or_default(),
        global_client_fingerprint: get_str("global-client-fingerprint").unwrap_or_default().to_string(),
        global_ua: get_str("global-ua").unwrap_or_default().to_string(),
        ..BaseConfig::default()
    }
}

fn build_tun_config_from_mapping(tun: &serde_yaml_ng::Mapping) -> TunConfig {
    let get_str = |key: &str| tun.get(key).and_then(Value::as_str);
    let get_bool = |key: &str| tun.get(key).and_then(Value::as_bool);

    let stack = match get_str("stack").map(str::to_ascii_lowercase).as_deref() {
        Some("gvisor") => TunStack::Gvisor,
        Some("system") => TunStack::System,
        _ => TunStack::Mixed,
    };
    let dns_hijack = tun
        .get("dns-hijack")
        .and_then(Value::as_sequence)
        .map(|items| items.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();

    TunConfig {
        enable: get_bool("enable").unwrap_or_default(),
        device: get_str("device").unwrap_or_default().to_string(),
        stack,
        dns_hijack,
        auto_route: get_bool("auto-route").unwrap_or_default(),
        auto_detect_interface: get_bool("auto-detect-interface").unwrap_or_default(),
        ..TunConfig::default()
    }
}

pub fn build_proxies_from_runtime_config(config: &serde_yaml_ng::Mapping) -> Proxies {
    let mut proxies = HashMap::new();

    if let Some(items) = config.get("proxies").and_then(Value::as_sequence) {
        for item in items {
            if let Some(proxy) = proxy_from_config_item(item) {
                proxies.insert(proxy.name.clone(), proxy);
            }
        }
    }

    let mut group_names = Vec::new();
    if let Some(groups) = config.get("proxy-groups").and_then(Value::as_sequence) {
        for item in groups {
            if let Some(group) = proxy_group_from_config_item(item) {
                group_names.push(group.name.clone());
                proxies.insert(group.name.clone(), group);
            }
        }
    }

    for builtin in [
        builtin_proxy("DIRECT", ProxyType::Direct),
        builtin_proxy("REJECT", ProxyType::Reject),
        builtin_proxy("REJECT-DROP", ProxyType::RejectDrop),
    ] {
        proxies.entry(builtin.name.clone()).or_insert(builtin);
    }

    if !proxies.contains_key("GLOBAL") {
        let global_all = if group_names.is_empty() {
            proxies
                .keys()
                .filter(|name| !matches!(name.as_str(), "GLOBAL" | "DIRECT" | "REJECT" | "REJECT-DROP"))
                .cloned()
                .collect::<Vec<_>>()
        } else {
            group_names
        };
        proxies.insert(
            "GLOBAL".into(),
            proxy_group("GLOBAL", ProxyType::Selector, global_all, None, None, None, None),
        );
    }

    apply_proxy_selection_state(&mut proxies);
    apply_proxy_delay_state(&mut proxies);

    Proxies { proxies }
}

pub fn runtime_proxy_selection_state() -> HashMap<String, String> {
    RUNTIME_PROXY_SELECTION_STATE
        .read()
        .map(|state| state.clone())
        .unwrap_or_default()
}

pub fn record_runtime_proxy_selection(group_name: &str, proxy_name: &str) {
    if let Ok(mut state) = RUNTIME_PROXY_SELECTION_STATE.write() {
        state.insert(group_name.to_string(), proxy_name.to_string());
    }
}

pub fn record_and_persist_runtime_proxy_selection(group_name: &str, proxy_name: &str) {
    record_runtime_proxy_selection(group_name, proxy_name);
    if let Err(error) = persist_runtime_proxy_selection_state() {
        log::warn!("failed to persist runtime proxy selection state: {error}");
    }
    record_and_persist_runtime_lifecycle_event(
        "select-runtime-proxy",
        true,
        None,
        Some(format!("group={group_name};proxy={proxy_name}")),
    );
}

pub fn runtime_proxy_delay_state() -> RuntimeProxyDelayState {
    RUNTIME_PROXY_DELAY_STATE
        .read()
        .map(|state| state.clone())
        .unwrap_or_default()
}

pub fn record_and_persist_runtime_proxy_delay(group_name: &str, proxy_name: &str, delay: u32, test_url: &str) {
    if let Ok(mut state) = RUNTIME_PROXY_DELAY_STATE.write() {
        let updated_at = now_millis();
        let record = RuntimeProxyDelayRecord {
            group_name: group_name.to_string(),
            proxy_name: proxy_name.to_string(),
            delay,
            test_url: test_url.to_string(),
            updated_at,
        };
        if let Some(existing) = state
            .records
            .iter_mut()
            .find(|item| item.group_name == group_name && item.proxy_name == proxy_name)
        {
            *existing = record;
        } else {
            state.records.push(record);
        }
    }
    if let Err(error) = persist_runtime_proxy_delay_state() {
        log::warn!("failed to persist runtime proxy delay state: {error}");
    }
}

pub fn load_runtime_proxy_delay_state_from_disk() -> Result<()> {
    let path = runtime_proxy_delay_state_path()?;
    if !path.exists() {
        return Ok(());
    }
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            log::warn!("failed to read runtime proxy delay state: {error}");
            return Ok(());
        }
    };
    let document = match serde_yaml_ng::from_str::<RuntimeProxyDelayState>(&content) {
        Ok(document) => document,
        Err(error) => {
            log::warn!("failed to parse runtime proxy delay state: {error}");
            return Ok(());
        }
    };
    if let Ok(mut state) = RUNTIME_PROXY_DELAY_STATE.write() {
        *state = document;
    }
    Ok(())
}

pub fn runtime_provider_health_state() -> RuntimeProviderHealthState {
    RUNTIME_PROVIDER_HEALTH_STATE
        .read()
        .map(|state| state.clone())
        .unwrap_or_default()
}

pub fn record_and_persist_runtime_provider_health(provider_name: &str, success: bool, error: Option<String>) {
    if let Ok(mut state) = RUNTIME_PROVIDER_HEALTH_STATE.write() {
        let updated_at = now_millis();
        let record = RuntimeProviderHealthRecord {
            provider_name: provider_name.to_string(),
            success,
            error,
            updated_at,
        };
        if let Some(existing) = state
            .records
            .iter_mut()
            .find(|item| item.provider_name == provider_name)
        {
            *existing = record;
        } else {
            state.records.push(record);
        }
    }
    if let Err(error) = persist_runtime_provider_health_state() {
        log::warn!("failed to persist runtime provider health state: {error}");
    }
}

pub fn runtime_lifecycle_state() -> RuntimeLifecycleState {
    RUNTIME_LIFECYCLE_STATE
        .read()
        .map(|state| state.clone())
        .unwrap_or_default()
}

pub fn record_and_persist_runtime_lifecycle_event(
    kind: &str,
    success: bool,
    error: Option<String>,
    detail: Option<String>,
) {
    if let Ok(mut state) = RUNTIME_LIFECYCLE_STATE.write() {
        let record = RuntimeLifecycleRecord {
            kind: kind.to_string(),
            success,
            error,
            detail,
            updated_at: now_millis(),
        };
        state.records.push(record);
        let len = state.records.len();
        if len > RUNTIME_LIFECYCLE_EVENTS_CAP {
            state.records.drain(0..len - RUNTIME_LIFECYCLE_EVENTS_CAP);
        }
    }
    if let Err(error) = persist_runtime_lifecycle_state() {
        log::warn!("failed to persist runtime lifecycle state: {error}");
    }
}

pub fn runtime_upgrade_history_state() -> RuntimeUpgradeHistoryState {
    RUNTIME_UPGRADE_HISTORY_STATE
        .read()
        .map(|state| state.clone())
        .unwrap_or_default()
}

pub fn record_and_persist_runtime_upgrade_event(
    kind: &str,
    success: bool,
    error: Option<String>,
    detail: Option<String>,
) {
    if let Ok(mut state) = RUNTIME_UPGRADE_HISTORY_STATE.write() {
        let record = RuntimeLifecycleRecord {
            kind: kind.to_string(),
            success,
            error,
            detail,
            updated_at: now_millis(),
        };
        state.records.push(record);
        let len = state.records.len();
        if len > RUNTIME_UPGRADE_HISTORY_CAP {
            state.records.drain(0..len - RUNTIME_UPGRADE_HISTORY_CAP);
        }
    }
    if let Err(error) = persist_runtime_upgrade_history() {
        log::warn!("failed to persist runtime upgrade history: {error}");
    }
}

pub fn load_runtime_upgrade_history_from_disk() -> Result<()> {
    let path = runtime_upgrade_history_path()?;
    if !path.exists() {
        return Ok(());
    }
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            log::warn!("failed to read runtime upgrade history: {error}");
            return Ok(());
        }
    };
    let document = match serde_yaml_ng::from_str::<RuntimeUpgradeHistoryState>(&content) {
        Ok(document) => document,
        Err(error) => {
            log::warn!("failed to parse runtime upgrade history: {error}");
            return Ok(());
        }
    };
    if let Ok(mut state) = RUNTIME_UPGRADE_HISTORY_STATE.write() {
        *state = document;
    }
    Ok(())
}

pub fn load_runtime_lifecycle_state_from_disk() -> Result<()> {
    let path = runtime_lifecycle_state_path()?;
    if !path.exists() {
        return Ok(());
    }
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            log::warn!("failed to read runtime lifecycle state: {error}");
            return Ok(());
        }
    };
    let document = match serde_yaml_ng::from_str::<RuntimeLifecycleState>(&content) {
        Ok(document) => document,
        Err(error) => {
            log::warn!("failed to parse runtime lifecycle state: {error}");
            return Ok(());
        }
    };
    if let Ok(mut state) = RUNTIME_LIFECYCLE_STATE.write() {
        *state = document;
    }
    Ok(())
}

pub fn load_runtime_provider_health_state_from_disk() -> Result<()> {
    let path = runtime_provider_health_state_path()?;
    if !path.exists() {
        return Ok(());
    }
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            log::warn!("failed to read runtime provider health state: {error}");
            return Ok(());
        }
    };
    let document = match serde_yaml_ng::from_str::<RuntimeProviderHealthState>(&content) {
        Ok(document) => document,
        Err(error) => {
            log::warn!("failed to parse runtime provider health state: {error}");
            return Ok(());
        }
    };
    if let Ok(mut state) = RUNTIME_PROVIDER_HEALTH_STATE.write() {
        *state = document;
    }
    Ok(())
}

pub fn load_runtime_proxy_selection_state_from_disk() -> Result<()> {
    let path = runtime_proxy_selection_state_path()?;
    if !path.exists() {
        return Ok(());
    }
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) => {
            log::warn!("failed to read runtime proxy selection state: {error}");
            return Ok(());
        }
    };
    let document = match serde_yaml_ng::from_str::<RuntimeProxySelectionState>(&content) {
        Ok(document) => document,
        Err(error) => {
            log::warn!("failed to parse runtime proxy selection state: {error}");
            return Ok(());
        }
    };
    if let Ok(mut state) = RUNTIME_PROXY_SELECTION_STATE.write() {
        *state = document.groups;
    }
    Ok(())
}

fn persist_runtime_proxy_selection_state() -> Result<()> {
    let path = runtime_proxy_selection_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let document = RuntimeProxySelectionState {
        groups: runtime_proxy_selection_state(),
    };
    fs::write(path, serde_yaml_ng::to_string(&document)?)?;
    Ok(())
}

fn persist_runtime_proxy_delay_state() -> Result<()> {
    let path = runtime_proxy_delay_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_yaml_ng::to_string(&runtime_proxy_delay_state())?)?;
    Ok(())
}

fn persist_runtime_provider_health_state() -> Result<()> {
    let path = runtime_provider_health_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_yaml_ng::to_string(&runtime_provider_health_state())?)?;
    Ok(())
}

fn persist_runtime_lifecycle_state() -> Result<()> {
    let path = runtime_lifecycle_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_yaml_ng::to_string(&runtime_lifecycle_state())?)?;
    Ok(())
}

fn persist_runtime_upgrade_history() -> Result<()> {
    let path = runtime_upgrade_history_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_yaml_ng::to_string(&runtime_upgrade_history_state())?)?;
    Ok(())
}

fn runtime_upgrade_history_path() -> Result<PathBuf> {
    Ok(crate::utils::dirs::app_runtime_dir()?.join(RUNTIME_UPGRADE_HISTORY_FILE))
}

fn runtime_proxy_selection_state_path() -> Result<PathBuf> {
    Ok(crate::utils::dirs::app_runtime_dir()?.join(RUNTIME_PROXY_SELECTIONS_FILE))
}

fn runtime_proxy_delay_state_path() -> Result<PathBuf> {
    Ok(crate::utils::dirs::app_runtime_dir()?.join(RUNTIME_PROXY_DELAYS_FILE))
}

fn runtime_provider_health_state_path() -> Result<PathBuf> {
    Ok(crate::utils::dirs::app_runtime_dir()?.join(RUNTIME_PROVIDER_HEALTH_FILE))
}

fn runtime_lifecycle_state_path() -> Result<PathBuf> {
    Ok(crate::utils::dirs::app_runtime_dir()?.join(RUNTIME_LIFECYCLE_EVENTS_FILE))
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn apply_proxy_selection_state(proxies: &mut HashMap<String, Proxy>) {
    let state = runtime_proxy_selection_state();
    for (group_name, proxy_name) in state {
        let Some(group) = proxies.get_mut(&group_name) else {
            continue;
        };
        let Some(all) = group.all.as_ref() else {
            continue;
        };
        if all.iter().any(|candidate| candidate == &proxy_name) {
            group.now = Some(proxy_name);
        }
    }
}

fn apply_proxy_delay_state(proxies: &mut HashMap<String, Proxy>) {
    for record in runtime_proxy_delay_state().records {
        let Some(proxy) = proxies.get_mut(&record.proxy_name) else {
            continue;
        };
        apply_proxy_delay_record(proxy, &record);
    }
}

fn apply_proxy_delay_state_to_list(proxies: &mut [Proxy]) {
    let records = runtime_proxy_delay_state().records;
    for proxy in proxies {
        let proxy_name = proxy.name.clone();
        for record in records.iter().filter(|record| record.proxy_name == proxy_name) {
            apply_proxy_delay_record(proxy, record);
        }
    }
}

fn apply_proxy_delay_record(proxy: &mut Proxy, record: &RuntimeProxyDelayRecord) {
    proxy.history.push(DelayHistory {
        time: record.updated_at.to_string(),
        delay: u16::try_from(record.delay).unwrap_or(u16::MAX),
    });
    proxy.alive = record.delay > 0 && record.delay < 1_000_000;
}

fn proxy_from_config_item(item: &Value) -> Option<Proxy> {
    let name = string_field(item, "name")?;
    let proxy_type = proxy_type_from_str(string_field(item, "type").as_deref());
    Some(Proxy {
        name,
        proxy_type,
        alive: true,
        udp: bool_field(item, "udp").unwrap_or(false),
        uot: bool_field(item, "uot").unwrap_or(false),
        xudp: bool_field(item, "xudp").unwrap_or(false),
        tfo: bool_field(item, "tfo").unwrap_or(false),
        mptcp: bool_field(item, "mptcp").unwrap_or(false),
        smux: bool_field(item, "smux").unwrap_or(false),
        interface: string_field(item, "interface-name").unwrap_or_default(),
        dialer_proxy: string_field(item, "dialer-proxy").unwrap_or_default(),
        routing_mark: i32_field(item, "routing-mark").unwrap_or_default(),
        provider_name: string_field(item, "provider"),
        all: None,
        expected_status: None,
        fixed: None,
        hidden: bool_field(item, "hidden"),
        icon: string_field(item, "icon"),
        now: None,
        test_url: None,
        id: None,
        history: Vec::new(),
        extra: HashMap::new(),
    })
}

fn proxy_group_from_config_item(item: &Value) -> Option<Proxy> {
    let name = string_field(item, "name")?;
    let all = item
        .get("proxies")
        .and_then(Value::as_sequence)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(std::string::String::from)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(proxy_group(
        &name,
        proxy_type_from_str(string_field(item, "type").as_deref()),
        all,
        string_field(item, "test-url"),
        bool_field(item, "hidden"),
        string_field(item, "icon"),
        string_field(item, "fixed"),
    ))
}

fn proxy_group(
    name: &str,
    proxy_type: ProxyType,
    all: Vec<String>,
    test_url: Option<String>,
    hidden: Option<bool>,
    icon: Option<String>,
    fixed: Option<String>,
) -> Proxy {
    Proxy {
        name: name.into(),
        proxy_type,
        alive: true,
        udp: true,
        uot: false,
        xudp: false,
        tfo: false,
        mptcp: false,
        smux: false,
        interface: String::new(),
        dialer_proxy: String::new(),
        routing_mark: 0,
        provider_name: None,
        now: all.first().cloned(),
        all: Some(all),
        expected_status: None,
        fixed,
        hidden,
        icon,
        test_url,
        id: None,
        history: Vec::new(),
        extra: HashMap::<String, Extra>::new(),
    }
}

fn builtin_proxy(name: &str, proxy_type: ProxyType) -> Proxy {
    Proxy {
        name: name.into(),
        proxy_type,
        alive: true,
        udp: true,
        uot: false,
        xudp: false,
        tfo: false,
        mptcp: false,
        smux: false,
        interface: String::new(),
        dialer_proxy: String::new(),
        routing_mark: 0,
        provider_name: None,
        all: None,
        expected_status: None,
        fixed: None,
        hidden: None,
        icon: None,
        now: None,
        test_url: None,
        id: None,
        history: Vec::<DelayHistory>::new(),
        extra: HashMap::new(),
    }
}

fn proxy_type_from_str(value: Option<&str>) -> ProxyType {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "direct" => ProxyType::Direct,
        "reject" => ProxyType::Reject,
        "reject-drop" => ProxyType::RejectDrop,
        "compatible" => ProxyType::Compatible,
        "pass" => ProxyType::Pass,
        "dns" => ProxyType::Dns,
        "ss" | "shadowsocks" => ProxyType::Shadowsocks,
        "ssr" | "shadowsocksr" => ProxyType::ShadowsocksR,
        "snell" => ProxyType::Snell,
        "socks" | "socks5" => ProxyType::Socks5,
        "http" => ProxyType::Http,
        "vmess" => ProxyType::Vmess,
        "vless" => ProxyType::Vless,
        "trojan" => ProxyType::Trojan,
        "hysteria" => ProxyType::Hysteria,
        "hysteria2" | "hy2" => ProxyType::Hysteria2,
        "wireguard" | "wg" => ProxyType::WireGuard,
        "tuic" => ProxyType::Tuic,
        "ssh" => ProxyType::Ssh,
        "mieru" => ProxyType::Mieru,
        "masque" => ProxyType::Masque,
        "anytls" => ProxyType::AnyTLS,
        "relay" => ProxyType::Relay,
        "select" | "selector" => ProxyType::Selector,
        "fallback" => ProxyType::Fallback,
        "url-test" => ProxyType::URLTest,
        "load-balance" | "loadbalance" => ProxyType::LoadBalance,
        other if other.is_empty() => ProxyType::Unknown("unknown".into()),
        other => ProxyType::Unknown(other.into()),
    }
}

fn string_field(item: &Value, field: &str) -> Option<String> {
    item.get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(std::string::String::from)
}

fn bool_field(item: &Value, field: &str) -> Option<bool> {
    item.get(field).and_then(Value::as_bool)
}

fn i32_field(item: &Value, field: &str) -> Option<i32> {
    item.get(field)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn i64_field(item: &Value, field: &str) -> Option<i64> {
    item.get(field).and_then(Value::as_i64)
}

/// Build proxy providers from runtime config YAML and provider files on disk.
pub fn build_proxy_providers_from_runtime_config(config: &serde_yaml_ng::Mapping) -> ProxyProviders {
    let mut providers = HashMap::new();

    let Some(provider_map) = config.get("proxy-providers").and_then(Value::as_mapping) else {
        return ProxyProviders { providers };
    };

    let app_home = crate::utils::dirs::app_home_dir().unwrap_or_default();

    for (key, value) in provider_map {
        let Some(name) = key.as_str() else { continue };
        let Some(provider) = build_single_provider(name, value, &app_home) else {
            continue;
        };
        providers.insert(name.to_string(), provider);
    }

    ProxyProviders { providers }
}

fn build_single_provider(name: &str, value: &Value, app_home: &std::path::Path) -> Option<ProxyProvider> {
    let vehicle_type = match string_field(value, "type").as_deref() {
        Some("http") => VehicleType::HTTP,
        Some("file") => VehicleType::File,
        Some("inline") => VehicleType::Inline,
        _ => VehicleType::Compatible,
    };

    let test_url = value
        .get("health-check")
        .and_then(|hc| hc.get("url"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let expected_status = value
        .get("health-check")
        .and_then(|hc| hc.get("expected-status"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let mut proxies = load_provider_proxies(value, app_home, name);
    apply_proxy_delay_state_to_list(&mut proxies);

    let subscription_info = load_subscription_info(value);

    Some(ProxyProvider {
        name: name.to_string(),
        provider_type: ProviderType::Proxy,
        vehicle_type,
        proxies,
        test_url,
        expected_status,
        updated_at: None,
        subscription_info,
    })
}

/// Load proxy nodes from provider file on disk.
fn load_provider_proxies(provider_config: &Value, app_home: &std::path::Path, provider_name: &str) -> Vec<Proxy> {
    // Inline providers have proxies embedded in the config
    if let Some(payload) = provider_config.get("payload").and_then(Value::as_sequence) {
        return payload
            .iter()
            .filter_map(|item| {
                let mut proxy = proxy_from_config_item(item)?;
                proxy.provider_name = Some(provider_name.to_string());
                Some(proxy)
            })
            .collect();
    }

    // File/HTTP providers store proxies in a file on disk
    let path_str = match string_field(provider_config, "path") {
        Some(p) => p,
        None => return Vec::new(),
    };

    let file_path = if std::path::Path::new(&path_str).is_absolute() {
        std::path::PathBuf::from(&path_str)
    } else {
        app_home.join(&path_str)
    };

    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    parse_provider_file_content(&content, provider_name)
}

/// Parse provider file content (supports both proxies key and bare sequence).
fn parse_provider_file_content(content: &str, provider_name: &str) -> Vec<Proxy> {
    let value: Value = match serde_yaml_ng::from_str(content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let sequence = if let Some(seq) = value.get("proxies").and_then(Value::as_sequence) {
        seq.clone()
    } else if let Some(seq) = value.as_sequence() {
        seq.clone()
    } else {
        return Vec::new();
    };

    sequence
        .iter()
        .filter_map(|item| {
            let mut proxy = proxy_from_config_item(item)?;
            proxy.provider_name = Some(provider_name.to_string());
            Some(proxy)
        })
        .collect()
}

/// Try to load subscription info from the provider config.
fn load_subscription_info(provider_config: &Value) -> Option<SubScriptionInfo> {
    let sub_info = provider_config.get("subscription-info")?;
    Some(SubScriptionInfo {
        upload: i64_field(sub_info, "Upload")
            .or_else(|| i64_field(sub_info, "upload"))
            .unwrap_or(0),
        download: i64_field(sub_info, "Download")
            .or_else(|| i64_field(sub_info, "download"))
            .unwrap_or(0),
        total: i64_field(sub_info, "Total")
            .or_else(|| i64_field(sub_info, "total"))
            .unwrap_or(0),
        expire: i64_field(sub_info, "Expire")
            .or_else(|| i64_field(sub_info, "expire"))
            .unwrap_or(0),
    })
}

pub fn build_rules_from_runtime_config(config: &serde_yaml_ng::Mapping) -> Rules {
    let mut rules = Vec::new();
    let mut rule_set_targets = HashMap::new();

    if let Some(items) = config.get("rules").and_then(Value::as_sequence) {
        for item in items {
            let Some(rule) = rule_from_value(item, rules.len() as i32, "profile", None) else {
                continue;
            };
            if matches!(rule.rule_type, RuleType::RuleSet) && !rule.payload.is_empty() && !rule.proxy.is_empty() {
                rule_set_targets.insert(rule.payload.clone(), rule.proxy.clone());
            }
            rules.push(rule);
        }
    }

    append_rule_provider_rules(config, &mut rules, &rule_set_targets);

    let total = i32::try_from(rules.len()).unwrap_or(i32::MAX);
    Rules {
        rules,
        total: Some(total),
        page: Some(1),
        page_size: Some(total),
    }
}

pub fn build_rule_providers_from_runtime_config(config: &serde_yaml_ng::Mapping) -> RuleProviders {
    let mut providers = HashMap::new();

    let Some(provider_map) = config.get("rule-providers").and_then(Value::as_mapping) else {
        return RuleProviders { providers };
    };

    let app_home = crate::utils::dirs::app_home_dir().unwrap_or_default();

    for (key, value) in provider_map {
        let Some(name) = key.as_str() else { continue };
        let provider = build_single_rule_provider(name, value, &app_home);
        providers.insert(name.to_string(), provider);
    }

    RuleProviders { providers }
}

fn append_rule_provider_rules(
    config: &serde_yaml_ng::Mapping,
    rules: &mut Vec<Rule>,
    targets: &HashMap<String, String>,
) {
    let Some(provider_map) = config.get("rule-providers").and_then(Value::as_mapping) else {
        return;
    };

    let app_home = crate::utils::dirs::app_home_dir().unwrap_or_default();

    for (key, value) in provider_map {
        let Some(name) = key.as_str() else { continue };
        let behavior = rule_behavior_from_str(string_field(value, "behavior").as_deref());
        let target = targets.get(name).map(std::string::String::as_str);
        let source = format!("provider:{name}");

        for payload in load_rule_provider_payloads(value, &app_home) {
            let index = i32::try_from(rules.len()).unwrap_or(i32::MAX);
            let rule = match behavior {
                RuleBehavior::Classical => rule_from_line(&payload, index, &source, target),
                RuleBehavior::Domain => Some(rule_from_provider_payload(
                    index,
                    RuleType::Domain,
                    payload,
                    target.unwrap_or_default().to_string(),
                    source.clone(),
                )),
                RuleBehavior::IpCidr => Some(rule_from_provider_payload(
                    index,
                    RuleType::IPCIDR,
                    payload,
                    target.unwrap_or_default().to_string(),
                    source.clone(),
                )),
            };

            if let Some(rule) = rule {
                rules.push(rule);
            }
        }
    }
}

fn build_single_rule_provider(name: &str, value: &Value, app_home: &std::path::Path) -> RuleProvider {
    let payloads = load_rule_provider_payloads(value, app_home);
    RuleProvider {
        behavior: rule_behavior_from_str(string_field(value, "behavior").as_deref()),
        format: rule_format_from_str(string_field(value, "format").as_deref()),
        name: name.to_string(),
        rule_count: u32::try_from(payloads.len()).unwrap_or(u32::MAX),
        provider_type: ProviderType::Rule,
        updated_at: provider_file_updated_at(value, app_home),
        vehicle_type: vehicle_type_from_str(string_field(value, "type").as_deref()),
    }
}

fn rule_from_value(item: &Value, index: i32, source: &str, fallback_proxy: Option<&str>) -> Option<Rule> {
    let line = item.as_str()?;
    rule_from_line(line, index, source, fallback_proxy)
}

fn rule_from_line(line: &str, index: i32, source: &str, fallback_proxy: Option<&str>) -> Option<Rule> {
    let fields = split_rule_fields(line);
    let rule_type_field = fields.first()?.trim();
    let rule_type = rule_type_from_str(Some(rule_type_field));
    let payload = if matches!(rule_type, RuleType::Match) {
        String::new()
    } else {
        fields.get(1).cloned().unwrap_or_default()
    };
    let proxy = if matches!(rule_type, RuleType::Match) {
        fields
            .get(1)
            .cloned()
            .or_else(|| fallback_proxy.map(std::string::String::from))
            .unwrap_or_default()
    } else {
        fields
            .get(2)
            .cloned()
            .or_else(|| fallback_proxy.map(std::string::String::from))
            .unwrap_or_default()
    };

    Some(Rule {
        index,
        rule_type,
        payload,
        proxy,
        size: i32::try_from(line.len()).unwrap_or(i32::MAX),
        source: source.to_string(),
        extra: None,
    })
}

fn rule_from_provider_payload(index: i32, rule_type: RuleType, payload: String, proxy: String, source: String) -> Rule {
    Rule {
        index,
        rule_type,
        size: i32::try_from(payload.len()).unwrap_or(i32::MAX),
        payload,
        proxy,
        source,
        extra: None,
    }
}

fn load_rule_provider_payloads(provider_config: &Value, app_home: &std::path::Path) -> Vec<String> {
    if let Some(payload) = provider_config.get("payload").and_then(Value::as_sequence) {
        return collect_payload_entries(payload);
    }

    let Some(file_path) = provider_file_path(provider_config, app_home) else {
        return Vec::new();
    };
    let content = match std::fs::read_to_string(&file_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };

    parse_rule_provider_file_content(&content)
}

fn parse_rule_provider_file_content(content: &str) -> Vec<String> {
    let value: Result<Value, _> = serde_yaml_ng::from_str(content);
    let Ok(value) = value else {
        return content_lines(content);
    };

    if let Some(payload) = value.get("payload").and_then(Value::as_sequence) {
        return collect_payload_entries(payload);
    }
    if let Some(rules) = value.get("rules").and_then(Value::as_sequence) {
        return collect_payload_entries(rules);
    }
    if let Some(sequence) = value.as_sequence() {
        return collect_payload_entries(sequence);
    }
    if let Some(text) = value.as_str() {
        return content_lines(text);
    }

    Vec::new()
}

fn content_lines(content: &str) -> Vec<String> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(std::string::String::from)
        .collect()
}

fn collect_payload_entries(items: &[Value]) -> Vec<String> {
    items
        .iter()
        .filter_map(|item| {
            item.as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(std::string::String::from)
        })
        .collect()
}

fn provider_file_path(provider_config: &Value, app_home: &std::path::Path) -> Option<std::path::PathBuf> {
    let path_str = string_field(provider_config, "path")?;
    if std::path::Path::new(&path_str).is_absolute() {
        Some(std::path::PathBuf::from(&path_str))
    } else {
        Some(app_home.join(&path_str))
    }
}

fn provider_file_updated_at(provider_config: &Value, app_home: &std::path::Path) -> String {
    let Some(file_path) = provider_file_path(provider_config, app_home) else {
        return String::new();
    };
    let Ok(metadata) = std::fs::metadata(&file_path) else {
        return String::new();
    };
    let Ok(modified) = metadata.modified() else {
        return String::new();
    };
    chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339()
}

fn split_rule_fields(line: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut depth = 0_i32;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for ch in line.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            current.push(ch);
            escaped = true;
            continue;
        }

        match ch {
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
                current.push(ch);
            }
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
                current.push(ch);
            }
            '(' | '[' | '{' if !in_single_quote && !in_double_quote => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' | '}' if !in_single_quote && !in_double_quote => {
                depth = (depth - 1).max(0);
                current.push(ch);
            }
            ',' if depth == 0 && !in_single_quote && !in_double_quote => {
                fields.push(current.trim().to_string());
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    fields.push(current.trim().to_string());
    fields
}

fn rule_type_from_str(value: Option<&str>) -> RuleType {
    let Some(raw) = value else {
        return RuleType::Unknown("unknown".into());
    };
    match raw.replace(['-', '_'], "").to_ascii_uppercase().as_str() {
        "DOMAIN" => RuleType::Domain,
        "DOMAINSUFFIX" => RuleType::DomainSuffix,
        "DOMAINKEYWORD" => RuleType::DomainKeyword,
        "DOMAINREGEX" => RuleType::DomainRegex,
        "GEOSITE" => RuleType::GeoSite,
        "GEOIP" => RuleType::GeoIP,
        "SRCGEOIP" => RuleType::SrcGeoIP,
        "IPASN" => RuleType::IPASN,
        "SRCIPASN" => RuleType::SrcIPASN,
        "IPCIDR" => RuleType::IPCIDR,
        "SRCIPCIDR" => RuleType::SrcIPCIDR,
        "IPSUFFIX" => RuleType::IPSuffix,
        "SRCIPSUFFIX" => RuleType::SrcIPSuffix,
        "SRCPORT" => RuleType::SrcPort,
        "DSTPORT" => RuleType::DstPort,
        // spellchecker:disable-next-line
        "INPORT" => RuleType::InPort,
        "INUSER" => RuleType::InUser,
        "INNAME" => RuleType::InName,
        "INTYPE" => RuleType::InType,
        "PROCESSNAME" => RuleType::ProcessName,
        "PROCESSPATH" => RuleType::ProcessPath,
        "PROCESSNAMEREGEX" => RuleType::ProcessNameRegex,
        "PROCESSPATHREGEX" => RuleType::ProcessPathRegex,
        "MATCH" => RuleType::Match,
        "RULESET" => RuleType::RuleSet,
        "NETWORK" => RuleType::Network,
        "DSCP" => RuleType::DSCP,
        "UID" => RuleType::Uid,
        "SUBRULES" => RuleType::SubRules,
        "AND" => RuleType::AND,
        "OR" => RuleType::OR,
        "NOT" => RuleType::NOT,
        _ => RuleType::Unknown(raw.to_string()),
    }
}

fn rule_behavior_from_str(value: Option<&str>) -> RuleBehavior {
    match value
        .unwrap_or_default()
        .replace(['-', '_'], "")
        .to_ascii_lowercase()
        .as_str()
    {
        "domain" => RuleBehavior::Domain,
        "ipcidr" => RuleBehavior::IpCidr,
        _ => RuleBehavior::Classical,
    }
}

fn rule_format_from_str(value: Option<&str>) -> RuleFormat {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "text" => RuleFormat::Text,
        "mrs" => RuleFormat::Mrs,
        _ => RuleFormat::Yaml,
    }
}

fn vehicle_type_from_str(value: Option<&str>) -> VehicleType {
    match value.unwrap_or_default().to_ascii_lowercase().as_str() {
        "http" => VehicleType::HTTP,
        "file" => VehicleType::File,
        "inline" => VehicleType::Inline,
        _ => VehicleType::Compatible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clash_dtos::{Proxies, Proxy, ProxyType};
    use std::collections::HashMap;

    fn proxy_group(name: &str, now: &str) -> Proxy {
        Proxy {
            all: Some(vec!["node-a".into(), "node-b".into()]),
            expected_status: None,
            fixed: None,
            hidden: None,
            icon: None,
            now: Some(now.into()),
            test_url: None,
            id: None,
            alive: true,
            history: Vec::new(),
            extra: HashMap::new(),
            name: name.into(),
            udp: true,
            uot: false,
            proxy_type: ProxyType::Selector,
            xudp: false,
            tfo: false,
            mptcp: false,
            smux: false,
            interface: String::new(),
            dialer_proxy: String::new(),
            routing_mark: 0,
            provider_name: None,
        }
    }

    #[test]
    fn snapshot_collects_stable_group_selections() {
        let snapshot = RuntimeSnapshot {
            core_running: true,
            proxies: Some(Proxies {
                proxies: HashMap::from([
                    (
                        "VERGE-STABLE-example".into(),
                        proxy_group("VERGE-STABLE-example", "node-a"),
                    ),
                    ("GLOBAL".into(), proxy_group("GLOBAL", "node-b")),
                ]),
            }),
            proxies_from_runtime_config: false,
            ..RuntimeSnapshot::default()
        };

        let selections = snapshot.stable_group_selected_nodes();

        assert_eq!(
            selections.get("VERGE-STABLE-example").map(std::string::String::as_str),
            Some("node-a")
        );
        assert_eq!(selections.get("GLOBAL"), None);
    }

    #[test]
    fn snapshot_without_proxies_has_no_stable_group_selections() {
        let snapshot = RuntimeSnapshot {
            core_running: false,
            proxies: None,
            proxies_from_runtime_config: false,
            ..RuntimeSnapshot::default()
        };

        assert!(snapshot.stable_group_selected_nodes().is_empty());
    }

    #[test]
    fn global_snapshot_service_is_available() {
        let service = RuntimeSnapshotService::global();

        assert!(std::ptr::eq(service, RuntimeSnapshotService::global()));
    }

    #[test]
    fn runtime_config_topology_builds_proxies_groups_and_global() {
        let config: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r#"
proxies:
  - name: node-a
    type: ss
    udp: true
    dialer-proxy: relay-a
  - name: node-b
    type: vmess
proxy-groups:
  - name: Auto
    type: url-test
    proxies:
      - node-a
      - node-b
    test-url: https://example.com/generate_204
"#,
        )
        .unwrap();

        let topology = build_proxies_from_runtime_config(&config);

        let node_a = topology.proxies.get("node-a").unwrap();
        assert_eq!(node_a.proxy_type, ProxyType::Shadowsocks);
        assert_eq!(node_a.dialer_proxy, "relay-a");
        let auto = topology.proxies.get("Auto").unwrap();
        assert_eq!(auto.proxy_type, ProxyType::URLTest);
        assert_eq!(auto.now.as_deref(), Some("node-a"));
        assert_eq!(
            auto.all.as_ref().unwrap(),
            &vec!["node-a".to_string(), "node-b".to_string()]
        );
        let global = topology.proxies.get("GLOBAL").unwrap();
        assert_eq!(global.proxy_type, ProxyType::Selector);
        assert_eq!(global.all.as_ref().unwrap(), &vec!["Auto".to_string()]);
        assert!(topology.proxies.contains_key("DIRECT"));
        assert!(topology.proxies.contains_key("REJECT"));
    }

    #[test]
    fn runtime_proxy_topology_applies_selection_state_cache() {
        let config: serde_yaml_ng::Mapping = serde_yaml_ng::from_str(
            r#"
proxies:
  - name: cache-node-a
    type: ss
  - name: cache-node-b
    type: ss
proxy-groups:
  - name: CacheSelector
    type: select
    proxies:
      - cache-node-a
      - cache-node-b
"#,
        )
        .unwrap();

        record_runtime_proxy_selection("CacheSelector", "cache-node-b");

        let topology = build_proxies_from_runtime_config(&config);

        assert_eq!(
            topology
                .proxies
                .get("CacheSelector")
                .and_then(|group| group.now.as_deref()),
            Some("cache-node-b")
        );
    }

    #[test]
    fn builds_rules_from_runtime_config_and_inline_rule_provider() {
        let runtime_yaml = r#"
rules:
  - DOMAIN-SUFFIX,example.com,DIRECT
  - RULE-SET,ads,REJECT
  - MATCH,DIRECT
rule-providers:
  ads:
    type: http
    behavior: domain
    format: yaml
    payload:
      - ads.example
"#;
        let value = serde_yaml_ng::from_str::<Value>(runtime_yaml).unwrap();
        let config = value.as_mapping().unwrap();

        let rules = build_rules_from_runtime_config(config);

        assert_eq!(rules.rules.len(), 4);
        assert_eq!(rules.rules[0].rule_type, RuleType::DomainSuffix);
        assert_eq!(rules.rules[0].payload, "example.com");
        assert_eq!(rules.rules[0].proxy, "DIRECT");
        assert_eq!(rules.rules[2].rule_type, RuleType::Match);
        assert_eq!(rules.rules[2].payload, "");
        assert_eq!(rules.rules[2].proxy, "DIRECT");
        assert_eq!(rules.rules[3].source, "provider:ads");
        assert_eq!(rules.rules[3].rule_type, RuleType::Domain);
        assert_eq!(rules.rules[3].payload, "ads.example");
        assert_eq!(rules.rules[3].proxy, "REJECT");
    }

    #[test]
    fn builds_rule_providers_from_runtime_config() {
        let runtime_yaml = r#"
rule-providers:
  cn:
    type: file
    behavior: ipcidr
    format: text
    payload:
      - 10.0.0.0/8
      - 192.168.0.0/16
"#;
        let value = serde_yaml_ng::from_str::<Value>(runtime_yaml).unwrap();
        let config = value.as_mapping().unwrap();

        let providers = build_rule_providers_from_runtime_config(config);
        let provider = providers.providers.get("cn").unwrap();

        assert_eq!(provider.name, "cn");
        assert_eq!(provider.rule_count, 2);
        assert_eq!(provider.behavior, RuleBehavior::IpCidr);
        assert_eq!(provider.format, RuleFormat::Text);
        assert_eq!(provider.provider_type, ProviderType::Rule);
        assert_eq!(provider.vehicle_type, VehicleType::File);
    }

    fn conn(id: u64, rule: &str, payload: &str, upload: u64, download: u64, start: u64) -> learn_gripe::ConnSnapshot {
        learn_gripe::ConnSnapshot {
            id,
            meta: learn_gripe::ConnMeta {
                network: learn_gripe::ConnNetwork::Tcp,
                source: None,
                inbound_local: None,
                host: "example.com".into(),
                destination_ip: None,
                destination_port: 443,
                chains: vec!["DIRECT".into()],
                rule: rule.into(),
                rule_payload: payload.into(),
            },
            upload,
            download,
            start_unix_ms: start,
        }
    }

    #[test]
    fn rule_traffic_aggregates_bytes_and_connections_per_rule() {
        let table = learn_gripe::ConnTableSnapshot {
            connections: vec![
                conn(1, "DomainSuffix", "example.com", 100, 200, 1_000),
                conn(2, "DomainSuffix", "example.com", 50, 70, 2_000),
                conn(3, "GeoIP", "CN", 10, 20, 1_500),
                // No rule router matched -> skipped (no rule attribution).
                conn(4, "", "", 999, 999, 3_000),
            ],
            upload_total: 1159,
            download_total: 1289,
        };

        let traffic = rule_traffic_from_kernel(&table);

        assert_eq!(traffic.len(), 2);
        let suffix = traffic.get("DomainSuffix:example.com").unwrap();
        assert_eq!(suffix.rule_type, "DomainSuffix");
        assert_eq!(suffix.rule_payload, "example.com");
        assert_eq!(suffix.upload, 150);
        assert_eq!(suffix.download, 270);
        assert_eq!(suffix.connections, 2);
        assert_eq!(suffix.last_active, 2_000);

        let geoip = traffic.get("GeoIP:CN").unwrap();
        assert_eq!(geoip.connections, 1);
        assert_eq!(geoip.upload, 10);

        assert!(traffic.keys().all(|key| !key.starts_with(':')));
    }

    #[test]
    fn dns_metrics_map_cache_hits_misses_and_query_totals() {
        let stats = learn_gripe::DnsStatsSnapshot {
            total_queries: 10,
            a_queries: 8,
            aaaa_queries: 1,
            other_queries: 1,
            cache_hits: 6,
            errors: 2,
            fake_ip_entries: 2,
            recent: vec![
                learn_gripe::DnsRecentQuery {
                    domain: "example.com".to_string(),
                    q_type: "A".to_string(),
                    success: true,
                    unix_ms: 1_700_000_000_000,
                },
                learn_gripe::DnsRecentQuery {
                    domain: "blocked.test".to_string(),
                    q_type: "HTTPS".to_string(),
                    success: false,
                    unix_ms: 1_700_000_000_500,
                },
            ],
        };

        let metrics = dns_metrics_from_stats(&stats);

        // Cache: 6 of 8 A-questions hit; the other 2 allocated new entries.
        assert_eq!(metrics.cache.hit, 6);
        assert_eq!(metrics.cache.miss, 2);
        assert_eq!(metrics.cache.size, 2);
        assert!((metrics.cache.hit_rate - 0.75).abs() < 1e-9);

        // Queries: success == total - errors; no latency source.
        assert_eq!(metrics.queries.total, 10);
        assert_eq!(metrics.queries.success, 8);
        assert_eq!(metrics.queries.failed, 2);
        assert_eq!(metrics.queries.avg_latency_us, 0);
        assert_eq!(metrics.queries.max_latency_us, 0);

        // Recent queries map through newest-first with the wire-level facts the
        // answerer observed; routing/latency fields stay unset.
        assert_eq!(metrics.recent.len(), 2);
        assert_eq!(metrics.recent[0].domain, "example.com");
        assert_eq!(metrics.recent[0].q_type, "A");
        assert_eq!(metrics.recent[0].server, "fake-ip (in-stack)");
        assert!(metrics.recent[0].success);
        assert_eq!(metrics.recent[0].latency_us, 0);
        assert!(metrics.recent[0].rule.is_none());
        assert_eq!(metrics.recent[1].domain, "blocked.test");
        assert!(!metrics.recent[1].success);

        // The in-stack answerer is surfaced as the single DNS server, carrying
        // the same totals; `last_query` is the newest recorded question's time.
        assert_eq!(metrics.servers.len(), 1);
        assert_eq!(metrics.servers[0].server, "fake-ip (in-stack)");
        assert_eq!(metrics.servers[0].queries, 10);
        assert_eq!(metrics.servers[0].successes, 8);
        assert_eq!(metrics.servers[0].failures, 2);
        assert_eq!(metrics.servers[0].avg_latency_us, 0);
        assert_eq!(metrics.servers[0].last_query, metrics.recent[0].timestamp);
        assert!(metrics.servers[0].last_error.is_none());

        // Resolution-path trust is honest in fake-IP TUN mode: the sole resolver
        // is the local in-stack answerer, queries never leave the host, and real
        // resolution happens at the proxy egress over the encrypted tunnel, so
        // the path is leak-free at maximum trust.
        assert_eq!(metrics.trust.total, 1);
        assert_eq!(metrics.trust.encrypted, 1);
        assert_eq!(metrics.trust.unencrypted, 0);
        assert_eq!(metrics.trust.leak_risk_score, 0.0);
        assert_eq!(metrics.trust.by_trust_level.get("maximum"), Some(&1));
        assert_eq!(metrics.trust.servers.len(), 1);
        assert_eq!(metrics.trust.servers[0].address, "fake-ip (in-stack)");
        assert_eq!(metrics.trust.servers[0].protocol, "fakeip");
        assert_eq!(metrics.trust.servers[0].trust_level, "maximum");
        assert!(metrics.trust.servers[0].encrypted);
        assert!(!metrics.trust.last_evaluated.is_empty());

        // Pollution detection has no honest in-process source.
        assert_eq!(metrics.pollution.total_checked, 0);
    }

    #[test]
    fn dns_metrics_empty_snapshot_has_zero_hit_rate() {
        let metrics = dns_metrics_from_stats(&learn_gripe::DnsStatsSnapshot::default());
        assert_eq!(metrics.cache.hit, 0);
        assert_eq!(metrics.cache.miss, 0);
        assert_eq!(metrics.cache.hit_rate, 0.0);
        assert_eq!(metrics.queries.total, 0);
        assert_eq!(metrics.queries.success, 0);
        // No queries served yet, so no server entry is surfaced.
        assert!(metrics.servers.is_empty());
        // The leak-free resolution-path trust holds even before the first query.
        assert_eq!(metrics.trust.total, 1);
        assert_eq!(metrics.trust.leak_risk_score, 0.0);
    }
}
