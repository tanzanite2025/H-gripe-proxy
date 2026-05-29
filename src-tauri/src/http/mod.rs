/**
 * HTTP 模块
 * 
 * 包含：
 * - header_sanitization: HTTP 头净化
 */

pub mod header_sanitization;

pub use header_sanitization::{
    HeaderSanitizationConfig,
    HeaderSanitizer,
    BrowserTemplate,
    BrowserFingerprint,
};
