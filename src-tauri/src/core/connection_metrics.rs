use crate::core::{runtime_lifecycle, runtime_snapshot::read_runtime_connections};
use anyhow::Result;
use once_cell::sync::Lazy;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tauri_plugin_mihomo::models::{Connection, Connections};
use tokio::sync::{RwLock, watch};

/// Per-connection speed snapshot computed from two consecutive Mihomo snapshots.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSpeed {
    pub id: String,
    pub cur_upload: u64,
    pub cur_download: u64,
}

/// Sanitized per-connection metadata used by app-runtime attribution planning.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionAttributionCandidate {
    pub id: String,
    pub process: String,
    pub process_path: String,
    pub host: String,
    pub rule: String,
    pub rule_payload: String,
    pub chains: Vec<String>,
    pub upload: u64,
    pub download: u64,
}

/// Aggregated traffic totals at a point in time.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrafficSnapshot {
    pub upload_total: u64,
    pub download_total: u64,
    pub upload_speed: u64,
    pub download_speed: u64,
    pub active_connection_count: usize,
    pub closed_since_last: usize,
    pub memory: u32,
}

/// Full metrics snapshot exposed via Tauri command.
#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionMetricsSnapshot {
    pub traffic: TrafficSnapshot,
    pub speeds: Vec<ConnectionSpeed>,
    pub attribution_candidates: Vec<ConnectionAttributionCandidate>,
    pub stale: bool,
}

/// Combined payload emitted as a Tauri event: computed metrics + raw connections.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionMetricsEventPayload {
    pub metrics: ConnectionMetricsSnapshot,
    pub raw: serde_json::Value,
}

struct PreviousState {
    upload_by_id: HashMap<String, u64>,
    download_by_id: HashMap<String, u64>,
    upload_total: u64,
    download_total: u64,
    updated_at: Instant,
}

/// Aggregates successive Mihomo connection snapshots into a unified metrics model.
pub struct ConnectionMetricsAggregator {
    previous: RwLock<Option<PreviousState>>,
    latest_snapshot: RwLock<Option<ConnectionMetricsSnapshot>>,
    stale_threshold_secs: u64,
}

impl ConnectionMetricsAggregator {
    pub fn new(stale_threshold_secs: u64) -> Arc<Self> {
        Arc::new(Self {
            previous: RwLock::new(None),
            latest_snapshot: RwLock::new(None),
            stale_threshold_secs,
        })
    }

    /// Ingest a raw `Connections` payload from Mihomo and compute the delta.
    pub async fn ingest(&self, payload: &Connections) {
        let connections = payload.connections.as_deref().unwrap_or(&[]);

        let now = Instant::now();
        let mut prev_guard = self.previous.write().await;

        let mut speeds = Vec::with_capacity(connections.len());
        let mut closed_since_last: usize = 0;

        let (upload_speed, download_speed) = if let Some(prev) = prev_guard.as_ref() {
            closed_since_last = count_closed(prev, connections);

            for conn in connections {
                let (cur_upload, cur_download) = match (
                    prev.upload_by_id.get(&conn.id).copied(),
                    prev.download_by_id.get(&conn.id).copied(),
                ) {
                    (Some(prev_up), Some(prev_down)) => (
                        conn.upload.saturating_sub(prev_up),
                        conn.download.saturating_sub(prev_down),
                    ),
                    _ => (0, 0),
                };
                speeds.push(ConnectionSpeed {
                    id: conn.id.clone(),
                    cur_upload,
                    cur_download,
                });
            }

            (
                payload.upload_total.saturating_sub(prev.upload_total),
                payload.download_total.saturating_sub(prev.download_total),
            )
        } else {
            for conn in connections {
                speeds.push(ConnectionSpeed {
                    id: conn.id.clone(),
                    cur_upload: 0,
                    cur_download: 0,
                });
            }
            (0, 0)
        };

        let traffic = TrafficSnapshot {
            upload_total: payload.upload_total,
            download_total: payload.download_total,
            upload_speed,
            download_speed,
            active_connection_count: connections.len(),
            closed_since_last,
            memory: payload.memory,
        };

        let snapshot = ConnectionMetricsSnapshot {
            traffic,
            speeds,
            attribution_candidates: connections.iter().map(connection_attribution_candidate).collect(),
            stale: false,
        };

        let mut upload_by_id = HashMap::with_capacity(connections.len());
        let mut download_by_id = HashMap::with_capacity(connections.len());
        for conn in connections {
            upload_by_id.insert(conn.id.clone(), conn.upload);
            download_by_id.insert(conn.id.clone(), conn.download);
        }

        *prev_guard = Some(PreviousState {
            upload_by_id,
            download_by_id,
            upload_total: payload.upload_total,
            download_total: payload.download_total,
            updated_at: now,
        });
        drop(prev_guard);

        *self.latest_snapshot.write().await = Some(snapshot);
    }

