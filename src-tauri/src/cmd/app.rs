use super::CmdResult;
use crate::core::autostart;
use crate::{
    app::{runtime, system, window},
    cmd::StringifyErr as _,
    feat,
    utils::dirs,
};
use smartstring::alias::String;
use tauri::Url;
#[cfg(debug_assertions)]
use tauri::{AppHandle, Manager as _};

fn parse_web_url(url: &str) -> CmdResult<Url> {
    let url = url.trim();
    if url.is_empty() {
        return Err("url is empty".into());
    }
    let parsed = Url::parse(url).stringify_err()?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("only http and https URLs are allowed".into());
    }
    if parsed.host_str().is_none() {
        return Err("invalid web url".into());
    }
    Ok(parsed)
}

/// 打开应用程序所在目录
#[tauri::command]
pub async fn open_app_dir() -> CmdResult<()> {
    system::open_app_dir().await.stringify_err()
}

/// 打开核心所在目录
#[tauri::command]
pub async fn open_core_dir() -> CmdResult<()> {
    system::open_core_dir().await.stringify_err()
}

/// 打开日志目录
#[tauri::command]
pub async fn open_logs_dir() -> CmdResult<()> {
    system::open_logs_dir().await.stringify_err()
}

/// 打开网页链接
#[tauri::command]
pub fn open_web_url(url: String) -> CmdResult<()> {
    let url = parse_web_url(&url)?;
    open::that(url.as_str()).stringify_err()
}

// TODO 后续可以为前端提供接口，当前作为托盘菜单使用
/// 打开 Verge 最新日志
#[tauri::command]
pub async fn open_app_log() -> CmdResult<()> {
    system::open_app_log().await.stringify_err()
}

// TODO 后续可以为前端提供接口，当前作为托盘菜单使用
/// 打开 Clash 最新日志
#[tauri::command]
pub async fn open_core_log() -> CmdResult<()> {
    system::open_core_log().await.stringify_err()
}

/// 打开/关闭开发者工具
#[cfg(debug_assertions)]
#[tauri::command]
pub fn open_devtools(app_handle: AppHandle) {
    if let Some(window) = app_handle.get_webview_window("main") {
        if !window.is_devtools_open() {
            window.open_devtools();
        } else {
            window.close_devtools();
        }
    }
}

/// 退出应用
#[tauri::command]
pub async fn exit_app() {
    window::quit().await;
}

/// 重启应用
#[tauri::command]
pub async fn restart_app() -> CmdResult<()> {
    runtime::restart_app().await;
    Ok(())
}

/// 获取便携版标识
#[tauri::command]
pub fn get_portable_flag() -> bool {
    *dirs::PORTABLE_FLAG.get().unwrap_or(&false)
}

/// 获取当前自启动状态
#[tauri::command]
pub fn get_auto_launch_status() -> CmdResult<bool> {
    autostart::get_launch_status().stringify_err()
}

/// 下载图标缓存
#[tauri::command]
pub async fn download_icon_cache(url: String, name: String) -> CmdResult<String> {
    feat::download_icon_cache(url.to_string(), name.to_string())
        .await
        .map(Into::into)
        .stringify_err()
}
