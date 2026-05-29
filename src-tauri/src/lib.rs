#![allow(non_snake_case)]
#![recursion_limit = "512"]

mod cmd;
pub mod config;
mod constants;
mod core;
mod enhance;
mod feat;
mod module;
mod process;
pub mod utils;
mod anti_probe;
mod tls_fingerprint;
mod security;
mod http;
mod traffic;
#[cfg(target_os = "linux")]
mod xdp;
mod multipath;

use crate::constants::files;
use crate::config::AdvancedConfig;
use crate::{
    core::handle,
    process::AsyncHandler,
    utils::{dirs, resolve, server},
};
use anyhow::Result;
use clash_verge_logging::{Type, logging};
use once_cell::sync::OnceCell;
use tauri::{AppHandle, Manager as _};
#[cfg(target_os = "macos")]
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_deep_link::DeepLinkExt as _;
use tauri_plugin_mihomo::RejectPolicy;

pub static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();
/// Application initialization helper functions
mod app_init {
    use super::*;

    /// Initialize singleton monitoring for other instances
    pub fn init_singleton_check() -> Result<()> {
        AsyncHandler::block_on(async move {
            logging!(info, Type::Setup, "开始检查单例实例...");
            server::check_singleton().await?;
            Ok(())
        })
    }

    /// Setup plugins for the Tauri builder
    pub fn setup_plugins(builder: tauri::Builder<tauri::Wry>) -> tauri::Builder<tauri::Wry> {
        #[allow(unused_mut)]
        let mut builder = builder
            .plugin(tauri_plugin_clash_verge_sysinfo::init())
            .plugin(tauri_plugin_notification::init())
            .plugin(tauri_plugin_updater::Builder::new().build())
            .plugin(tauri_plugin_clipboard_manager::init())
            .plugin(tauri_plugin_global_shortcut::Builder::new().build())
            .plugin(tauri_plugin_dialog::init())
            .plugin(tauri_plugin_shell::init())
            .plugin(tauri_plugin_deep_link::init())
            .plugin(tauri_plugin_http::init())
            .plugin(
                tauri_plugin_mihomo::Builder::new()
                    .protocol(tauri_plugin_mihomo::models::Protocol::LocalSocket)
                    .socket_path(crate::config::IClashTemp::guard_external_controller_ipc())
                    .pool_config(
                        tauri_plugin_mihomo::IpcPoolConfigBuilder::new()
                            .min_connections(3)
                            .max_connections(32)
                            .idle_timeout(std::time::Duration::from_secs(60))
                            .health_check_interval(std::time::Duration::from_secs(60))
                            .reject_policy(RejectPolicy::Wait)
                            .build(),
                    )
                    .build(),
            );

        // Devtools plugin only in debug mode with feature tauri-dev
        // to avoid duplicated registering of logger since the devtools plugin also registers a logger
        #[cfg(all(debug_assertions, not(feature = "tokio-trace"), feature = "tauri-dev"))]
        {
            builder = builder.plugin(tauri_plugin_devtools::init());
        }
        builder
    }

    /// Setup deep link handling
    pub fn setup_deep_links(app: &tauri::App) {
        #[cfg(any(target_os = "linux", all(debug_assertions, windows)))]
        {
            logging!(info, Type::Setup, "注册深层链接...");
            let _ = app.deep_link().register_all();
        }

        app.deep_link().on_open_url(|event| {
            let urls = event.urls();
            AsyncHandler::spawn(move || async move {
                if let Some(url) = urls.first()
                    && let Err(e) = resolve::resolve_scheme(url.as_ref()).await
                {
                    logging!(error, Type::Setup, "Failed to resolve scheme: {}", e);
                }
            });
        });
    }

    /// Setup autostart plugin
    pub fn setup_autostart(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(target_os = "macos")]
        let mut auto_start_plugin_builder = tauri_plugin_autostart::Builder::new();
        #[cfg(not(target_os = "macos"))]
        let auto_start_plugin_builder = tauri_plugin_autostart::Builder::new();

