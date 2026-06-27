use crate::{
    config::{DEFAULT_PAC, deserialize_encrypted, serialize_encrypted},
    utils::{dirs, help},
};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use log::LevelFilter;
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

/// ### `verge.yaml` schema
#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct IVerge {
    /// app log level
    /// silent | error | warn | info | debug | trace
    pub app_log_level: Option<String>,

    /// app log max size in KB
    pub app_log_max_size: Option<u64>,

    /// app log max count
    pub app_log_max_count: Option<usize>,

    // i18n
    pub language: Option<String>,

    /// `light` or `dark` or `system`
    pub theme_mode: Option<String>,

    /// copy env type
    pub env_type: Option<String>,

    /// start page
    pub start_page: Option<String>,
    /// startup script path
    pub startup_script: Option<String>,

    /// menu order
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu_order: Option<Vec<String>>,

    /// clash tun mode
    pub enable_tun_mode: Option<bool>,

    /// can the app auto startup
    pub enable_auto_launch: Option<bool>,

    /// not show the window on launch
    pub enable_silent_start: Option<bool>,

    /// set system proxy
    pub enable_system_proxy: Option<bool>,

    /// enable proxy guard
    pub enable_proxy_guard: Option<bool>,

    /// enable bypass format check
    pub enable_bypass_check: Option<bool>,

    /// enable dns settings - this controls whether dns_config.yaml is applied
    pub enable_dns_settings: Option<bool>,

    /// always use default bypass
    pub use_default_bypass: Option<bool>,

    /// set system proxy bypass
    pub system_proxy_bypass: Option<String>,

    /// proxy guard duration
    pub proxy_guard_duration: Option<u64>,

    /// use pac mode
    pub proxy_auto_config: Option<bool>,

    /// pac script content
    pub pac_file_content: Option<String>,

    /// proxy host address
    pub proxy_host: Option<String>,

    /// theme setting
    pub theme_setting: Option<IVergeTheme>,

    /// hotkey map
    /// format: {func},{key}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hotkeys: Option<Vec<String>>,

    /// enable global hotkey
    pub enable_global_hotkey: Option<bool>,

    /// 首页卡片设置
    /// 控制首页各个卡片的显示和隐藏
    pub home_cards: Option<serde_json::Value>,

    /// auto check updates
    pub auto_check_update: Option<bool>,

    /// default latency test URL
    pub default_latency_test: Option<String>,

    /// default latency timeout in milliseconds
    pub default_latency_timeout: Option<i16>,

    /// proxy layout columns
    pub proxy_layout_column: Option<u8>,

    /// 测试站列表
    pub test_list: Option<Vec<IVergeTestItem>>,

    /// 日志清理
    /// 0: 不清理; 1: 1天；2: 7天; 3: 30天; 4: 90天
    pub auto_log_clean: Option<i32>,

    /// Enable scheduled automatic backups
    pub enable_auto_backup_schedule: Option<bool>,

    /// Automatic backup interval in hours
    pub auto_backup_interval_hours: Option<u64>,

    /// Create backups automatically when critical configs change
    pub auto_backup_on_change: Option<bool>,

    /// verge 的各种 port 用于覆盖 clash 的各种 port
    pub verge_mixed_port: Option<u16>,

    pub verge_socks_port: Option<u16>,

    pub verge_socks_enabled: Option<bool>,

    pub verge_port: Option<u16>,

    pub verge_http_enabled: Option<bool>,

    /// WebDAV 配置 (加密存储)
    #[serde(
        serialize_with = "serialize_encrypted",
        deserialize_with = "deserialize_encrypted",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub webdav_url: Option<String>,

    /// WebDAV 用户名 (加密存储)
    #[serde(
        serialize_with = "serialize_encrypted",
        deserialize_with = "deserialize_encrypted",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub webdav_username: Option<String>,

    /// WebDAV 密码 (加密存储)
    #[serde(
        serialize_with = "serialize_encrypted",
        deserialize_with = "deserialize_encrypted",
        skip_serializing_if = "Option::is_none",
        default
    )]
    pub webdav_password: Option<String>,

    pub enable_tor_proxy: Option<bool>,

    pub tor_socks_host: Option<String>,

    pub tor_socks_port: Option<u16>,

    pub tor_control_port: Option<u16>,

    pub tor_use_bridges: Option<bool>,

    pub tor_bridges: Option<Vec<String>>,

    /// 启用外部控制器
    pub enable_external_controller: Option<bool>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct IVergeTestItem {
    pub uid: Option<String>,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub url: Option<String>,
}

#[derive(Default, Debug, Clone, Deserialize, Serialize)]
pub struct IVergeTheme {
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub primary_text: Option<String>,
    pub secondary_text: Option<String>,

    pub info_color: Option<String>,
    pub error_color: Option<String>,
    pub warning_color: Option<String>,
    pub success_color: Option<String>,

    pub font_family: Option<String>,
    pub css_injection: Option<String>,
}

