use anyhow::Result;
use compact_str::CompactString;
use std::sync::Arc;

use crate::core::{CoreManager, manager::RunningMode, runtime_snapshot, validate::ValidationOutcome};

pub async fn init_runtime_core(reason: &str) -> Result<()> {
    let result = CoreManager::global().init().await;
    record_runtime_lifecycle_result("init-runtime-core", result.as_ref().map(|_| ()), reason);
    result
}

pub async fn start_runtime_core(reason: &str) -> Result<()> {
    let result = CoreManager::global().start_core().await;
    record_runtime_lifecycle_result("start-runtime-core", result.as_ref().map(|_| ()), reason);
    result
}

pub async fn stop_runtime_core(reason: &str) -> Result<()> {
    let result = CoreManager::global().stop_core().await;
    record_runtime_lifecycle_result("stop-runtime-core", result.as_ref().map(|_| ()), reason);
    result
}

pub async fn restart_runtime_core(reason: &str) -> Result<()> {
    let result = CoreManager::global().restart_core().await;
    record_runtime_lifecycle_result("restart-runtime-core", result.as_ref().map(|_| ()), reason);
    result
}

pub async fn update_runtime_config_checked(reason: &str) -> Result<()> {
    let result = CoreManager::global().update_config_checked().await;
    record_runtime_lifecycle_result("update-runtime-config-checked", result.as_ref().map(|_| ()), reason);
    result
}

pub async fn update_runtime_config_forced(reason: &str) -> Result<ValidationOutcome> {
    let result = CoreManager::global().update_config_forced().await;
    record_runtime_config_update_outcome("update-runtime-config-forced", &result, reason);
    result
}

pub async fn update_runtime_config_through_restart_boundary(force: bool, reason: &str) -> Result<ValidationOutcome> {
    let result = CoreManager::global().update_config_with_force(force).await;
    record_runtime_config_update_outcome("update-runtime-config-through-restart-boundary", &result, reason);
    result
}

pub async fn use_default_runtime_config(error_key: &str, error_msg: &str, reason: &str) -> Result<()> {
    let result = CoreManager::global().use_default_config(error_key, error_msg).await;
    record_runtime_lifecycle_result("use-default-runtime-config", result.as_ref().map(|_| ()), reason);
    result
}

pub async fn read_runtime_core_logs() -> Result<Vec<CompactString>> {
    CoreManager::global().get_clash_logs().await
}

pub fn read_runtime_running_mode() -> Arc<RunningMode> {
    CoreManager::global().get_running_mode()
}

pub fn runtime_is_not_running() -> bool {
    *read_runtime_running_mode() == RunningMode::NotRunning
}

fn record_runtime_config_update_outcome(kind: &str, result: &Result<ValidationOutcome>, reason: &str) {
    match result {
        Ok(outcome) if outcome.is_valid() => {
            runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                kind,
                true,
                None,
                Some(format!("reason={reason};outcome=valid")),
            );
        }
        Ok(outcome) => {
            runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                kind,
                false,
                Some(outcome.to_string()),
                Some(format!("reason={reason};outcome={outcome}")),
            );
        }
        Err(error) => {
            runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                kind,
                false,
                Some(error.to_string()),
                Some(format!("reason={reason}")),
            );
        }
    }
}

fn record_runtime_lifecycle_result<E: std::fmt::Display>(
    kind: &str,
    result: std::result::Result<(), &E>,
    reason: &str,
) {
    runtime_snapshot::record_and_persist_runtime_lifecycle_event(
        kind,
        result.is_ok(),
        result.err().map(ToString::to_string),
        Some(format!("reason={reason}")),
    );
}
