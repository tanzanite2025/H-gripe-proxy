/**
 * 时区/NTP 主动伪装模块
 *
 * 功能：
 * 1. 根据出口节点所在区域自动选择对应区域的 NTP 服务器
 * 2. 注入 Mihomo ntp: 配置，确保系统时钟与出口区域同步
 * 3. 防止时区/时间偏差泄露用户真实地理位置
 *
 * 原理：
 * - 当出口节点在日本但系统时钟显示 UTC+8（中国），
 *   目标服务器可通过时间偏移推断用户不在日本
 * - Mihomo 内置 NTP 服务可通过代理同步时间，
 *   选择出口区域附近的 NTP 服务器可减少时钟偏差
 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use once_cell::sync::Lazy;
use parking_lot::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObservedEgressRegion {
    pub country_code: String,
    pub timezone: String,
    pub source: String,
    pub updated_at_ms: u64,
}

static OBSERVED_EGRESS_REGION: Lazy<RwLock<Option<ObservedEgressRegion>>> = Lazy::new(|| RwLock::new(None));

// ── 配置 ──────────────────────────────────────────────────────────

/// 时区/NTP 伪装配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimezoneSpoofConfig {
    /// 启用时区伪装
    #[serde(default)]
    pub enabled: bool,
    /// NTP 策略
    #[serde(default)]
    pub ntp_strategy: NtpStrategy,
    /// 手动指定 NTP 服务器（仅 Manual 模式）
    #[serde(default)]
    pub manual_ntp_server: Option<String>,
    /// NTP 同步间隔（分钟）
    #[serde(default = "default_ntp_interval")]
    pub ntp_interval_min: u32,
    /// 是否写入系统时间（需要管理员权限）
    #[serde(default)]
    pub write_to_system: bool,
    /// 用于 NTP 同步的代理组名（空则走 DIRECT）
    #[serde(default)]
    pub dialer_proxy: Option<String>,
}

/// NTP 服务器选择策略
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum NtpStrategy {
    /// 根据出口节点区域自动选择
    #[default]
    Auto,
    /// 手动指定
    Manual,
    /// 禁用 NTP（仅伪装 HTTP 头）
    Disabled,
}

impl Default for TimezoneSpoofConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ntp_strategy: NtpStrategy::Auto,
            manual_ntp_server: None,
            ntp_interval_min: default_ntp_interval(),
            write_to_system: false,
            dialer_proxy: None,
        }
    }
}

fn default_ntp_interval() -> u32 {
    30
}

pub fn remember_observed_egress_region(country_code: Option<&str>, timezone: Option<&str>, source: &str) {
    let Some(country_code) = normalize_country_code(country_code) else {
        return;
    };

    let timezone = timezone
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| country_to_timezone(&country_code).to_string());

    if timezone == "UTC" {
        return;
    }

    *OBSERVED_EGRESS_REGION.write() = Some(ObservedEgressRegion {
        country_code,
        timezone,
        source: source.trim().to_string(),
        updated_at_ms: now_ms(),
    });
}

pub fn get_observed_egress_region() -> Option<ObservedEgressRegion> {
    OBSERVED_EGRESS_REGION.read().clone()
}

pub fn get_fresh_observed_egress_region(max_age_ms: u64) -> Option<ObservedEgressRegion> {
    let observed = get_observed_egress_region()?;
    (now_ms().saturating_sub(observed.updated_at_ms) <= max_age_ms).then_some(observed)
}

fn normalize_country_code(country_code: Option<&str>) -> Option<String> {
    country_code
        .map(str::trim)
        .filter(|value| !value.is_empty() && !value.eq_ignore_ascii_case("unknown"))
        .map(str::to_ascii_uppercase)
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// ── 区域 → NTP 服务器映射 ─────────────────────────────────────────

/// 国家/区域代码 → 推荐的 NTP 服务器池
///
/// 使用各国的国家代码级 NTP 池（pool.ntp.org 体系）
fn region_ntp_map() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    // 东亚
    m.insert("CN", "cn.pool.ntp.org");
    m.insert("JP", "jp.pool.ntp.org");
    m.insert("KR", "kr.pool.ntp.org");
    m.insert("TW", "tw.pool.ntp.org");
    m.insert("HK", "hk.pool.ntp.org");
    // 东南亚
    m.insert("SG", "sg.pool.ntp.org");
    m.insert("MY", "my.pool.ntp.org");
    m.insert("TH", "th.pool.ntp.org");
    m.insert("PH", "ph.pool.ntp.org");
    m.insert("VN", "vn.pool.ntp.org");
    m.insert("ID", "id.pool.ntp.org");
    // 南亚
    m.insert("IN", "in.pool.ntp.org");
    // 中东
    m.insert("AE", "ae.pool.ntp.org");
    m.insert("IL", "il.pool.ntp.org");
    // 欧洲
    m.insert("GB", "uk.pool.ntp.org");
    m.insert("DE", "de.pool.ntp.org");
    m.insert("FR", "fr.pool.ntp.org");
    m.insert("NL", "nl.pool.ntp.org");
    m.insert("RU", "ru.pool.ntp.org");
    m.insert("SE", "se.pool.ntp.org");
    m.insert("CH", "ch.pool.ntp.org");
    m.insert("UA", "ua.pool.ntp.org");
    m.insert("PL", "pl.pool.ntp.org");
    m.insert("IT", "it.pool.ntp.org");
    m.insert("ES", "es.pool.ntp.org");
    // 北美
    m.insert("US", "us.pool.ntp.org");
    m.insert("CA", "ca.pool.ntp.org");
    m.insert("MX", "mx.pool.ntp.org");
    // 南美
    m.insert("BR", "br.pool.ntp.org");
    m.insert("AR", "ar.pool.ntp.org");
    m.insert("CL", "cl.pool.ntp.org");
    // 大洋洲
    m.insert("AU", "au.pool.ntp.org");
    m.insert("NZ", "nz.pool.ntp.org");
    // 非洲
    m.insert("ZA", "za.pool.ntp.org");
    m
}

/// 根据国家代码选择 NTP 服务器
///
/// 优先使用国家级池，回退到洲级池，最终回退到全球池
pub fn select_ntp_server(country_code: &str) -> String {
    let map = region_ntp_map();

    // 1. 精确匹配国家级
    if let Some(server) = map.get(country_code) {
        return server.to_string();
    }

    // 2. 洲级回退
    let continent_server = match country_code {
        "CN" | "JP" | "KR" | "TW" | "HK" | "MN" => "asia.pool.ntp.org",
        "SG" | "MY" | "TH" | "PH" | "VN" | "ID" | "MM" | "KH" | "LA" => "asia.pool.ntp.org",
        "IN" | "PK" | "BD" | "LK" | "NP" => "asia.pool.ntp.org",
        "AE" | "IL" | "SA" | "QA" | "KW" | "BH" | "OM" | "IQ" | "IR" => "asia.pool.ntp.org",
        "GB" | "DE" | "FR" | "NL" | "RU" | "SE" | "CH" | "UA" | "PL" | "IT" | "ES" | "PT" | "NO" | "FI" | "DK"
        | "AT" | "BE" | "IE" | "CZ" | "RO" | "HU" | "GR" | "BG" | "HR" | "RS" | "SK" | "SI" | "LT" | "LV" | "EE" => {
            "europe.pool.ntp.org"
        }
        "US" | "CA" | "MX" | "GT" | "CR" | "PA" | "CU" | "JM" => "north-america.pool.ntp.org",
        "BR" | "AR" | "CL" | "CO" | "PE" | "VE" | "EC" | "UY" | "PY" | "BO" => "south-america.pool.ntp.org",
        "AU" | "NZ" | "PG" | "FJ" => "oceania.pool.ntp.org",
        "ZA" | "NG" | "EG" | "KE" | "GH" | "TZ" | "ET" | "MA" | "TN" | "DZ" => "africa.pool.ntp.org",
        _ => "",
    };

    if !continent_server.is_empty() {
        return continent_server.to_string();
    }

    // 3. 全球回退
    "pool.ntp.org".to_string()
}

/// 根据国家代码推断 IANA 时区名
///
/// 用于 UI 显示和 HTTP Accept-Language 生成
pub fn country_to_timezone(country_code: &str) -> &'static str {
    match country_code {
        "CN" => "Asia/Shanghai",
        "JP" => "Asia/Tokyo",
        "KR" => "Asia/Seoul",
        "TW" => "Asia/Taipei",
        "HK" => "Asia/Hong_Kong",
        "SG" => "Asia/Singapore",
        "MY" => "Asia/Kuala_Lumpur",
        "TH" => "Asia/Bangkok",
        "PH" => "Asia/Manila",
        "VN" => "Asia/Ho_Chi_Minh",
        "ID" => "Asia/Jakarta",
        "IN" => "Asia/Kolkata",
        "AE" => "Asia/Dubai",
        "IL" => "Asia/Jerusalem",
        "GB" => "Europe/London",
        "DE" => "Europe/Berlin",
        "FR" => "Europe/Paris",
        "NL" => "Europe/Amsterdam",
        "RU" => "Europe/Moscow",
        "SE" => "Europe/Stockholm",
        "CH" => "Europe/Zurich",
        "UA" => "Europe/Kyiv",
        "PL" => "Europe/Warsaw",
        "IT" => "Europe/Rome",
        "ES" => "Europe/Madrid",
        "US" => "America/New_York",
        "CA" => "America/Toronto",
        "MX" => "America/Mexico_City",
        "BR" => "America/Sao_Paulo",
        "AR" => "America/Buenos_Aires",
        "AU" => "Australia/Sydney",
        "NZ" => "Pacific/Auckland",
        "ZA" => "Africa/Johannesburg",
        _ => "UTC",
    }
}

/// 根据时区生成 Accept-Language风格的 locale 标签
pub fn timezone_to_locale(timezone: &str) -> String {
    match timezone {
        "Asia/Shanghai" => "zh-CN,zh;q=0.9,en;q=0.8".to_string(),
        "Asia/Tokyo" => "ja-JP,ja;q=0.9,en;q=0.8".to_string(),
        "Asia/Seoul" => "ko-KR,ko;q=0.9,en;q=0.8".to_string(),
        "Asia/Taipei" => "zh-TW,zh;q=0.9,en;q=0.8".to_string(),
        "Asia/Hong_Kong" => "zh-HK,zh;q=0.9,en;q=0.8".to_string(),
        "Asia/Singapore" => "en-SG,en;q=0.9,zh;q=0.8".to_string(),
        "Asia/Bangkok" => "th-TH,th;q=0.9,en;q=0.8".to_string(),
        "Asia/Kolkata" => "en-IN,en;q=0.9,hi;q=0.8".to_string(),
        "Asia/Dubai" => "ar-AE,ar;q=0.9,en;q=0.8".to_string(),
        "Europe/London" => "en-GB,en;q=0.9".to_string(),
        "Europe/Berlin" => "de-DE,de;q=0.9,en;q=0.8".to_string(),
        "Europe/Paris" => "fr-FR,fr;q=0.9,en;q=0.8".to_string(),
        "Europe/Moscow" => "ru-RU,ru;q=0.9,en;q=0.8".to_string(),
        "America/New_York" => "en-US,en;q=0.9".to_string(),
        "America/Toronto" => "en-CA,en;q=0.9,fr;q=0.8".to_string(),
        "America/Sao_Paulo" => "pt-BR,pt;q=0.9,en;q=0.8".to_string(),
        "Australia/Sydney" => "en-AU,en;q=0.9".to_string(),
        _ => "en-US,en;q=0.9".to_string(),
    }
}

// ── 测试 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_select_ntp_server_known_country() {
        assert_eq!(select_ntp_server("JP"), "jp.pool.ntp.org");
        assert_eq!(select_ntp_server("US"), "us.pool.ntp.org");
        assert_eq!(select_ntp_server("DE"), "de.pool.ntp.org");
    }

    #[test]
    fn test_select_ntp_server_unknown_country_fallback_continent() {
        // 蒙古不在国家级映射中，但应回退到亚洲池
        assert_eq!(select_ntp_server("MN"), "asia.pool.ntp.org");
    }

    #[test]
    fn test_select_ntp_server_unknown_fallback_global() {
        // 完全未知的国家代码回退到全球池
        assert_eq!(select_ntp_server("XX"), "pool.ntp.org");
    }

    #[test]
    fn test_country_to_timezone() {
        assert_eq!(country_to_timezone("JP"), "Asia/Tokyo");
        assert_eq!(country_to_timezone("US"), "America/New_York");
        assert_eq!(country_to_timezone("XX"), "UTC");
    }

    #[test]
    fn test_timezone_to_locale() {
        assert_eq!(timezone_to_locale("Asia/Tokyo"), "ja-JP,ja;q=0.9,en;q=0.8");
        assert_eq!(timezone_to_locale("America/New_York"), "en-US,en;q=0.9");
    }

    #[test]
    fn test_remember_observed_egress_region_derives_timezone_from_country() {
        remember_observed_egress_region(Some("jp"), None, "test");

        let observed = get_observed_egress_region().expect("observed egress region should exist");
        assert_eq!(observed.country_code, "JP");
        assert_eq!(observed.timezone, "Asia/Tokyo");
        assert_eq!(observed.source, "test");
    }

    #[test]
    fn test_get_fresh_observed_egress_region_returns_recent_value() {
        remember_observed_egress_region(Some("DE"), Some("Europe/Berlin"), "test");

        let observed = get_fresh_observed_egress_region(60_000).expect("fresh observed region should exist");
        assert_eq!(observed.country_code, "DE");
        assert_eq!(observed.timezone, "Europe/Berlin");
    }
}
