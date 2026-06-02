/**
 * HTTP 头净化模块（前端测试工具）
 *
 * 注意：此模块仅作为前端面板的测试/演示工具，不接入实际流量路径。
 * 真正的流量混淆由 Go 内核（mihomo）的 ObfuscatedConn 在连接层执行，
 * 包括 TLS 指纹轮换、行为保护等，效果远超静态 header 替换。
 *
 * 功能：
 * 1. 代理头清除 - 清除代理特征头
 * 2. 浏览器指纹伪造 - 伪造真实浏览器指纹
 * 3. 头部顺序规范化 - 规范化头部顺序
 */
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 代理特征头列表
///
/// 这些头部会暴露代理的存在，需要清除
pub const PROXY_HEADERS: &[&str] = &[
    "X-Forwarded-For",
    "X-Real-IP",
    "Via",
    "Proxy-Connection",
    "X-Proxy-ID",
    "Forwarded",
    "X-Forwarded-Host",
    "X-Forwarded-Proto",
    "X-Forwarded-Server",
    "X-Forwarded-Port",
    "X-Original-URL",
    "X-Rewrite-URL",
    "X-ProxyUser-Ip",
    "Client-IP",
    "True-Client-IP",
    "CF-Connecting-IP",
    "X-Client-IP",
    "X-Host",
    "Proxy-Authorization",
];

/// HTTP 头净化配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderSanitizationConfig {
    /// 启用净化
    pub enabled: bool,
    /// 清除代理头
    pub remove_proxy_headers: bool,
    /// 自定义要清除的头
    pub custom_headers_to_remove: Vec<String>,
    /// 伪造 User-Agent
    pub forge_user_agent: bool,
    /// 浏览器模板
    pub browser_template: BrowserTemplate,
    /// 自定义 User-Agent
    pub custom_user_agent: Option<String>,
    /// 规范化 Accept 头
    pub normalize_accept: bool,
    /// 规范化头部顺序
    pub normalize_header_order: bool,
}

impl Default for HeaderSanitizationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            remove_proxy_headers: true,
            custom_headers_to_remove: Vec::new(),
            forge_user_agent: true,
            browser_template: BrowserTemplate::Chrome,
            custom_user_agent: None,
            normalize_accept: true,
            normalize_header_order: true,
        }
    }
}

/// 浏览器模板
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BrowserTemplate {
    Chrome,
    Firefox,
    Safari,
    Edge,
    Custom,
}

impl BrowserTemplate {
    pub fn as_str(&self) -> &str {
        match self {
            BrowserTemplate::Chrome => "Chrome",
            BrowserTemplate::Firefox => "Firefox",
            BrowserTemplate::Safari => "Safari",
            BrowserTemplate::Edge => "Edge",
            BrowserTemplate::Custom => "Custom",
        }
    }
}

/// 浏览器指纹
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserFingerprint {
    pub user_agent: String,
    pub accept: String,
    pub accept_language: String,
    pub accept_encoding: String,
    pub header_order: Vec<String>,
}

/// HTTP 头净化器
pub struct HeaderSanitizer {
    config: HeaderSanitizationConfig,
}

impl HeaderSanitizer {
    /// 创建新的头净化器
    pub fn new(config: HeaderSanitizationConfig) -> Self {
        Self { config }
    }

    /// 获取当前配置
    pub fn config(&self) -> HeaderSanitizationConfig {
        self.config.clone()
    }

    /// 净化 HTTP 头
    ///
    /// 执行以下操作：
    /// 1. 清除代理特征头
    /// 2. 伪造浏览器指纹
    /// 3. 规范化头部顺序
    pub fn sanitize(&self, headers: &mut HashMap<String, String>) -> Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        // 1. 清除代理头
        if self.config.remove_proxy_headers {
            self.remove_proxy_headers(headers);
        }

        // 2. 伪造浏览器指纹
        if self.config.forge_user_agent {
            self.apply_browser_fingerprint(headers)?;
        }

        // 3. 规范化头部顺序（返回新的有序 HashMap）
        if self.config.normalize_header_order {
            let fingerprint = self.get_browser_fingerprint();
            *headers = self.normalize_header_order(headers, &fingerprint.header_order);
        }