impl IVerge {
    pub async fn new() -> Self {
        match dirs::verge_path() {
            Ok(path) => match help::read_yaml::<Self>(&path).await {
                Ok(mut config) => {
                    // compatibility
                    if let Some(start_page) = config.start_page.clone()
                        && start_page == "/home"
                    {
                        config.start_page = Some(String::from("/"));
                    }
                    config
                }
                Err(err) => {
                    logging!(error, Type::Config, "{err}");
                    Self::template()
                }
            },
            Err(err) => {
                logging!(error, Type::Config, "{err}");
                Self::template()
            }
        }
    }

    pub fn template() -> Self {
        Self {
            app_log_max_size: Some(128),
            app_log_max_count: Some(8),
            language: Some(clash_verge_i18n::system_language().into()),
            theme_mode: Some("system".into()),
            env_type: Some("powershell".into()),
            start_page: Some("/".into()),
            enable_auto_launch: Some(false),
            enable_silent_start: Some(false),
            enable_system_proxy: Some(false),
            proxy_auto_config: Some(false),
            pac_file_content: Some(DEFAULT_PAC.into()),
            proxy_host: Some("127.0.0.1".into()),
            verge_mixed_port: Some(7897),
            verge_socks_port: Some(7898),
            verge_socks_enabled: Some(false),
            verge_port: Some(7899),
            verge_http_enabled: Some(false),
            enable_proxy_guard: Some(false),
            enable_bypass_check: Some(true),
            use_default_bypass: Some(true),
            proxy_guard_duration: Some(30),
            auto_check_update: Some(true),
            auto_log_clean: Some(2), // 1: 1天, 2: 7天, 3: 30天, 4: 90天
            enable_auto_backup_schedule: Some(false),
            auto_backup_interval_hours: Some(24),
            auto_backup_on_change: Some(true),
            webdav_url: None,
            webdav_username: None,
            webdav_password: None,
            enable_global_hotkey: Some(true),
            enable_dns_settings: Some(false),
            home_cards: None,
            enable_tor_proxy: Some(false),
            tor_socks_host: Some("127.0.0.1".into()),
            tor_socks_port: Some(9050),
            tor_control_port: Some(9051),
            tor_use_bridges: Some(false),
            tor_bridges: Some(Vec::new()),
            enable_external_controller: Some(false),
            ..Self::default()
        }
    }

    /// Save IVerge App Config
    pub async fn save_file(&self) -> Result<()> {
        help::save_yaml(&dirs::verge_path()?, &self, Some("# Clash Verge Optimized Config")).await
    }

    /// patch verge config
    /// only save to file
    #[allow(clippy::cognitive_complexity)]
    pub fn patch_config(&mut self, patch: &Self) {
        macro_rules! patch {
            ($key: tt) => {
                if patch.$key.is_some() {
                    self.$key = patch.$key.clone();
                }
            };
        }

        patch!(app_log_level);
        patch!(app_log_max_size);
        patch!(app_log_max_count);

        patch!(language);
        patch!(theme_mode);
        patch!(env_type);
        patch!(start_page);
        patch!(startup_script);
        patch!(menu_order);

        patch!(enable_tun_mode);
        patch!(enable_auto_launch);
        patch!(enable_silent_start);
        patch!(verge_mixed_port);
        patch!(verge_socks_port);
        patch!(verge_socks_enabled);
        patch!(verge_port);
        patch!(verge_http_enabled);
        patch!(enable_system_proxy);
        patch!(enable_proxy_guard);
        patch!(enable_bypass_check);
        patch!(use_default_bypass);
        patch!(system_proxy_bypass);
        patch!(proxy_guard_duration);
        patch!(proxy_auto_config);
        patch!(pac_file_content);
        patch!(proxy_host);
        patch!(theme_setting);
        patch!(hotkeys);
        patch!(enable_global_hotkey);

        patch!(auto_check_update);
        patch!(default_latency_test);
        patch!(default_latency_timeout);
        patch!(proxy_layout_column);
        patch!(test_list);
        patch!(auto_log_clean);
        patch!(enable_auto_backup_schedule);
        patch!(auto_backup_interval_hours);
        patch!(auto_backup_on_change);

        patch!(webdav_url);
        patch!(webdav_username);
        patch!(webdav_password);
        patch!(enable_dns_settings);
        patch!(home_cards);
        patch!(enable_external_controller);
    }

    pub const fn get_singleton_port() -> u16 {
        crate::constants::network::ports::SINGLETON_SERVER
    }

    /// 获取日志等级
    pub fn get_log_level(&self) -> LevelFilter {
        if let Some(level) = self.app_log_level.as_ref() {
            match level.to_lowercase().as_str() {
                "silent" => LevelFilter::Off,
                "error" => LevelFilter::Error,
                "warn" => LevelFilter::Warn,
                "info" => LevelFilter::Info,
                "debug" => LevelFilter::Debug,
                "trace" => LevelFilter::Trace,
                _ => LevelFilter::Info,
            }
        } else {
            LevelFilter::Info
        }
    }
}