    /// Return the most recently computed snapshot (with stale detection).
    pub async fn snapshot(&self) -> ConnectionMetricsSnapshot {
        let mut snap = self
            .latest_snapshot
            .read()
            .await
            .clone()
            .unwrap_or_else(|| ConnectionMetricsSnapshot {
                traffic: TrafficSnapshot::default(),
                speeds: Vec::new(),
                attribution_candidates: Vec::new(),
                stale: true,
            });

        if let Some(prev) = self.previous.read().await.as_ref() {
            if prev.updated_at.elapsed().as_secs() > self.stale_threshold_secs {
                snap.stale = true;
            }
        }

        snap
    }

    /// Reset accumulated state (e.g. on sidecar restart).
    pub async fn reset(&self) {
        *self.previous.write().await = None;
        *self.latest_snapshot.write().await = None;
    }
}

fn count_closed(prev: &PreviousState, current: &[Connection]) -> usize {
    let current_ids: std::collections::HashSet<&str> = current.iter().map(|c| c.id.as_str()).collect();
    prev.upload_by_id
        .keys()
        .filter(|id| !current_ids.contains(id.as_str()))
        .count()
}

fn connection_attribution_candidate(conn: &Connection) -> ConnectionAttributionCandidate {
    ConnectionAttributionCandidate {
        id: conn.id.clone(),
        process: conn.metadata.process.clone(),
        process_path: conn.metadata.process_path.clone(),
        host: conn.metadata.host.clone(),
        rule: conn.rule.clone(),
        rule_payload: conn.rule_payload.clone(),
        chains: conn.chains.clone(),
        upload: conn.upload,
        download: conn.download,
    }
}

static CONNECTION_METRICS_AGGREGATOR: Lazy<Arc<ConnectionMetricsAggregator>> =
    Lazy::new(|| ConnectionMetricsAggregator::new(5));

static CONNECTION_METRICS_WATCH: Lazy<watch::Sender<ConnectionMetricsSnapshot>> = Lazy::new(|| {
    let (tx, _rx) = watch::channel(ConnectionMetricsSnapshot::default());
    tx
});

pub fn subscribe_connection_metrics() -> watch::Receiver<ConnectionMetricsSnapshot> {
    CONNECTION_METRICS_WATCH.subscribe()
}

pub async fn refresh_connection_metrics_snapshot() -> Result<ConnectionMetricsSnapshot> {
    if runtime_lifecycle::runtime_is_not_running() {
        CONNECTION_METRICS_AGGREGATOR.reset().await;
        return Ok(CONNECTION_METRICS_AGGREGATOR.snapshot().await);
    }

    let payload = read_runtime_connections().await?;
    CONNECTION_METRICS_AGGREGATOR.ingest(&payload).await;
    Ok(CONNECTION_METRICS_AGGREGATOR.snapshot().await)
}

pub async fn ingest_connection_metrics_snapshot(payload: &Connections) -> ConnectionMetricsSnapshot {
    CONNECTION_METRICS_AGGREGATOR.ingest(payload).await;
    let snapshot = CONNECTION_METRICS_AGGREGATOR.snapshot().await;
    let _ = CONNECTION_METRICS_WATCH.send(snapshot.clone());
    snapshot
}

pub async fn get_connection_metrics_snapshot() -> ConnectionMetricsSnapshot {
    CONNECTION_METRICS_AGGREGATOR.snapshot().await
}

