use super::{CmdResult, StringifyErr};
use crate::core::validate::ValidationOutcome;

#[tauri::command]
pub async fn read_china_rules_file() -> CmdResult<String> {
    crate::feat::read_china_rules_file().await.stringify_err()
}

#[tauri::command]
pub async fn save_china_rules_file(file_data: Option<String>) -> CmdResult<ValidationOutcome> {
    crate::feat::save_china_rules_file(file_data).await.stringify_err()
}
