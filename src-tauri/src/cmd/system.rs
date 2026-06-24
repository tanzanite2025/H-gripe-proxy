use std::sync::Arc;

use crate::core::{manager::RunningMode, runtime_lifecycle};

/// 获取当前内核运行模式
#[tauri::command]
pub async fn get_running_mode() -> Result<Arc<RunningMode>, String> {
    Ok(runtime_lifecycle::read_runtime_running_mode())
}
