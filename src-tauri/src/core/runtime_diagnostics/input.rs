use crate::core::runtime_snapshot::{RuntimeSnapshot, RuntimeSnapshotService};
use tauri_plugin_mihomo::models::DnsMetrics;

#[derive(Debug, Default)]
pub struct DiagnosticsInput {
    pub core_running: bool,
    pub dns_metrics: Option<DnsMetrics>,
}

impl DiagnosticsInput {
    pub fn from_snapshot(snapshot: RuntimeSnapshot) -> Self {
        Self {
            core_running: snapshot.core_running,
            dns_metrics: snapshot.dns_metrics,
        }
    }
}

pub async fn build_diagnostics_input(snapshot_service: &RuntimeSnapshotService) -> DiagnosticsInput {
    DiagnosticsInput::from_snapshot(snapshot_service.refresh_dns_metrics().await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::runtime_snapshot::RuntimeSnapshot;

    #[test]
    fn diagnostics_input_preserves_missing_dns_metrics() {
        let snapshot = RuntimeSnapshot {
            core_running: true,
            proxies: None,
            dns_metrics: None,
        };

        let input = DiagnosticsInput::from_snapshot(snapshot);

        assert!(input.core_running);
        assert!(input.dns_metrics.is_none());
    }
}
