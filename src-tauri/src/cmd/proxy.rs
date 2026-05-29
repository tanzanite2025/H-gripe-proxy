use super::{
    CmdResult,
    coordinator::{get_coordinator, sync_coordinator_from_advanced_config},
    egress_identity::enrich_egress_selection_context,
    session_affinity::get_session_affinity_manager,
};
use crate::{
    config::Config,
    core::{
        egress_identity::EgressSelectionContext,
        handle,
        stable_egress::{collect_stable_group_patterns, domain_probe_for_pattern},
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

    let stable_group_patterns = collect_stable_group_patterns(&runtime_config);
    if stable_group_patterns.is_empty() {
        return Ok(());
    }

    let proxies = handle::Handle::mihomo()
        .await
        .get_proxies()
        .await
        .map_err(|error| error.to_string())?;
    let coordinator = get_coordinator();
    let session_affinity_manager = get_session_affinity_manager();

    for (group_name, domain_patterns) in stable_group_patterns {
        let Some(group_data) = proxies.proxies.get(group_name.as_str()) else {
            continue;
        };

        let Some(selected_node) = group_data
            .now
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
        else {
            continue;
        };

        let available_nodes = with_selected_node(
            group_data.all.clone().unwrap_or_default(),
            &selected_node,
        );

        if available_nodes.is_empty() {
            continue;
        }

        for domain_pattern in domain_patterns {
            let Some(domain_probe) = domain_probe_for_pattern(&domain_pattern) else {
                continue;
            };

            let egress_context = enrich_egress_selection_context(EgressSelectionContext {
                domain: Some(domain_probe),
                available_nodes: available_nodes.clone(),
                ..Default::default()
            })
            .await;

            if let Err(error) = coordinator.egress_identity_manager().record_domain_override(
                &domain_pattern,
                egress_context,
                selected_node.clone(),
            ) {
                logging!(
                    warn,
                    Type::Cmd,
                    "Failed to backwrite stable egress selection into egress identity for {} -> {}: {}",
                    domain_pattern,
                    selected_node,
                    error
                );
            }

            if let Err(error) = session_affinity_manager
                .record_domain_rule_binding(&domain_pattern, selected_node.clone())
                .await
            {
                logging!(
                    warn,
                    Type::Cmd,
                    "Failed to backwrite stable egress selection into session affinity for {} -> {}: {}",
                    domain_pattern,
                    selected_node,
                    error
                );
            }
        }
    }

    Ok(())
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

fn with_selected_node(mut available_nodes: Vec<String>, selected_node: &str) -> Vec<String> {
    if !available_nodes.iter().any(|node| node == selected_node) {
        available_nodes.insert(0, selected_node.to_string());
    }

    available_nodes
}
