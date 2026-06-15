use super::{CmdResult, StringifyErr};
use crate::core::config_explain::{ConfigDiffReport, explain_config_diff as explain_config_diff_impl};
use smartstring::alias::String;

#[tauri::command]
pub async fn explain_config_diff(before_yaml: String, after_yaml: String) -> CmdResult<ConfigDiffReport> {
    explain_config_diff_impl(&before_yaml, &after_yaml).stringify_err()
}
