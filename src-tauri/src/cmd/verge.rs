use super::CmdResult;
use crate::{app::config as app_config, cmd::StringifyErr as _, config::IVerge, utils::init};
use clash_verge_draft::SharedDraft;

/// 获取Verge配置
#[tauri::command]
pub async fn get_verge_config() -> CmdResult<SharedDraft<IVerge>> {
    app_config::fetch_verge_config().await.stringify_err()
}

/// 修改Verge配置
#[tauri::command]
pub async fn patch_verge_config(payload: IVerge) -> CmdResult {
    app_config::patch_verge(&payload, false).await.stringify_err()
}

#[tauri::command]
pub async fn authorize_startup_script(path: String) -> CmdResult {
    init::authorize_startup_script(path).await.stringify_err()
}

#[tauri::command]
pub async fn clear_startup_script_authorization() -> CmdResult {
    init::clear_startup_script_authorization().await.stringify_err()
}
