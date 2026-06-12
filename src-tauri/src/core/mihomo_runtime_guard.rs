use anyhow::{Result, anyhow};
use smartstring::alias::String;
use std::time::Duration;
use tauri_plugin_mihomo::Error as MihomoError;
use tauri_plugin_mihomo::MihomoExt as _;
use tokio::sync::Mutex;

use once_cell::sync::Lazy;

use crate::core::{CoreManager, handle};
use clash_verge_logging::{Type, logging};

static MIHOMO_RECOVERY_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

async fn probe_mihomo_ipc() -> Result<(), MihomoError> {
    handle::Handle::mihomo().await.get_version().await.map(|_| ())
}

pub async fn ensure_mihomo_core_ready() -> Result<()> {
    let _guard = MIHOMO_RECOVERY_LOCK.lock().await;

    handle::Handle::sync_mihomo_controller_state().await?;

    match probe_mihomo_ipc().await {
        Ok(()) => return Ok(()),
        Err(err) if !CoreManager::is_mihomo_ipc_unavailable(&err) => {
            return Err(anyhow!("Mihomo IPC probe failed: {err}"));
        }
        Err(err) => {
            let running_mode = CoreManager::global().get_running_mode();
            logging!(
                warn,
                Type::Core,
                "Mihomo IPC is unavailable while checking readiness (mode: {}). Attempting recovery: {}",
                running_mode,
                err
            );

            match &*running_mode {
                crate::core::manager::RunningMode::NotRunning => {
                    CoreManager::global().start_core().await?;
                }
                crate::core::manager::RunningMode::Sidecar | crate::core::manager::RunningMode::Service => {
                    CoreManager::global().restart_core().await?;
                }
            }
        }
    }

    tokio::time::sleep(Duration::from_millis(250)).await;
    handle::Handle::sync_mihomo_controller_state().await?;
    probe_mihomo_ipc()
        .await
        .map_err(|err| anyhow!("Mihomo IPC is still unavailable after recovery: {err}"))?;
    handle::Handle::refresh_clash();

    Ok(())
}

#[derive(Debug, Clone)]
pub struct MihomoRuntimeRuleSpec {
    pub rule_type: String,
    pub payload: String,
    pub proxy: String,
    pub sub_rule: Option<String>,
}

impl MihomoRuntimeRuleSpec {
    pub fn new(rule_type: impl Into<String>, payload: impl Into<String>, proxy: impl Into<String>) -> Self {
        Self {
            rule_type: rule_type.into(),
            payload: payload.into(),
            proxy: proxy.into(),
            sub_rule: None,
        }
    }
}

pub struct MihomoRuleGuard<'a> {
    app_handle: &'a tauri::AppHandle,
    rule_indexes: Vec<i32>,
}

impl<'a> MihomoRuleGuard<'a> {
    pub async fn create(
        app_handle: &'a tauri::AppHandle,
        rules: &[MihomoRuntimeRuleSpec],
        source: Option<&str>,
        position: Option<&str>,
    ) -> Result<Self> {
        // Validate every rule through the Rust rule engine before sending to Go
        for (i, rule) in rules.iter().enumerate() {
            let v = super::rule_engine::validate_rule_spec(&rule.rule_type, &rule.payload, &rule.proxy);
            if !v.valid {
                return Err(anyhow::anyhow!(
                    "runtime rule[{i}] ({},{},{}): {}",
                    rule.rule_type,
                    rule.payload,
                    rule.proxy,
                    v.error.unwrap_or_else(|| "invalid rule".into())
                ));
            }
        }

        let mihomo = app_handle.mihomo().read().await;
        let mut rule_indexes = Vec::new();

        for rule in rules {
            let index = mihomo
                .create_rule(
                    rule.rule_type.as_str(),
                    rule.payload.as_str(),
                    rule.proxy.as_str(),
                    source,
                    rule.sub_rule.as_deref(),
                    position,
                )
                .await?;
            if index >= 0 {
                rule_indexes.push(index);
            }
        }

        drop(mihomo);

        Ok(Self {
            app_handle,
            rule_indexes,
        })
    }

    pub async fn restore(self) -> Result<()> {
        let mihomo = self.app_handle.mihomo().read().await;
        for index in self.rule_indexes.into_iter().rev() {
            mihomo.delete_rule(index).await?;
        }

        Ok(())
    }
}

pub struct MihomoSelectionGuard<'a> {
    app_handle: &'a tauri::AppHandle,
    group_name: String,
    previous_node: Option<String>,
}

impl<'a> MihomoSelectionGuard<'a> {
    pub async fn select(app_handle: &'a tauri::AppHandle, group_name: &str, node_name: &str) -> Result<Self> {
        let mihomo = app_handle.mihomo().read().await;
        let target = mihomo.get_proxy_by_name(node_name).await?;
        if !target.alive {
            return Err(anyhow!("mihomo proxy {node_name} is not alive"));
        }

        let group = mihomo.get_group_by_name(group_name).await?;
        let previous_node = group.now.clone().filter(|node| !node.is_empty());
        let selectable = group
            .all
            .as_ref()
            .map(|nodes| nodes.iter().any(|node| node == node_name))
            .unwrap_or(false);
        if !selectable {
            return Err(anyhow!("mihomo group {group_name} does not contain {node_name}"));
        }

        mihomo.select_node_for_group(group_name, node_name).await?;
        drop(mihomo);

        Ok(Self {
            app_handle,
            group_name: group_name.to_string().into(),
            previous_node: previous_node.map(Into::into),
        })
    }

    pub async fn restore(self) -> Result<()> {
        if let Some(previous_node) = self.previous_node {
            let mihomo = self.app_handle.mihomo().read().await;
            mihomo.select_node_for_group(&self.group_name, &previous_node).await?;
        }

        Ok(())
    }
}
