use crate::{
    config::{Config, IVerge},
    core::{autostart, handle, hotkey, logger::Logger, runtime_bridge, runtime_lifecycle, sysopt, tray},
    module::auto_backup::AutoBackupManager,
};
use anyhow::Result;
use bitflags::bitflags;
use clash_verge_draft::SharedDraft;
use clash_verge_logging::{Type, logging, logging_error};
use serde_yaml_ng::Mapping;

const LIFECYCLE_PATCH_SENSITIVE: &str = "patch_sensitive_config";

pub async fn patch_clash(patch: &Mapping) -> Result<()> {
    let sensitive_keys: Vec<&str> = ["secret", "external-controller"]
        .into_iter()
        .filter(|key| patch.get(*key).is_some())
        .collect();

    Config::clash().await.edit_draft(|d| d.patch_config(patch));

    let res = {
        if !sensitive_keys.is_empty() {
            Config::generate().await?;
            runtime_lifecycle::restart_runtime_core("patch-clash-sensitive").await?;
        } else {
            if patch.get("mode").is_some() {
                tray::Tray::global().update_menu_and_icon().await;
            }
            Config::runtime().await.edit_draft(|d| d.patch_config(patch));
            runtime_lifecycle::update_runtime_config_checked("patch-clash-runtime").await?;
        }
        handle::Handle::refresh_clash();
        <Result<()>>::Ok(())
    };

    match res {
        Ok(()) => {
            Config::clash().await.apply();
            let clash_data = Config::clash().await.data_arc();
            clash_data.save_config().await?;
            if !sensitive_keys.is_empty() {
                crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                    LIFECYCLE_PATCH_SENSITIVE,
                    true,
                    None,
                    Some(sensitive_keys.join(", ")),
                );
            }
            Ok(())
        }
        Err(err) => {
            Config::clash().await.discard();
            if !sensitive_keys.is_empty() {
                crate::core::runtime_snapshot::record_and_persist_runtime_lifecycle_event(
                    LIFECYCLE_PATCH_SENSITIVE,
                    false,
                    Some(err.to_string()),
                    Some(sensitive_keys.join(", ")),
                );
            }
            Err(err)
        }
    }
}

bitflags! {
     #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
     struct UpdateFlags: u16 {
        const RESTART_CORE = 1 << 0;
        const CLASH_CONFIG = 1 << 1;
        const VERGE_CONFIG = 1 << 2;
        const LAUNCH = 1 << 3;
        const SYS_PROXY = 1 << 4;
        const SYSTRAY_ICON = 1 << 5;
        const HOTKEY = 1 << 6;
        const SYSTRAY_MENU = 1 << 7;
        const SYSTRAY_TOOLTIP = 1 << 8;
        const LANGUAGE = 1 << 11;
        const LOG_LEVEL = 1 << 12;
        const LOG_FILE = 1 << 13;

        const GROUP_SYS_TRAY = Self::SYSTRAY_MENU.bits()
                             | Self::SYSTRAY_TOOLTIP.bits()
                             | Self::SYSTRAY_ICON.bits();
     }
}

