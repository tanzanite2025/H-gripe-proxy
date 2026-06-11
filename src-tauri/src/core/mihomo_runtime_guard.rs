use anyhow::{Result, anyhow};
use smartstring::alias::String;
use tauri_plugin_mihomo::MihomoExt as _;

#[derive(Debug, Clone)]
pub struct MihomoRuntimeRuleSpec {
    pub rule_type: String,
    pub payload: String,
    pub proxy: String,
    pub sub_rule: Option<String>,
}

impl MihomoRuntimeRuleSpec {
    pub fn new(
        rule_type: impl Into<String>,
        payload: impl Into<String>,
        proxy: impl Into<String>,
    ) -> Self {
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
