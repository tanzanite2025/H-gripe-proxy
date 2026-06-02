use super::CmdResult;
/**
 * HTTP 功能 Tauri 命令
 */
use crate::http::{BrowserFingerprint, BrowserTemplate, HeaderSanitizationConfig, HeaderSanitizer};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

static HEADER_SANITIZER: Lazy<Arc<RwLock<HeaderSanitizer>>> =
    Lazy::new(|| Arc::new(RwLock::new(HeaderSanitizer::new(HeaderSanitizationConfig::default()))));

/// 获取 HTTP 头净化配置
#[tauri::command]
pub fn header_sanitization_get_config() -> CmdResult<HeaderSanitizationConfig> {
    let sanitizer = HEADER_SANITIZER.read();
    Ok(sanitizer.config())
}

/// 更新 HTTP 头净化配置
#[tauri::command]
pub fn header_sanitization_update_config(config: HeaderSanitizationConfig) -> CmdResult<()> {
    let mut sanitizer = HEADER_SANITIZER.write();
    *sanitizer = HeaderSanitizer::new(config);
    log::info!("✅ HTTP 头净化配置已更新");
    Ok(())
}

/// 测试 HTTP 头净化效果
#[tauri::command]
pub fn header_sanitization_test(headers: HashMap<String, String>) -> Result<HashMap<String, String>, String> {
    let sanitizer = HEADER_SANITIZER.read();
    sanitizer
        .test_sanitization(headers)
        .map_err(|e| format!("测试净化失败: {}", e))
}

/// 获取浏览器模板列表
#[tauri::command]
pub fn header_sanitization_get_templates() -> Result<Vec<String>, String> {
    Ok(vec![
        "Chrome".to_string(),
        "Firefox".to_string(),
        "Safari".to_string(),
        "Edge".to_string(),
        "Custom".to_string(),
    ])
}

/// 获取指定浏览器模板的指纹
#[tauri::command]
pub fn header_sanitization_get_fingerprint(template: String) -> Result<BrowserFingerprint, String> {
    let browser_template = match template.as_str() {
        "Chrome" => BrowserTemplate::Chrome,
        "Firefox" => BrowserTemplate::Firefox,
        "Safari" => BrowserTemplate::Safari,
        "Edge" => BrowserTemplate::Edge,
        "Custom" => BrowserTemplate::Custom,
        _ => return Err(format!("未知的浏览器模板: {}", template)),
    };

    let config = HeaderSanitizationConfig {
        browser_template,
        ..Default::default()
    };
    let sanitizer = HeaderSanitizer::new(config);
    Ok(sanitizer.get_browser_fingerprint())
}
