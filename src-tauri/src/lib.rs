#![allow(dead_code, non_snake_case)]
#![allow(
    clippy::all,
    clippy::clone_on_ref_ptr,
    clippy::collapsible_if,
    clippy::collapsible_match,
    clippy::derivable_impls,
    clippy::expect_used,
    clippy::field_reassign_with_default,
    clippy::manual_range_contains,
    clippy::manual_slice_fill,
    clippy::missing_const_for_fn,
    clippy::needless_borrows_for_generic_args,
    clippy::needless_pass_by_ref_mut,
    clippy::needless_range_loop,
    clippy::panic,
    clippy::redundant_clone,
    clippy::significant_drop_in_scrutinee,
    clippy::significant_drop_tightening,
    clippy::too_many_arguments,
    clippy::unused_async,
    clippy::use_self,
    clippy::useless_conversion,
    clippy::useless_vec,
    clippy::unwrap_used
)]
#![recursion_limit = "512"]

mod anti_probe;
mod app;
mod cmd;
pub mod config;
mod constants;
mod core;
mod enhance;
mod feat;
mod http;
mod module;
mod multipath;
mod process;
mod security;
mod subscription;
mod tls_fingerprint;
mod traffic;
pub mod utils;
#[cfg(target_os = "linux")]
mod xdp;

