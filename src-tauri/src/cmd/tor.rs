use crate::cmd::{CmdResult, StringifyErr};
use crate::core::tor_runtime::{TorRuntimeStatus, build_tor_runtime_status};

#[tauri::command]
pub async fn get_tor_status() -> CmdResult<TorRuntimeStatus> {
    build_tor_runtime_status().await.stringify_err()
}

#[tauri::command]
pub async fn test_tor_connection() -> CmdResult<TorRuntimeStatus> {
    build_tor_runtime_status().await.stringify_err()
}