        Ok(())
    }

    /// 清除代理特征头
    fn remove_proxy_headers(&self, headers: &mut HashMap<String, String>) {
        // 清除标准代理头
        for header in PROXY_HEADERS {
            headers.remove(*header);
            // 也尝试小写版本
            headers.remove(&header.to_lowercase());
        }

        // 清除自定义代理头
        for header in &self.config.custom_headers_to_remove {
            headers.remove(header);
            headers.remove(&header.to_lowercase());
        }

        log::trace!("Removed proxy headers");
    }

    /// 应用浏览器指纹
    fn apply_browser_fingerprint(&self, headers: &mut HashMap<String, String>) -> Result<()> {
        let fingerprint = self.get_browser_fingerprint();

        // 设置 User-Agent
        headers.insert("User-Agent".to_string(), fingerprint.user_agent);

        // 设置 Accept 系列头
        if self.config.normalize_accept {
            headers.insert("Accept".to_string(), fingerprint.accept);
            headers.insert("Accept-Language".to_string(), fingerprint.accept_language);
            headers.insert("Accept-Encoding".to_string(), fingerprint.accept_encoding);
        }

        // 设置其他常见头
        headers.insert("DNT".to_string(), "1".to_string());
        headers.insert("Upgrade-Insecure-Requests".to_string(), "1".to_string());

        log::trace!("Applied browser fingerprint: {}", self.config.browser_template.as_str());
        Ok(())
    }

    /// 获取浏览器指纹
    pub fn get_browser_fingerprint(&self) -> BrowserFingerprint {
        // 如果有自定义 User-Agent，使用自定义
        if let Some(custom_ua) = &self.config.custom_user_agent {
            if self.config.browser_template == BrowserTemplate::Custom {
                return BrowserFingerprint {
                    user_agent: custom_ua.clone(),
                    accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".to_string(),
                    accept_language: "en-US,en;q=0.9".to_string(),
                    accept_encoding: "gzip, deflate, br".to_string(),
                    header_order: vec![
                        "Host".to_string(),
                        "User-Agent".to_string(),
                        "Accept".to_string(),
                        "Accept-Language".to_string(),
                        "Accept-Encoding".to_string(),
                    ],
                };
            }
        }

        match self.config.browser_template {
            BrowserTemplate::Chrome => BrowserFingerprint {
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
                accept: "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".to_string(),
                accept_language: "en-US,en;q=0.9".to_string(),
                accept_encoding: "gzip, deflate, br".to_string(),
                header_order: vec![
                    "Host".to_string(),
                    "Connection".to_string(),
                    "Cache-Control".to_string(),
                    "sec-ch-ua".to_string(),
                    "sec-ch-ua-mobile".to_string(),
                    "sec-ch-ua-platform".to_string(),
                    "Upgrade-Insecure-Requests".to_string(),
                    "User-Agent".to_string(),
                    "Accept".to_string(),
                    "Sec-Fetch-Site".to_string(),
                    "Sec-Fetch-Mode".to_string(),
                    "Sec-Fetch-User".to_string(),
                    "Sec-Fetch-Dest".to_string(),
                    "Accept-Encoding".to_string(),
                    "Accept-Language".to_string(),
                ],
            },
            BrowserTemplate::Firefox => BrowserFingerprint {
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0".to_string(),
                accept: "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8".to_string(),
                accept_language: "en-US,en;q=0.5".to_string(),
                accept_encoding: "gzip, deflate, br".to_string(),
                header_order: vec![
                    "Host".to_string(),
                    "User-Agent".to_string(),
                    "Accept".to_string(),
                    "Accept-Language".to_string(),
                    "Accept-Encoding".to_string(),
                    "Connection".to_string(),
                    "Upgrade-Insecure-Requests".to_string(),
                    "Sec-Fetch-Dest".to_string(),
                    "Sec-Fetch-Mode".to_string(),
                    "Sec-Fetch-Site".to_string(),
                    "Sec-Fetch-User".to_string(),
                ],
            },
            BrowserTemplate::Safari => BrowserFingerprint {
                user_agent: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15".to_string(),
                accept: "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8".to_string(),
                accept_language: "en-US,en;q=0.9".to_string(),
                accept_encoding: "gzip, deflate, br".to_string(),
                header_order: vec![
                    "Host".to_string(),
                    "Accept".to_string(),
                    "User-Agent".to_string(),
                    "Accept-Language".to_string(),
                    "Accept-Encoding".to_string(),
                    "Connection".to_string(),
                ],
            },
            BrowserTemplate::Edge => BrowserFingerprint {
                user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0".to_string(),
                accept: "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".to_string(),
                accept_language: "en-US,en;q=0.9".to_string(),
                accept_encoding: "gzip, deflate, br".to_string(),
                header_order: vec![
                    "Host".to_string(),
                    "Connection".to_string(),
                    "Cache-Control".to_string(),
                    "sec-ch-ua".to_string(),
                    "sec-ch-ua-mobile".to_string(),
                    "sec-ch-ua-platform".to_string(),
                    "Upgrade-Insecure-Requests".to_string(),
                    "User-Agent".to_string(),
                    "Accept".to_string(),
                    "Sec-Fetch-Site".to_string(),
                    "Sec-Fetch-Mode".to_string(),
                    "Sec-Fetch-User".to_string(),
                    "Sec-Fetch-Dest".to_string(),
                    "Accept-Encoding".to_string(),
                    "Accept-Language".to_string(),
                ],
            },
            BrowserTemplate::Custom => {
                // 使用 Chrome 作为默认
                self.get_browser_fingerprint()
            }
        }
    }

    /// 规范化头部顺序
    fn normalize_header_order(&self, headers: &HashMap<String, String>, order: &[String]) -> HashMap<String, String> {
        let mut ordered_headers = HashMap::new();

        // 按照指定顺序添加头部
        for header_name in order {
            if let Some(value) = headers.get(header_name) {
                ordered_headers.insert(header_name.clone(), value.clone());
            }
        }

        // 添加剩余的头部
        for (name, value) in headers.iter() {
            if !ordered_headers.contains_key(name) {
                ordered_headers.insert(name.clone(), value.clone());
            }
        }

        ordered_headers
    }

    /// 测试净化效果
    pub fn test_sanitization(&self, headers: HashMap<String, String>) -> Result<HashMap<String, String>> {
        let mut test_headers = headers;
        self.sanitize(&mut test_headers)?;
        Ok(test_headers)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_proxy_headers() {
        let config = HeaderSanitizationConfig::default();
        let sanitizer = HeaderSanitizer::new(config);

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "Test".to_string());
        headers.insert("X-Forwarded-For".to_string(), "1.2.3.4".to_string());
        headers.insert("Via".to_string(), "proxy".to_string());
        headers.insert("Proxy-Connection".to_string(), "keep-alive".to_string());

        sanitizer.remove_proxy_headers(&mut headers);

        assert!(headers.contains_key("User-Agent"));
        assert!(!headers.contains_key("X-Forwarded-For"));
        assert!(!headers.contains_key("Via"));
        assert!(!headers.contains_key("Proxy-Connection"));
    }

    #[test]
    fn test_remove_custom_headers() {
        let config = HeaderSanitizationConfig {
            custom_headers_to_remove: vec!["X-Custom-Header".to_string()],
            ..Default::default()
        };
        let sanitizer = HeaderSanitizer::new(config);

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "Test".to_string());
        headers.insert("X-Custom-Header".to_string(), "value".to_string());

        sanitizer.remove_proxy_headers(&mut headers);

        assert!(headers.contains_key("User-Agent"));
        assert!(!headers.contains_key("X-Custom-Header"));
    }

    #[test]
    fn test_apply_browser_fingerprint() {
        let config = HeaderSanitizationConfig::default();
        let sanitizer = HeaderSanitizer::new(config);

        let mut headers = HashMap::new();
        sanitizer.apply_browser_fingerprint(&mut headers).unwrap();

        assert!(headers.contains_key("User-Agent"));
        assert!(headers.contains_key("Accept"));
        assert!(headers.contains_key("Accept-Language"));
        assert!(headers.contains_key("Accept-Encoding"));
        assert!(headers.contains_key("DNT"));
        assert!(headers.contains_key("Upgrade-Insecure-Requests"));
    }

    #[test]
    fn test_get_browser_fingerprint_chrome() {
        let config = HeaderSanitizationConfig {
            browser_template: BrowserTemplate::Chrome,
            ..Default::default()
        };
        let sanitizer = HeaderSanitizer::new(config);

        let fingerprint = sanitizer.get_browser_fingerprint();
        assert!(fingerprint.user_agent.contains("Chrome"));
        assert!(fingerprint.accept.contains("image/avif"));
    }

    #[test]
    fn test_get_browser_fingerprint_firefox() {
        let config = HeaderSanitizationConfig {
            browser_template: BrowserTemplate::Firefox,
            ..Default::default()
        };
        let sanitizer = HeaderSanitizer::new(config);

        let fingerprint = sanitizer.get_browser_fingerprint();
        assert!(fingerprint.user_agent.contains("Firefox"));
        assert_eq!(fingerprint.accept_language, "en-US,en;q=0.5");
    }

    #[test]
    fn test_get_browser_fingerprint_safari() {
        let config = HeaderSanitizationConfig {
            browser_template: BrowserTemplate::Safari,
            ..Default::default()
        };
        let sanitizer = HeaderSanitizer::new(config);

        let fingerprint = sanitizer.get_browser_fingerprint();
        assert!(fingerprint.user_agent.contains("Safari"));
        assert!(fingerprint.user_agent.contains("Macintosh"));
    }

    #[test]
    fn test_custom_user_agent() {
        let config = HeaderSanitizationConfig {
            browser_template: BrowserTemplate::Custom,
            custom_user_agent: Some("Custom UA".to_string()),
            ..Default::default()
        };
        let sanitizer = HeaderSanitizer::new(config);

        let fingerprint = sanitizer.get_browser_fingerprint();
        assert_eq!(fingerprint.user_agent, "Custom UA");
    }

    #[test]
    fn test_normalize_header_order() {
        let config = HeaderSanitizationConfig::default();
        let sanitizer = HeaderSanitizer::new(config);

        let mut headers = HashMap::new();
        headers.insert("Accept".to_string(), "text/html".to_string());
        headers.insert("Host".to_string(), "example.com".to_string());
        headers.insert("User-Agent".to_string(), "Test".to_string());

        let order = vec!["Host".to_string(), "User-Agent".to_string(), "Accept".to_string()];

        let ordered = sanitizer.normalize_header_order(&headers, &order);

        // 验证所有头部都存在
        assert_eq!(ordered.len(), 3);
        assert!(ordered.contains_key("Host"));
        assert!(ordered.contains_key("User-Agent"));
        assert!(ordered.contains_key("Accept"));
    }

    #[test]
    fn test_full_sanitization() {
        let config = HeaderSanitizationConfig::default();
        let sanitizer = HeaderSanitizer::new(config);

        let mut headers = HashMap::new();
        headers.insert("User-Agent".to_string(), "Old UA".to_string());
        headers.insert("X-Forwarded-For".to_string(), "1.2.3.4".to_string());
        headers.insert("Via".to_string(), "proxy".to_string());

        sanitizer.sanitize(&mut headers).unwrap();

        // 代理头应该被删除
        assert!(!headers.contains_key("X-Forwarded-For"));
        assert!(!headers.contains_key("Via"));

        // User-Agent 应该被替换
        assert!(headers.get("User-Agent").unwrap().contains("Chrome"));

        // 应该添加新的头部
        assert!(headers.contains_key("Accept"));
        assert!(headers.contains_key("DNT"));
    }

    #[test]
    fn test_disabled_sanitization() {
        let config = HeaderSanitizationConfig {
            enabled: false,
            ..Default::default()
        };
        let sanitizer = HeaderSanitizer::new(config);

        let mut headers = HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "1.2.3.4".to_string());

        sanitizer.sanitize(&mut headers).unwrap();

        // 禁用时不应该修改头部
        assert!(headers.contains_key("X-Forwarded-For"));
    }

    #[test]
    fn test_test_sanitization() {
        let config = HeaderSanitizationConfig::default();
        let sanitizer = HeaderSanitizer::new(config);

        let mut headers = HashMap::new();
        headers.insert("X-Forwarded-For".to_string(), "1.2.3.4".to_string());
        headers.insert("User-Agent".to_string(), "Old UA".to_string());

        let result = sanitizer.test_sanitization(headers.clone()).unwrap();

        // 原始 headers 不应该被修改
        assert!(headers.contains_key("X-Forwarded-For"));

        // 结果应该被净化
        assert!(!result.contains_key("X-Forwarded-For"));
        assert!(result.get("User-Agent").unwrap().contains("Chrome"));
    }
}
