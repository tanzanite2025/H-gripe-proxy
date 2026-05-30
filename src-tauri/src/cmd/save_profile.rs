use super::CmdResult;
use super::StringifyErr as _;
use crate::core::validate::ValidationOutcome;
use smartstring::alias::String;

/// 保存profiles的配置
#[tauri::command]
pub async fn save_profile_file(index: String, file_data: Option<String>) -> CmdResult<ValidationOutcome> {
    crate::feat::save_profile_file(&index, file_data).await.stringify_err()
}
