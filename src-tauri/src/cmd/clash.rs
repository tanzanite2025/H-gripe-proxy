use super::CmdResult;
use super::StringifyErr as _;
use crate::config::{ClashInfo, Config};
use crate::feat;
use compact_str::CompactString;
use serde_yaml_ng::Mapping;
use smartstring::alias::String;

/// 复制Clash环境变量
#[tauri::command]
pub async fn copy_clash_env() -> CmdResult {
    feat::copy_clash_env().await;
    Ok(())
}

/// 获取Clash信息
#[tauri::command]
pub async fn get_clash_info() -> CmdResult<ClashInfo> {
    Ok(Config::clash().await.data_arc().get_client_info())
}

/// 修改Clash配置
#[tauri::command]
pub async fn patch_clash_config(payload: Mapping) -> CmdResult {
    feat::patch_clash(&payload).await.stringify_err()
}

/// 修改Clash模式
#[tauri::command]
pub async fn patch_clash_mode(payload: String) -> CmdResult {
    feat::change_clash_mode(payload.into()).await;
    Ok(())
}

/// 切换Clash核心
#[tauri::command]
pub async fn change_clash_core(clash_core: String) -> CmdResult<Option<String>> {
    feat::change_clash_core(&clash_core).await.stringify_err()
}

/// 启动核心
#[tauri::command]
pub async fn start_core() -> CmdResult {
    feat::start_core().await.stringify_err()
}

/// 关闭核心
#[tauri::command]
pub async fn stop_core() -> CmdResult {
    feat::stop_core().await.stringify_err()
}

/// 重启核心
#[tauri::command]
pub async fn restart_core() -> CmdResult {
    feat::restart_core().await.stringify_err()
}

/// 测试URL延迟
#[tauri::command]
pub async fn test_delay(url: String) -> CmdResult<u32> {
    let result = match feat::test_delay(url.into()).await {
        Ok(delay) => delay,
        Err(e) => {
            clash_verge_logging::logging!(error, clash_verge_logging::Type::Cmd, "{}", e);
            10000u32
        }
    };
    Ok(result)
}

/// 应用或撤销DNS配置
#[tauri::command]
pub async fn apply_dns_config(apply: bool) -> CmdResult {
    feat::apply_dns_config(apply).await.stringify_err()
}

#[tauri::command]
pub async fn get_clash_logs() -> CmdResult<Vec<CompactString>> {
    Ok(feat::get_clash_logs().await)
}