        #[cfg(target_os = "macos")]
        {
            auto_start_plugin_builder = auto_start_plugin_builder
                .macos_launcher(MacosLauncher::LaunchAgent)
                .app_name(&app.config().identifier);
        }
        app.handle().plugin(auto_start_plugin_builder.build())?;
        Ok(())
    }

    /// Setup window state management
    pub fn setup_window_state(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
        logging!(info, Type::Setup, "初始化窗口状态管理...");
        let window_state_plugin = tauri_plugin_window_state::Builder::new()
            .with_filename(files::WINDOW_STATE)
            .with_state_flags(tauri_plugin_window_state::StateFlags::default())
            .build();
        app.handle().plugin(window_state_plugin)?;
        Ok(())
    }

    pub fn generate_handlers() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
        tauri::generate_handler![
            tauri_plugin_clash_verge_sysinfo::commands::get_system_info,
            tauri_plugin_clash_verge_sysinfo::commands::get_app_uptime,
            tauri_plugin_clash_verge_sysinfo::commands::app_is_admin,
            tauri_plugin_clash_verge_sysinfo::commands::export_diagnostic_info,
            cmd::is_port_in_use,
            cmd::get_sys_proxy,
            cmd::get_auto_proxy,
            cmd::open_app_dir,
            cmd::open_logs_dir,
            cmd::open_web_url,
            cmd::open_core_dir,
            cmd::open_app_log,
            cmd::open_core_log,
            cmd::get_portable_flag,
            cmd::get_network_interfaces,
            cmd::get_system_hostname,
            cmd::restart_app,
            cmd::start_core,
            cmd::stop_core,
            cmd::restart_core,
            cmd::get_running_mode,
            cmd::get_auto_launch_status,
            cmd::entry_lightweight_mode,
            cmd::exit_lightweight_mode,
            cmd::install_service,
            cmd::uninstall_service,
            cmd::reinstall_service,
            cmd::repair_service,
            cmd::is_service_available,
            cmd::get_clash_info,
            cmd::patch_clash_config,
            cmd::patch_clash_mode,
            cmd::change_clash_core,
            cmd::get_runtime_config,
            cmd::get_runtime_yaml,
            cmd::get_dns_runtime_status,
            cmd::test_dns_leak,
            cmd::test_proxy_detection,
            cmd::get_tor_status,
            cmd::test_tor_connection,
            cmd::get_runtime_exists,
            cmd::get_runtime_logs,
            cmd::get_runtime_proxy_chain_config,
            cmd::update_proxy_chain_config_in_runtime,
            cmd::invoke_uwp_tool,
            cmd::copy_clash_env,
            cmd::sync_tray_proxy_selection,
            cmd::apply_dns_config,
            cmd::get_clash_logs,
            cmd::get_verge_config,
            cmd::patch_verge_config,
            cmd::test_delay,
            cmd::get_tray_icon_path,
            cmd::copy_icon_file,
            cmd::download_icon_cache,
            #[cfg(debug_assertions)]
            cmd::open_devtools,
            cmd::exit_app,
            cmd::get_network_interfaces_info,
            cmd::get_profiles,
            cmd::enhance_profiles,
            cmd::patch_profiles_config,
            cmd::view_profile,
            cmd::patch_profile,
            cmd::create_profile,
            cmd::create_profile_from_local_path,
            cmd::import_profile,
            cmd::reorder_profile,
            cmd::update_profile,
            cmd::delete_profile,
            cmd::read_profile_file,
            cmd::save_profile_file,
            cmd::get_next_update_time,
            cmd::script_validate_notice,
            cmd::validate_script_file,
            cmd::create_local_backup,
            cmd::list_local_backup,
            cmd::delete_local_backup,
            cmd::restore_local_backup,
            cmd::import_local_backup,
            cmd::export_local_backup,
            cmd::create_webdav_backup,
            cmd::save_webdav_config,
            cmd::list_webdav_backup,
            cmd::delete_webdav_backup,
            cmd::restore_webdav_backup,
            cmd::get_unlock_items,
            cmd::check_media_unlock,
            cmd::dns_query,
            cmd::dns_health_check,
            cmd::dns_batch_query,
            cmd::dns_batch_health_check,
            cmd::anti_probe_verify_handshake,
            cmd::anti_probe_generate_token,
            cmd::anti_probe_update_config,
            cmd::anti_probe_get_config,
            cmd::anti_probe_cleanup,
            cmd::tls_fingerprint_get_all,
            cmd::tls_fingerprint_get_by_name,
            cmd::tls_fingerprint_set,
            cmd::tls_fingerprint_set_by_name,
            cmd::tls_fingerprint_get_current,
            cmd::tls_fingerprint_generate_config,
            cmd::tls_fingerprint_clear,
            cmd::security_start_monitor,
            cmd::security_stop_monitor,
            cmd::security_check_status,
            cmd::security_deploy_decoy,
            cmd::security_cleanup_decoy,
            cmd::security_check_decoy_access,
            cmd::security_generate_encryption_key,
            cmd::security_encrypt_data,
            cmd::security_decrypt_data,
            cmd::security_check_encryption_key,
            cmd::security_self_destruct,
            cmd::security_start_monitor,
            cmd::security_stop_monitor,
            cmd::security_check_status,
            cmd::local_security_get_config,
            cmd::local_security_update_config,
            cmd::local_security_get_status,
            cmd::local_security_check_now,
            cmd::local_security_check_binding,
            cmd::local_security_check_port_conflict,
            cmd::local_security_find_available_port,
            cmd::local_security_configure_firewall,
            cmd::local_security_remove_firewall,
            cmd::leak_monitor_start,
            cmd::leak_monitor_stop,
            cmd::leak_monitor_is_running,
            cmd::leak_monitor_set_port,
            cmd::leak_monitor_get_port,
            cmd::multipath_get_config,
            cmd::multipath_update_config,
            cmd::multipath_get_bindings,
            cmd::multipath_add_binding,
            cmd::multipath_remove_binding,
            cmd::multipath_get_predefined_bindings,
            cmd::multipath_add_pool,
            cmd::multipath_remove_pool,
            cmd::multipath_update_pool,
            cmd::multipath_add_node,
            cmd::multipath_remove_node,
            cmd::multipath_test_node,
            cmd::multipath_import_nodes,
            cmd::multipath_export_nodes,
            cmd::multipath_get_recommended_config,
            cmd::coordinator_initialize,
            cmd::coordinator_get_config,
            cmd::coordinator_update_config,
            cmd::coordinator_shutdown,
            cmd::get_advanced_config,
            cmd::save_advanced_config,
            cmd::get_recommended_advanced_config,
            cmd::validate_advanced_config,
            cmd::coordinator_get_status,
            cmd::egress_identity_preview_match,
            cmd::egress_identity_assign_match,
            cmd::egress_identity_get_active_assignments,
            cmd::egress_identity_clear_assignment,
            cmd::session_affinity_get_bindings,
            cmd::session_affinity_clear_binding,
            cmd::session_affinity_get_predefined_rules,
            cmd::session_affinity_cleanup_expired,
            cmd::session_affinity_select_node_for_domain,
            cmd::session_affinity_select_node_for_process,
            cmd::session_affinity_select_node_for_connection,
            cmd::ip_reputation_get_config,
            cmd::ip_reputation_update_config,
            cmd::ip_reputation_check_ip,
            cmd::ip_reputation_get_predefined_rules,
            cmd::ip_reputation_select_node_for_domain,
            cmd::ip_reputation_clear_cache,
            cmd::ip_reputation_get_cache_stats,
            cmd::header_sanitization_get_config,
            cmd::header_sanitization_update_config,
            cmd::header_sanitization_test,
            cmd::header_sanitization_get_templates,
            cmd::header_sanitization_get_fingerprint,
            cmd::traffic_padding_get_config,
            cmd::traffic_padding_update_config,
            cmd::traffic_padding_start,
            cmd::traffic_padding_stop,
            cmd::traffic_padding_get_stats,
            cmd::traffic_padding_reset_stats,
            cmd::traffic_padding_is_running,
        ]
    }

    #[cfg(target_os = "linux")]
    pub fn generate_xdp_handlers() -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
        tauri::generate_handler![
            cmd::xdp_get_config,
            cmd::xdp_update_config,
            cmd::xdp_get_status,
            cmd::xdp_start,
            cmd::xdp_stop,
            cmd::xdp_add_route,
            cmd::xdp_remove_route,
            cmd::xdp_update_stats,
            cmd::xdp_check_support,
            cmd::xdp_get_interfaces,
        ]
    }
}