fn determine_update_flags(patch: &IVerge) -> UpdateFlags {
    let tun_mode = patch.enable_tun_mode;
    let auto_launch = patch.enable_auto_launch;
    let system_proxy = patch.enable_system_proxy;
    let pac = patch.proxy_auto_config;
    let pac_content = &patch.pac_file_content;
    let proxy_bypass = &patch.system_proxy_bypass;
    let language = &patch.language;
    let mixed_port = patch.verge_mixed_port;
    #[cfg(not(target_os = "windows"))]
    let redir_enabled = patch.verge_redir_enabled;
    #[cfg(not(target_os = "windows"))]
    let redir_port = patch.verge_redir_port;
    #[cfg(target_os = "linux")]
    let tproxy_enabled = patch.verge_tproxy_enabled;
    #[cfg(target_os = "linux")]
    let tproxy_port = patch.verge_tproxy_port;
    let socks_enabled = patch.verge_socks_enabled;
    let socks_port = patch.verge_socks_port;
    let http_enabled = patch.verge_http_enabled;
    let http_port = patch.verge_port;
    let enable_global_hotkey = patch.enable_global_hotkey;
    let home_cards = patch.home_cards.as_ref();
    let enable_external_controller = patch.enable_external_controller;
    let enable_proxy_guard = patch.enable_proxy_guard;
    let proxy_guard_duration = patch.proxy_guard_duration;
    let log_level = &patch.app_log_level;
    let log_max_size = patch.app_log_max_size;
    let log_max_count = patch.app_log_max_count;

    #[cfg(target_os = "windows")]
    let restart_core_needed = socks_enabled.is_some()
        || http_enabled.is_some()
        || socks_port.is_some()
        || http_port.is_some()
        || mixed_port.is_some()
        || enable_external_controller.is_some()
        || tun_mode.is_some();
    #[cfg(not(target_os = "windows"))]
    let mut restart_core_needed = socks_enabled.is_some()
        || http_enabled.is_some()
        || socks_port.is_some()
        || http_port.is_some()
        || mixed_port.is_some()
        || enable_external_controller.is_some();
    #[cfg(not(target_os = "windows"))]
    {
        restart_core_needed |= redir_enabled.is_some() || redir_port.is_some();
    }
    #[cfg(target_os = "linux")]
    {
        restart_core_needed |= tproxy_enabled.is_some() || tproxy_port.is_some();
        restart_core_needed |= tun_mode == Some(true);
    }

    let mut update_flags = UpdateFlags::empty();
    if restart_core_needed {
        update_flags.insert(UpdateFlags::RESTART_CORE);
    }
    #[cfg(target_os = "windows")]
    if tun_mode.is_some() {
        update_flags.insert(UpdateFlags::GROUP_SYS_TRAY);
    }
    #[cfg(not(target_os = "windows"))]
    if tun_mode.is_some() {
        update_flags.insert(UpdateFlags::CLASH_CONFIG | UpdateFlags::GROUP_SYS_TRAY);
    }
    if enable_global_hotkey.is_some() || home_cards.is_some() {
        update_flags.insert(UpdateFlags::VERGE_CONFIG);
    }
    if auto_launch.is_some() {
        update_flags.insert(UpdateFlags::LAUNCH);
    }
    if system_proxy.is_some() {
        update_flags.insert(UpdateFlags::SYS_PROXY | UpdateFlags::GROUP_SYS_TRAY);
    }
    if proxy_bypass.is_some()
        || pac_content.is_some()
        || pac.is_some()
        || enable_proxy_guard.is_some()
        || proxy_guard_duration.is_some()
    {
        update_flags.insert(UpdateFlags::SYS_PROXY);
    }
    if language.is_some() {
        update_flags.insert(UpdateFlags::LANGUAGE | UpdateFlags::SYSTRAY_MENU | UpdateFlags::SYSTRAY_TOOLTIP);
    }
    if patch.hotkeys.is_some() {
        update_flags.insert(UpdateFlags::HOTKEY | UpdateFlags::SYSTRAY_MENU);
    }
    if log_level.is_some() {
        update_flags.insert(UpdateFlags::LOG_LEVEL);
    }
    if log_max_size.is_some() || log_max_count.is_some() {
        update_flags.insert(UpdateFlags::LOG_FILE);
    }

    update_flags
}

fn should_close_connections_on_route_change(current: &IVerge, patch: &IVerge) -> bool {
    let will_disable_system_proxy =
        current.enable_system_proxy.unwrap_or(false) && patch.enable_system_proxy == Some(false);
    let will_disable_tun_mode = current.enable_tun_mode.unwrap_or(false) && patch.enable_tun_mode == Some(false);

    will_disable_system_proxy || will_disable_tun_mode
}