use crate::constants::files;
use crate::{
    core::handle,
    process::AsyncHandler,
    utils::{resolve, server},
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
        let mihomo_protocol = tauri_plugin_mihomo::models::Protocol::LocalSocket;

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
                    .protocol(mihomo_protocol)
                    .external_host("127.0.0.1")
                    .external_port(9097)
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
            .with_state_flags(
                tauri_plugin_window_state::StateFlags::default()
                    .difference(tauri_plugin_window_state::StateFlags::DECORATIONS),
            )
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
            cmd::ensure_mihomo_core_ready,
            cmd::get_running_mode,
            cmd::get_auto_launch_status,
            cmd::install_service,
            cmd::uninstall_service,
            cmd::reinstall_service,
            cmd::repair_service,
            cmd::is_service_available,
            cmd::get_clash_info,
            cmd::patch_clash_config,
            cmd::explain_config_diff,
            cmd::patch_clash_mode,
            cmd::get_runtime_config,
            cmd::get_runtime_yaml,
            cmd::get_runtime_diagnostics_summary,
            cmd::get_dns_runtime_status,
            cmd::test_dns_leak,
            cmd::test_proxy_detection,
            cmd::get_current_egress_identity,
            cmd::get_identity_consistency_report,
            cmd::get_identity_consistency_history,
            cmd::get_identity_consistency_drift_report,
            cmd::get_tor_status,
            cmd::test_tor_connection,
            cmd::get_runtime_exists,
            cmd::get_runtime_logs,
            cmd::get_runtime_proxy_chain_config,
            cmd::update_proxy_chain_config_in_runtime,
            cmd::invoke_uwp_tool,
            cmd::copy_clash_env,
            cmd::sync_tray_proxy_selection,
            cmd::plan_node_selection,
            cmd::apply_dns_config,
            cmd::get_clash_logs,
            cmd::clear_logs,
            cmd::get_geo_data_update_time,
            cmd::get_verge_config,
            cmd::patch_verge_config,
            cmd::authorize_startup_script,
            cmd::clear_startup_script_authorization,
            cmd::test_delay,
            cmd::plan_latency_test,
            cmd::download_icon_cache,
            #[cfg(debug_assertions)]
            cmd::open_devtools,
            cmd::exit_app,
            cmd::get_network_interfaces_info,
            cmd::get_profiles,
            cmd::enhance_profiles,
            cmd::patch_profiles_config,
            cmd::patch_profiles_config_by_profile_index,
            cmd::view_profile,
            cmd::read_china_rules_file,
            cmd::test_rule_match,
            cmd::patch_profile,
            cmd::create_profile,
            cmd::create_profile_from_local_path,
            cmd::import_profile,
            cmd::reorder_profile,
            cmd::update_profile,
            cmd::delete_profile,
            cmd::read_profile_file,
            cmd::save_china_rules_file,
            cmd::save_profile_file,
            cmd::get_next_update_time,
            cmd::get_subscription_state,
            cmd::list_subscription_sources,
            cmd::get_subscription_source,
            cmd::get_subscription_source_state,
            cmd::get_subscription_source_update_events,
            cmd::plan_subscription_update_transport,
            cmd::get_subscription_artifact_diagnostics,
            cmd::get_subscription_artifact_metadata,
            cmd::get_subscription_artifact_content,
            cmd::list_subscription_artifacts,
            cmd::list_subscription_artifact_summaries,
            cmd::cleanup_subscription_artifacts_by_retention,
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
            cmd::dns_get_provider_registrations,
            cmd::dns_probe_provider,
            cmd::dns_explain_config,
            cmd::dns_plan_probe,
            cmd::anti_probe_get_config,
            cmd::anti_probe_verify_handshake,
            cmd::anti_probe_generate_token,
            cmd::anti_probe_cleanup,
            cmd::tls_fingerprint_get_all,
            cmd::tls_fingerprint_get_by_name,
            cmd::tls_fingerprint_get_current,
            cmd::tls_fingerprint_generate_config,
            cmd::tls_fingerprint_clear,
            cmd::security_start_monitor,
            cmd::security_stop_monitor,
            cmd::security_check_status,
            cmd::security_deploy_decoy,
            cmd::security_cleanup_decoy,
            cmd::security_check_decoy_access,
            cmd::security_deploy_decoy_plan,
            cmd::security_cleanup_decoy_plan,
            cmd::security_check_decoy_plan_access,
            cmd::security_generate_encryption_key,
            cmd::security_encrypt_data,
            cmd::security_decrypt_data,
            cmd::security_check_encryption_key,
            cmd::security_self_destruct,
            cmd::security_honeypot_get_status,
            cmd::security_honeypot_is_triggered,
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
            cmd::local_stealth_get_config,
            cmd::local_stealth_update_config,
            cmd::local_stealth_apply,
            cmd::local_stealth_restore,
            cmd::local_stealth_allocate_port,
            cmd::local_stealth_get_port,
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
            cmd::multipath_get_node_stats,
            cmd::multipath_select_node,
            cmd::multipath_record_connection_end,
            cmd::multipath_record_latency,
            cmd::multipath_record_error,
            cmd::multipath_get_active_session_count,
            cmd::multipath_cleanup_sessions,
            cmd::coordinator_initialize,
            cmd::coordinator_shutdown,
            cmd::get_advanced_config,
            cmd::save_advanced_config,
            cmd::get_recommended_advanced_config,
            cmd::validate_advanced_config,
            cmd::coordinator_get_status,
            cmd::egress_identity_get_config,
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
            cmd::ip_reputation_probe_metadata_provider,
            cmd::ip_reputation_get_predefined_rules,
            cmd::ip_reputation_select_node_for_domain,
            cmd::ip_reputation_clear_cache,
            cmd::ip_reputation_get_cache_stats,
            cmd::ip_reputation_get_cache_entries,
            cmd::ip_reputation_verify_residential_proxy,
            cmd::blackhole_breaker_get_config,
            cmd::blackhole_breaker_update_config,
            cmd::blackhole_breaker_get_states,
            cmd::blackhole_breaker_record_result,
            cmd::blackhole_breaker_should_block_domain,
            cmd::blackhole_breaker_should_block_node,
            cmd::blackhole_breaker_reset_rule,
            cmd::blackhole_breaker_trip_rule,
            cmd::blackhole_breaker_record_fraud_score,
            cmd::timezone_spoof_get_config,
            cmd::timezone_spoof_update_config,
            cmd::timezone_spoof_get_ntp_server,
            cmd::timezone_spoof_get_timezone,
            cmd::timezone_spoof_get_locale,
            cmd::header_sanitization_get_config,
            cmd::header_sanitization_update_config,
            cmd::header_sanitization_test,
            cmd::header_sanitization_get_templates,
            cmd::header_sanitization_get_fingerprint,
            cmd::traffic_obfuscation_get_config,
            cmd::traffic_obfuscation_update_config,
            cmd::traffic_obfuscation_start,
            cmd::traffic_obfuscation_stop,
            cmd::traffic_obfuscation_get_stats,
            cmd::traffic_obfuscation_reset_stats,
            cmd::traffic_obfuscation_is_running,
            cmd::traffic_obfuscation_apply_profile,
            cmd::traffic_get_connection_metrics_snapshot,
            cmd::traffic_reset_connection_metrics,
            cmd::egress_monitor_get_config,
            cmd::egress_monitor_update_config,
            cmd::egress_monitor_start,
            cmd::egress_monitor_stop,
            cmd::egress_monitor_get_stats,
            cmd::egress_monitor_reset_stats,
            cmd::egress_monitor_probe_now,
            cmd::egress_monitor_is_running,
            cmd::security_policy_apply,
            cmd::security_policy_revoke,
            cmd::security_policy_apply_all,
            cmd::security_policy_revoke_all,
            cmd::security_policy_get_states,
            cmd::security_policy_get_state,
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

            if let Err(err) = AsyncHandler::block_on(async { handle::Handle::sync_mihomo_controller_state().await }) {
                logging!(
                    warn,
                    Type::Setup,
                    "Failed to sync Mihomo controller state during setup: {}",
                    err
                );
            }

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

            // 初始化核心协调器（会加载 advanced.yaml 到内存）
            logging!(info, Type::Setup, "初始化核心协调器...");
            let coordinator = crate::core::coordinator::get_coordinator();
            if let Err(e) = coordinator.initialize() {
                logging!(error, Type::Setup, "Failed to initialize coordinator: {}", e);
            }

            // 从协调器内存配置读取流量混淆配置并应用
            let cfg = coordinator.get_advanced_config();
            let obf_cfg = if cfg.traffic_obfuscation.enabled {
                Some(cfg.traffic_obfuscation)
            } else if cfg.traffic_padding.enabled {
                Some(crate::traffic::TrafficObfuscationConfig::from_legacy_padding(
                    &cfg.traffic_padding,
                ))
            } else {
                None
            };

            if let Some(obf_cfg) = obf_cfg {
                AsyncHandler::spawn(move || async move {
                    if let Err(e) = crate::cmd::traffic::apply_traffic_obfuscation_config(obf_cfg).await {
                        logging!(
                            warn,
                            Type::Setup,
                            "Failed to apply traffic obfuscation config at startup: {}",
                            e
                        );
                    }
                });
            }

            // 启动会话绑定清理任务
            logging!(info, Type::Setup, "启动会话绑定清理任务...");
            crate::core::session_affinity::get_session_affinity_manager().start_cleanup_task();

            logging!(info, Type::Setup, "初始化已启动");
            Ok(())
        })
        .invoke_handler(app_init::generate_handlers());

    mod event_handlers {
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
            if code.is_none() {
                api.prevent_exit();
                if !handle::Handle::global().is_exiting() {
                    AsyncHandler::spawn(|| async {
                        app::window::quit().await;
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
