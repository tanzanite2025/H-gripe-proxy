use super::CmdResult;
use super::StringifyErr as _;
use crate::core::mihomo_runtime_guard;
use crate::utils::dirs;
use crate::{
    app::{config as app_config, runtime},
    config::{ClashInfo, Config},
};
use compact_str::CompactString;
use serde::Serialize;
use serde_yaml_ng::Mapping;
use smartstring::alias::String;
use std::time::UNIX_EPOCH;

/// 复制Clash环境变量
#[tauri::command]
pub async fn copy_clash_env() -> CmdResult {
    runtime::copy_clash_env().await;
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
    app_config::patch_clash(&payload).await.stringify_err()
}

/// 修改Clash模式
#[tauri::command]
pub async fn patch_clash_mode(payload: String) -> CmdResult {
    let mode = payload.parse().stringify_err()?;
    runtime::change_clash_mode(mode).await.stringify_err()
}

/// 启动核心
#[tauri::command]
pub async fn start_core() -> CmdResult {
    runtime::start_core().await.stringify_err()
}

/// 关闭核心
#[tauri::command]
pub async fn stop_core() -> CmdResult {
    runtime::stop_core().await.stringify_err()
}

/// 重启核心
#[tauri::command]
pub async fn restart_core() -> CmdResult {
    runtime::restart_core().await.stringify_err()
}

/// Ensure Mihomo core and IPC are ready for frontend/runtime operations
#[tauri::command]
pub async fn ensure_mihomo_core_ready() -> CmdResult {
    mihomo_runtime_guard::ensure_mihomo_core_ready().await.stringify_err()
}

/// 测试URL延迟
#[tauri::command]
pub async fn test_delay(url: String) -> CmdResult<u32> {
    let result = match runtime::test_delay(url.into()).await {
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
    runtime::apply_dns_config(apply).await.stringify_err()
}

#[tauri::command]
pub async fn get_clash_logs() -> CmdResult<Vec<CompactString>> {
    Ok(runtime::get_clash_logs().await)
}

#[tauri::command]
pub async fn clear_logs() -> CmdResult {
    runtime::clear_clash_logs().await;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct GeoDataUpdateTime {
    pub mmdb: Option<u64>,
    pub geoip: Option<u64>,
    pub asn: Option<u64>,
    pub city: Option<u64>,
    pub geosite: Option<u64>,
}

/// 获取 GeoData 文件最后更新时间 (unix timestamp ms)
#[tauri::command]
pub async fn get_geo_data_update_time() -> CmdResult<GeoDataUpdateTime> {
    let app_dir = dirs::app_home_dir().stringify_err()?;
    let get_mtime = |name: &str| -> Option<u64> {
        std::fs::metadata(app_dir.join(name))
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
    };
    Ok(GeoDataUpdateTime {
        mmdb: get_mtime("GeoLite2-City.mmdb"),
        geoip: get_mtime("geoip.dat"),
        asn: get_mtime("GeoLite2-ASN.mmdb"),
        city: get_mtime("GeoLite2-City.mmdb"),
        geosite: get_mtime("geosite.dat"),
    })
}
