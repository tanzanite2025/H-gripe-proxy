use super::{
    CmdResult,
    coordinator::{get_coordinator, sync_coordinator_from_advanced_config},
    ip_reputation::get_ip_reputation_manager,
    session_affinity::get_session_affinity_manager,
};
use crate::{
    config::Config,
    core::{
        stable_egress::sync_runtime_stable_egress_selection as core_sync_stable_egress,
        tray::Tray,
    },
    process::AsyncHandler,
};
use clash_verge_logging::{Type, logging};
use std::sync::atomic::{AtomicBool, Ordering};

static TRAY_SYNC_RUNNING: AtomicBool = AtomicBool::new(false);
static TRAY_SYNC_PENDING: AtomicBool = AtomicBool::new(false);

/// 同步托盘和GUI的代理选择状态
#[tauri::command]
pub async fn sync_tray_proxy_selection() -> CmdResult<()> {
    if TRAY_SYNC_RUNNING
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_ok()
    {
        AsyncHandler::spawn(move || async move {
            run_tray_sync_loop().await;
        });
    } else {
        TRAY_SYNC_PENDING.store(true, Ordering::Release);
    }

    Ok(())
}

pub async fn sync_runtime_stable_egress_selection() -> Result<(), String> {
    sync_coordinator_from_advanced_config()?;

    let runtime_config = Config::runtime().await.latest_arc().config.clone();
    let Some(runtime_config) = runtime_config else {
        return Ok(());
    };

    let coordinator = get_coordinator();
    let session_affinity_manager = get_session_affinity_manager();
    let ip_reputation_manager = get_ip_reputation_manager();

    core_sync_stable_egress(
        &coordinator,
        &session_affinity_manager,
        &ip_reputation_manager,
        &runtime_config,
    )
    .await
    .map_err(|e| e.to_string())
}

async fn run_tray_sync_loop() {
    loop {
        if let Err(error) = sync_runtime_stable_egress_selection().await {
            logging!(
                error,
                Type::Cmd,
                "Failed to sync stable egress runtime selection state: {error}"
            );
        }

        match Tray::global().update_menu().await {
            Ok(_) => {
                logging!(info, Type::Cmd, "Tray proxy selection synced successfully");
            }
            Err(e) => {
                logging!(error, Type::Cmd, "Failed to sync tray proxy selection: {e}");
            }
        }

        if !TRAY_SYNC_PENDING.swap(false, Ordering::AcqRel) {
            TRAY_SYNC_RUNNING.store(false, Ordering::Release);

            if TRAY_SYNC_PENDING.swap(false, Ordering::AcqRel)
                && TRAY_SYNC_RUNNING
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok()
            {
                continue;
            }

            break;
        }
    }
}
