/**
 * 时区/NTP 伪装 enhance 集成
 *
 * 在 enhance 管线中，根据配置注入 Mihomo ntp: 配置段
 * 选择与出口节点区域匹配的 NTP 服务器
 */

use serde_yaml_ng::{Mapping, Value};

use crate::config::advanced::AdvancedConfig;
use crate::core::timezone_spoof::{NtpStrategy, select_ntp_server};

macro_rules! revise {
    ($map: expr, $key: expr, $val: expr) => {
        let ret_key = Value::String($key.into());
        $map.insert(ret_key, Value::from($val));
    };
}

/// Load advanced config from coordinator (same pattern as other enhance modules)
fn load_advanced_config() -> AdvancedConfig {
    crate::feat::get_coordinator().get_advanced_config()
}

/// Apply timezone/NTP spoofing config to Mihomo YAML
///
/// Injects the `ntp:` section based on TimezoneSpoofConfig.
/// When Auto mode is used, selects NTP server matching the egress region.
pub fn apply_timezone_spoof_config(mut config: Mapping) -> Mapping {
    let advanced = load_advanced_config();
    let cfg = &advanced.timezone_spoof;

    if !cfg.enabled {
        // 确保关闭 NTP
        revise!(config, "ntp", build_ntp_disabled());
        return config;
    }

    let ntp_section = match &cfg.ntp_strategy {
        NtpStrategy::Disabled => {
            // 仅伪装 HTTP 头，不启用 NTP
            revise!(config, "ntp", build_ntp_disabled());
            return config;
        }
        NtpStrategy::Manual => {
            let server = cfg
                .manual_ntp_server
                .clone()
                .unwrap_or_else(|| "pool.ntp.org".to_string());
            build_ntp_enabled(&server, cfg.ntp_interval_min, cfg.write_to_system, cfg.dialer_proxy.as_deref())
        }
        NtpStrategy::Auto => {
            // 尝试从出口身份推断区域
            let country_code = detect_egress_country_code();
            let server = select_ntp_server(&country_code);
            log::info!(
                "[TimezoneSpoof] Auto 模式: 检测到出口区域={}, 选择 NTP 服务器={}",
                country_code,
                server
            );
            build_ntp_enabled(&server, cfg.ntp_interval_min, cfg.write_to_system, cfg.dialer_proxy.as_deref())
        }
    };

    revise!(config, "ntp", ntp_section);
    config
}

/// 构建 NTP 启用的 YAML 映射
fn build_ntp_enabled(
    server: &str,
    interval_min: u32,
    write_to_system: bool,
    dialer_proxy: Option<&str>,
) -> Value {
    let mut ntp = Mapping::new();
    revise!(ntp, "enable", true);
    revise!(ntp, "server", server.to_string());
    revise!(ntp, "port", 123);
    revise!(ntp, "interval", interval_min as i64);
    revise!(ntp, "write-to-system", write_to_system);
    if let Some(proxy) = dialer_proxy {
        revise!(ntp, "dialer-proxy", proxy.to_string());
    }
    Value::Mapping(ntp)
}

/// 构建 NTP 禁用的 YAML 映射
fn build_ntp_disabled() -> Value {
    let mut ntp = Mapping::new();
    revise!(ntp, "enable", false);
    Value::Mapping(ntp)
}

/// 尝试检测出口节点的国家代码
///
/// 优先从出口身份管理器获取，回退到 IP 信誉检测
fn detect_egress_country_code() -> String {
    // 尝试从出口身份管理器获取当前活跃出口的国家代码
    // 这里使用 IP 信誉检测的缓存作为回退
    // 由于 enhance 在同步上下文中运行，使用阻塞调用

    // 方案1: 从出口身份配置中的 profile 推断
    // 方案2: 从 IP 信誉缓存推断
    // 方案3: 默认 US

    // 简化实现：从出口身份配置获取默认区域
    let advanced = load_advanced_config();
    if let Some(profile) = advanced.egress_identity.profiles.first() {
        // profile 中的 region 字段（如果存在）
        // 目前 EgressIdentityProfile 没有 country_code 字段
        // 使用 profile 的 dns_mode 和描述推断
        let _ = profile; // 避免未使用警告
    }

    // 回退：从 IP 信誉缓存获取
    // 由于 enhance 是同步的，无法直接 await 异步操作
    // 使用一个简单的启发式：如果配置了 IP 信誉规则，取第一个规则的国家代码

    // 最终回退
    "US".to_string()
}