pub async fn reset_connection_metrics() {
    CONNECTION_METRICS_AGGREGATOR.reset().await;
    let _ = CONNECTION_METRICS_WATCH.send(ConnectionMetricsSnapshot::default());
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_mihomo::models::{Connection, ConnectionMetaData, ConnectionType, Connections, DNSMode, Network};

    fn make_connection(id: &str, upload: u64, download: u64) -> Connection {
        Connection {
            id: id.to_string(),
            metadata: ConnectionMetaData {
                network: Network::TCP,
                connection_type: ConnectionType::HTTP,
                source_ip: "127.0.0.1".into(),
                destination_ip: "1.1.1.1".into(),
                source_geo_ip: None,
                destination_geo_ip: None,
                source_ip_asn: String::new(),
                destination_ip_asn: String::new(),
                source_port: "12345".into(),
                destination_port: "443".into(),
                inbound_ip: "127.0.0.1".into(),
                inbound_port: "7890".into(),
                inbound_name: String::new(),
                inbound_user: String::new(),
                host: "example.com".into(),
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
            upload,
            download,
            start: "2025-01-01T00:00:00Z".into(),
            chains: vec!["DIRECT".into()],
            provider_chains: None,
            rule: "MATCH".into(),
            rule_payload: String::new(),
        }
    }

    fn make_connections(conns: Vec<Connection>, upload_total: u64, download_total: u64) -> Connections {
        Connections {
            download_total,
            upload_total,
            connections: Some(conns),
            memory: 1024,
        }
    }

    #[tokio::test]
    async fn first_ingest_has_zero_speed() {
        let agg = ConnectionMetricsAggregator::new(5);
        let payload = make_connections(vec![make_connection("a", 100, 200)], 100, 200);
        agg.ingest(&payload).await;

        let snap = agg.snapshot().await;
        assert_eq!(snap.traffic.upload_total, 100);
        assert_eq!(snap.traffic.download_total, 200);
        assert_eq!(snap.traffic.upload_speed, 0);
        assert_eq!(snap.traffic.download_speed, 0);
        assert_eq!(snap.speeds.len(), 1);
        assert_eq!(snap.speeds[0].cur_upload, 0);
        assert_eq!(snap.attribution_candidates.len(), 1);
        assert_eq!(snap.attribution_candidates[0].id, "a");
        assert_eq!(snap.attribution_candidates[0].chains, vec!["DIRECT".to_string()]);
        assert!(!snap.stale);
    }

    #[tokio::test]
    async fn second_ingest_computes_delta() {
        let agg = ConnectionMetricsAggregator::new(5);

        let p1 = make_connections(vec![make_connection("a", 100, 200)], 100, 200);
        agg.ingest(&p1).await;

        let p2 = make_connections(vec![make_connection("a", 300, 500)], 300, 500);
        agg.ingest(&p2).await;

        let snap = agg.snapshot().await;
        assert_eq!(snap.traffic.upload_speed, 200);
        assert_eq!(snap.traffic.download_speed, 300);
        assert_eq!(snap.speeds[0].cur_upload, 200);
        assert_eq!(snap.speeds[0].cur_download, 300);
    }

    #[tokio::test]
    async fn closed_connections_detected() {
        let agg = ConnectionMetricsAggregator::new(5);

        let p1 = make_connections(
            vec![make_connection("a", 100, 200), make_connection("b", 50, 100)],
            150,
            300,
        );
        agg.ingest(&p1).await;

        // Connection "b" dropped
        let p2 = make_connections(vec![make_connection("a", 200, 400)], 200, 400);
        agg.ingest(&p2).await;

        let snap = agg.snapshot().await;
        assert_eq!(snap.traffic.active_connection_count, 1);
        assert_eq!(snap.traffic.closed_since_last, 1);
    }

    #[tokio::test]
    async fn empty_snapshot_is_stale() {
        let agg = ConnectionMetricsAggregator::new(5);
        let snap = agg.snapshot().await;
        assert!(snap.stale);
    }

    #[tokio::test]
    async fn reset_clears_state() {
        let agg = ConnectionMetricsAggregator::new(5);
        let p = make_connections(vec![make_connection("a", 10, 20)], 10, 20);
        agg.ingest(&p).await;

        agg.reset().await;

        let snap = agg.snapshot().await;
        assert!(snap.stale);
        assert_eq!(snap.traffic.upload_total, 0);
    }

    #[tokio::test]
    async fn new_connection_has_zero_speed() {
        let agg = ConnectionMetricsAggregator::new(5);

        let p1 = make_connections(vec![make_connection("a", 100, 200)], 100, 200);
        agg.ingest(&p1).await;

        // Connection "b" is new
        let p2 = make_connections(
            vec![make_connection("a", 200, 400), make_connection("b", 50, 100)],
            250,
            500,
        );
        agg.ingest(&p2).await;

        let snap = agg.snapshot().await;
        let b_speed = snap.speeds.iter().find(|s| s.id == "b").unwrap();
        assert_eq!(b_speed.cur_upload, 0);
        assert_eq!(b_speed.cur_download, 0);

        // But overall total speed still includes b's bytes
        assert_eq!(snap.traffic.upload_speed, 150);
        assert_eq!(snap.traffic.download_speed, 300);
    }
}
