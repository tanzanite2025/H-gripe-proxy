use anyhow::{Result, anyhow};
use smartstring::alias::String;
use tokio::sync::Mutex;

use once_cell::sync::Lazy;

use crate::core::{handle, runtime_snapshot::read_runtime_version};

static MIHOMO_RECOVERY_LOCK: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

pub async fn ensure_mihomo_core_ready() -> Result<()> {
    let _guard = MIHOMO_RECOVERY_LOCK.lock().await;

    read_runtime_version()
        .await
        .map_err(|err| anyhow!("Rust runtime is not ready: {err}"))?;
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

pub struct MihomoRuleGuard;

impl MihomoRuleGuard {
    pub async fn create(
        _app_handle: &tauri::AppHandle,
        rules: &[MihomoRuntimeRuleSpec],
        _source: Option<&str>,
        _position: Option<&str>,
    ) -> Result<Self> {
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

        Err(anyhow!(
            "Mihomo runtime rule mutation through the Go plugin API is retired; use the Rust runtime rule path"
        ))
    }

    pub async fn restore(self) -> Result<()> {
        Ok(())
    }
}

pub struct MihomoSelectionGuard;

impl MihomoSelectionGuard {
    pub async fn select(_app_handle: &tauri::AppHandle, _group_name: &str, _node_name: &str) -> Result<Self> {
        Err(anyhow!(
            "Mihomo proxy selection through the Go plugin API is retired; use the Rust runtime selection path"
        ))
    }

    pub async fn restore(self) -> Result<()> {
        Ok(())
    }
}
