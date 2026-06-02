/**
 * HTTP 模块
 *
 * 注意：header_sanitization 仅作为前端测试工具，不接入实际流量路径。
 * 真正的流量混淆由 Go 内核处理。
 *
 * 包含：
 * - header_sanitization: HTTP 头净化（前端测试工具）
 */
pub mod header_sanitization;

pub use header_sanitization::{BrowserFingerprint, BrowserTemplate, HeaderSanitizationConfig, HeaderSanitizer};
