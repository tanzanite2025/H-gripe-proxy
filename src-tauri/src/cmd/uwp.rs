use crate::cmd::CmdResult;
use crate::cmd::StringifyErr as _;
use crate::core::win_uwp;

/// Command exposed to Tauri
#[tauri::command]
pub async fn invoke_uwp_tool() -> CmdResult {
    win_uwp::invoke_uwptools().stringify_err()
}