async fn maybe_close_connections_after_route_change(current: &IVerge, patch: &IVerge) {
    if should_close_connections_on_route_change(current, patch) {
        if let Err(error) = runtime_bridge::close_all_runtime_connections("route-change-cleanup").await {
            logging!(
                warn,
                Type::ProxyMode,
                "Failed to close runtime connections after route change: {error}"
            );
        }
    }
}

#[allow(clippy::cognitive_complexity)]
async fn process_terminated_flags(update_flags: UpdateFlags, patch: &IVerge) -> Result<()> {
    if update_flags.contains(UpdateFlags::RESTART_CORE) {
        Config::generate().await?;
        runtime_lifecycle::restart_runtime_core("verge-config-restart").await?;
    }
    if update_flags.contains(UpdateFlags::CLASH_CONFIG) {
        runtime_lifecycle::update_runtime_config_checked("verge-config-runtime").await?;
        handle::Handle::refresh_clash();
    }
    if update_flags.contains(UpdateFlags::VERGE_CONFIG) {
        Config::verge()
            .await
            .edit_draft(|d| d.enable_global_hotkey = patch.enable_global_hotkey);
        handle::Handle::refresh_verge();
    }
    if update_flags.contains(UpdateFlags::LAUNCH) {
        autostart::update_launch().await?;
    }
    if update_flags.contains(UpdateFlags::LANGUAGE)
        && let Some(language) = &patch.language
    {
        clash_verge_i18n::set_locale(language.as_str());
    }
    if update_flags.contains(UpdateFlags::SYS_PROXY) {
        sysopt::Sysopt::global().update_sysproxy().await?;
        sysopt::Sysopt::global().refresh_guard().await;
    }
    if update_flags.contains(UpdateFlags::HOTKEY)
        && let Some(hotkeys) = &patch.hotkeys
    {
        hotkey::Hotkey::global().update(hotkeys.to_owned()).await?;
    }
    if update_flags.contains(UpdateFlags::SYSTRAY_MENU) {
        tray::Tray::global().update_menu().await?;
    }
    if update_flags.contains(UpdateFlags::SYSTRAY_ICON) {
        tray::Tray::global()
            .update_icon(&Config::verge().await.latest_arc())
            .await?;
    }
    if update_flags.contains(UpdateFlags::SYSTRAY_TOOLTIP) {
        tray::Tray::global().update_tooltip().await?;
    }
    if update_flags.contains(UpdateFlags::LOG_LEVEL) {
        Logger::global().update_log_level(patch.get_log_level())?;
    }
    if update_flags.contains(UpdateFlags::LOG_FILE) {
        let log_max_size = patch.app_log_max_size.unwrap_or(128);
        let log_max_count = patch.app_log_max_count.unwrap_or(8);
        Logger::global().update_log_config(log_max_size, log_max_count).await?;
    }
    Ok(())
}

pub async fn patch_verge(patch: &IVerge, not_save_file: bool) -> Result<()> {
    let verge = Config::verge().await;
    let current_verge = verge.latest_arc();
    verge.edit_draft(|d| d.patch_config(patch));

    let update_flags = determine_update_flags(patch);
    logging!(debug, Type::Setup, "Determined update flags: {:?}", update_flags);
    let process_flag_result: std::result::Result<(), anyhow::Error> = {
        process_terminated_flags(update_flags, patch).await?;
        Ok(())
    };

    if let Err(err) = process_flag_result {
        verge.discard();
        return Err(err);
    }
    verge.apply();
    maybe_close_connections_after_route_change(current_verge.as_ref(), patch).await;
    logging_error!(Type::Backup, AutoBackupManager::global().refresh_settings().await);
    if !not_save_file {
        let verge_data = Config::verge().await.data_arc();
        logging!(debug, Type::Setup, "Saving Verge configuration to file...");
        verge_data.save_file().await?;
    }
    Ok(())
}

pub async fn fetch_verge_config() -> Result<SharedDraft<IVerge>> {
    let draft = Config::verge().await;
    let data = draft.data_arc();
    Ok(data)
}