pub fn run() {
    if !(cfg!(debug_assertions) || cfg!(feature = "tauri-dev")) && app_init::init_singleton_check().is_err() {
        return;
    }

    #[cfg(target_os = "linux")]
    utils::linux::workarounds::apply_nvidia_dmabuf_renderer_workaround();
    #[cfg(target_os = "linux")]
    utils::linux::workarounds::apply_wayland_webkit_fix();

    let _ = utils::dirs::init_portable_flag();

    let builder = app_init::setup_plugins(tauri::Builder::default())
        .setup(|app| {
            #[allow(clippy::expect_used)]
            APP_HANDLE
                .set(app.app_handle().clone())
                .expect("failed to set global app handle");

            resolve::init_work_dir_and_logger()?;

            logging!(info, Type::Setup, "开始应用初始化...");
            if let Err(e) = app_init::setup_autostart(app) {
                logging!(error, Type::Setup, "Failed to setup autostart: {}", e);
            }

            app_init::setup_deep_links(app);

            if let Err(e) = app_init::setup_window_state(app) {
                logging!(error, Type::Setup, "Failed to setup window state: {}", e);
            }

            resolve::resolve_setup_async();
            resolve::resolve_setup_sync();
            resolve::init_signal();

            // 从高级配置加载流量填充配置并应用（如启用则自动启动）
            let padding_cfg = dirs::app_home_dir()
                .ok()
                .map(|path| path.join("advanced.yaml"))
                .and_then(|path| AdvancedConfig::load(&path).ok())
                .map(|cfg| cfg.traffic_padding);

            if let Some(padding_cfg) = padding_cfg {
                AsyncHandler::spawn(move || async move {
                    if let Err(e) = crate::cmd::traffic::apply_traffic_padding_config(padding_cfg).await {
                        logging!(warn, Type::Setup, "Failed to apply traffic padding config at startup: {}", e);
                    }
                });
            }

            // 初始化核心协调器
            logging!(info, Type::Setup, "初始化核心协调器...");
            let coordinator = cmd::coordinator::get_coordinator();
            if let Err(e) = coordinator.initialize() {
                logging!(error, Type::Setup, "协调器初始化失败: {}", e);
            } else {
                logging!(info, Type::Setup, "协调器初始化成功");
            }

            // 启动会话绑定清理任务
            logging!(info, Type::Setup, "启动会话绑定清理任务...");
            cmd::session_affinity::start_cleanup_task();

            logging!(info, Type::Setup, "初始化已启动");
            Ok(())
        })
        .invoke_handler(app_init::generate_handlers());

    mod event_handlers {
        #[cfg(target_os = "macos")]
        use crate::module::lightweight;
        use crate::utils::window_manager::WindowManager;
        use crate::{
            config::Config,
            core::{self, handle, hotkey},
            process::AsyncHandler,
        };
        use clash_verge_logging::{Type, logging};
        use tauri::AppHandle;
        #[cfg(target_os = "macos")]
        use tauri::Manager as _;

        pub fn handle_ready_resumed(_app_handle: &AppHandle) {
            if handle::Handle::global().is_exiting() {
                logging!(debug, Type::System, "应用正在退出，跳过处理");
                return;
            }

            logging!(info, Type::System, "应用就绪");

        }

        #[cfg(target_os = "macos")]
        pub async fn handle_reopen(has_visible_windows: bool) {
            if lightweight::is_in_lightweight_mode() {
                lightweight::exit_lightweight_mode().await;
                return;
            }

            if !has_visible_windows {
                handle::Handle::global().set_activation_policy_regular();
                let _ = WindowManager::show_main_window().await;
            }
        }

        pub fn handle_window_close(api: &tauri::WindowEvent) {
            #[cfg(target_os = "macos")]
            handle::Handle::global().set_activation_policy_accessory();

            if core::handle::Handle::global().is_exiting() {
                return;
            }

            if let tauri::WindowEvent::CloseRequested { api, .. } = api {
                api.prevent_close();
                if let Some(window) = WindowManager::get_main_window() {
                    let _ = window.hide();
                }
            }
        }

        pub fn handle_window_focus(focused: bool) {
            AsyncHandler::spawn(move || async move {
                let is_enable_global_hotkey = Config::verge().await.data_arc().enable_global_hotkey.unwrap_or(true);

                if focused {
                    #[cfg(target_os = "macos")]
                    {
                        use crate::core::hotkey::SystemHotkey;
                        let _ = hotkey::Hotkey::global()
                            .register_system_hotkey(SystemHotkey::CmdQ)
                            .await;
                        let _ = hotkey::Hotkey::global()
                            .register_system_hotkey(SystemHotkey::CmdW)
                            .await;
                    }
                    if !is_enable_global_hotkey {
                        let _ = hotkey::Hotkey::global().init(false).await;
                    }
                    return;
                }

                #[cfg(target_os = "macos")]
                {
                    use crate::core::hotkey::SystemHotkey;
                    let _ = hotkey::Hotkey::global().unregister_system_hotkey(SystemHotkey::CmdQ);
                    let _ = hotkey::Hotkey::global().unregister_system_hotkey(SystemHotkey::CmdW);
                }

                if !is_enable_global_hotkey {
                    let _ = hotkey::Hotkey::global().reset();
                }
            });
        }

        #[cfg(target_os = "macos")]
        pub fn handle_window_destroyed() {
            use crate::core::hotkey::SystemHotkey;
            AsyncHandler::spawn(move || async move {
                let _ = hotkey::Hotkey::global().unregister_system_hotkey(SystemHotkey::CmdQ);
                let _ = hotkey::Hotkey::global().unregister_system_hotkey(SystemHotkey::CmdW);
                let is_enable_global_hotkey = Config::verge().await.data_arc().enable_global_hotkey.unwrap_or(true);
                if !is_enable_global_hotkey {
                    let _ = hotkey::Hotkey::global().reset();
                }
            });
        }
    }

    #[cfg(feature = "clippy")]
    let context = tauri::test::mock_context(tauri::test::noop_assets());
    #[cfg(feature = "clippy")]
    let app = builder.build(context).unwrap_or_else(|e| {
        logging!(error, Type::Setup, "Failed to build Tauri application: {}", e);
        std::process::exit(1);
    });

    #[cfg(not(feature = "clippy"))]
    let app = builder.build(tauri::generate_context!()).unwrap_or_else(|e| {
        logging!(error, Type::Setup, "Failed to build Tauri application: {}", e);
        std::process::exit(1);
    });

    app.run(|app_handle, e| match e {
        tauri::RunEvent::Ready | tauri::RunEvent::Resumed => {
            if core::handle::Handle::global().is_exiting() {
                return;
            }
            event_handlers::handle_ready_resumed(app_handle);
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen {
            has_visible_windows, ..
        } => {
            if core::handle::Handle::global().is_exiting() {
                return;
            }
            AsyncHandler::spawn(move || async move {
                event_handlers::handle_reopen(has_visible_windows).await;
            });
        }
        tauri::RunEvent::Exit => {
            logging!(info, Type::System, "Application exited");
        }
        #[allow(unused_variables)]
        tauri::RunEvent::ExitRequested { api, code, .. } => {
            if module::lightweight::is_in_lightweight_mode() && !handle::Handle::global().is_exiting() {
                api.prevent_exit();
            } else if code.is_none() {
                api.prevent_exit();
                if !handle::Handle::global().is_exiting() {
                    AsyncHandler::spawn(|| async {
                        feat::quit().await;
                    });
                }
            }
        }
        tauri::RunEvent::WindowEvent { label, event, .. } if label == "main" => match event {
            tauri::WindowEvent::CloseRequested { .. } => {
                event_handlers::handle_window_close(&event);
            }
            tauri::WindowEvent::Focused(focused) => {
                event_handlers::handle_window_focus(focused);
            }
            #[cfg(target_os = "macos")]
            tauri::WindowEvent::Destroyed => {
                event_handlers::handle_window_destroyed();
            }
            _ => {}
        },
        _ => {}
    });
}
