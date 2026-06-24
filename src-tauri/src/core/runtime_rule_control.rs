use anyhow::{Result, anyhow, bail};
use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};
use std::{cmp::Reverse, fs, path::PathBuf};
use tauri_plugin_mihomo::models::{RuleExtra, Rules};

use crate::{
    config::Config,
    core::{
        rule_engine::validate_rule_spec,
        runtime_lifecycle, runtime_snapshot,
        security_policy::{PolicyRule, SECURITY_SOURCE_PREFIX},
    },
};

const RUNTIME_RULE_MUTATIONS_FILE: &str = "runtime-rule-mutations.yaml";
const LIFECYCLE_RUNTIME_RULE_MUTATION: &str = "runtime-rule-mutation";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeRuleMutationState {
    #[serde(default)]
    disabled_profile_rules: Vec<DisabledProfileRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisabledProfileRule {
    original_index: i32,
    line: String,
}

#[derive(Debug, Clone)]
enum RuntimeRuleDisplayEntry {
    Active { line: String, active_index: usize },
    Disabled { line: String, disabled_index: usize },
}

impl RuntimeRuleDisplayEntry {
    fn line(&self) -> &str {
        match self {
            RuntimeRuleDisplayEntry::Active { line, .. } | RuntimeRuleDisplayEntry::Disabled { line, .. } => line,
        }
    }

    fn is_disabled(&self) -> bool {
        matches!(self, RuntimeRuleDisplayEntry::Disabled { .. })
    }
}

pub async fn read_runtime_rules() -> Result<Rules> {
    let runtime = Config::runtime().await;
    let runtime = runtime.latest_arc();
    let config = runtime
        .config
        .as_ref()
        .ok_or_else(|| anyhow!("runtime config is not available"))?;
    let state = load_runtime_rule_mutation_state()?;
    Ok(build_rules_with_disabled_state(config, &state))
}

pub async fn disable_runtime_rules(payload: &std::collections::HashMap<i32, bool>) -> Result<()> {
    mutate_runtime_rules(
        "disable-runtime-rules",
        Some(format!("count={}", payload.len())),
        |config, state| {
            let entries = runtime_rule_display_entries(config, state)?;
            let rules = profile_rule_sequence_mut(config)?;
            let mut changes = payload.iter().collect::<Vec<_>>();
            changes.sort_by_key(|(index, _)| Reverse(**index));
            for (index, disabled) in changes {
                let index = usize::try_from(*index).map_err(|_| anyhow!("rule index must be positive"))?;
                let entry = entries
                    .get(index)
                    .ok_or_else(|| anyhow!("runtime rule index {index} is out of range"))?;
                match (disabled, entry) {
                    (true, RuntimeRuleDisplayEntry::Active { line, active_index }) => {
                        rules.remove(*active_index);
                        state.disabled_profile_rules.push(DisabledProfileRule {
                            original_index: i32::try_from(index).unwrap_or(i32::MAX),
                            line: line.clone(),
                        });
                    }
                    (false, RuntimeRuleDisplayEntry::Disabled { line, disabled_index }) => {
                        let insert_at = index.min(rules.len());
                        rules.insert(insert_at, Value::from(line.clone()));
                        state.disabled_profile_rules.remove(*disabled_index);
                    }
                    _ => {}
                }
            }
            Ok(())
        },
    )
    .await
}

pub async fn delete_runtime_rule(index: i32) -> Result<()> {
    mutate_runtime_rules(
        "delete-runtime-rule",
        Some(format!("index={index}")),
        |config, state| {
            let index = usize::try_from(index).map_err(|_| anyhow!("rule index must be positive"))?;
            let entries = runtime_rule_display_entries(config, state)?;
            let entry = entries
                .get(index)
                .ok_or_else(|| anyhow!("runtime rule index {index} is out of range"))?;
            match entry {
                RuntimeRuleDisplayEntry::Active { active_index, .. } => {
                    profile_rule_sequence_mut(config)?.remove(*active_index);
                }
                RuntimeRuleDisplayEntry::Disabled { disabled_index, .. } => {
                    state.disabled_profile_rules.remove(*disabled_index);
                }
            }
            Ok(())
        },
    )
    .await
}

pub async fn create_runtime_rule(
    rule_type: &str,
    payload: &str,
    proxy: &str,
    source: Option<&str>,
    _sub_rule: Option<&str>,
    position: Option<&str>,
) -> Result<i32> {
    let line = runtime_rule_line(rule_type, payload, proxy)?;
    let detail = Some(format!(
        "type={};source={};position={}",
        rule_type.trim(),
        source.unwrap_or("profile"),
        position.unwrap_or("append")
    ));
    mutate_runtime_rules("create-runtime-rule", detail, |config, state| {
        let index = insertion_index(position, runtime_rule_display_entries(config, state)?.len());
        profile_rule_sequence_mut(config)?.insert(index, Value::from(line.clone()));
        Ok(i32::try_from(index).unwrap_or(i32::MAX))
    })
    .await
}

pub async fn apply_security_policy_rules(policy_name: &str, rules: &[PolicyRule]) -> Result<Vec<i32>> {
    let source = format!("{SECURITY_SOURCE_PREFIX}{policy_name}");
    let lines = rules
        .iter()
        .map(|rule| runtime_rule_line(&rule.rule_type, &rule.payload, &rule.proxy))
        .collect::<Result<Vec<_>>>()?;
    mutate_runtime_rules(
        "apply-security-policy-rules",
        Some(format!("policy={policy_name};count={}", lines.len())),
        |config, state| {
            let start = runtime_rule_display_entries(config, state)?.len();
            let rules = profile_rule_sequence_mut(config)?;
            rules.extend(lines.iter().cloned().map(Value::from));
            Ok((start..start + lines.len())
                .map(|index| i32::try_from(index).unwrap_or(i32::MAX))
                .collect::<Vec<_>>())
        },
    )
    .await
    .inspect(|_| {
        runtime_snapshot::record_and_persist_runtime_lifecycle_event(
            LIFECYCLE_RUNTIME_RULE_MUTATION,
            true,
            None,
            Some(source),
        );
    })
}

pub async fn revoke_runtime_rule_indices(indices: &[i32]) -> Result<()> {
    mutate_runtime_rules(
        "revoke-security-policy-rules",
        Some(format!("count={}", indices.len())),
        |config, state| {
            let entries = runtime_rule_display_entries(config, state)?;
            let mut indices = indices.to_vec();
            indices.sort_by_key(|index| Reverse(*index));
            for index in indices {
                let index = usize::try_from(index).map_err(|_| anyhow!("rule index must be positive"))?;
                let entry = entries
                    .get(index)
                    .ok_or_else(|| anyhow!("runtime rule index {index} is out of range"))?;
                match entry {
                    RuntimeRuleDisplayEntry::Active { active_index, .. } => {
                        profile_rule_sequence_mut(config)?.remove(*active_index);
                    }
                    RuntimeRuleDisplayEntry::Disabled { disabled_index, .. } => {
                        state.disabled_profile_rules.remove(*disabled_index);
                    }
                }
            }
            Ok(())
        },
    )
    .await
}

async fn mutate_runtime_rules<T, F>(reason: &str, detail: Option<String>, mutate: F) -> Result<T>
where
    F: FnOnce(&mut Mapping, &mut RuntimeRuleMutationState) -> Result<T>,
{
    let mut state = load_runtime_rule_mutation_state()?;
    let mut output = None;
    Config::runtime().await.edit_draft(|draft| {
        let config = draft
            .config
            .as_mut()
            .ok_or_else(|| anyhow!("runtime config is not available"))?;
        output = Some(mutate(config, &mut state)?);
        Ok::<(), anyhow::Error>(())
    })?;
    let result = runtime_lifecycle::update_runtime_config_checked(reason).await;
    runtime_snapshot::record_and_persist_runtime_lifecycle_event(
        LIFECYCLE_RUNTIME_RULE_MUTATION,
        result.is_ok(),
        result.as_ref().err().map(ToString::to_string),
        detail,
    );
    result?;
    persist_runtime_rule_mutation_state(&state)?;
    output.ok_or_else(|| anyhow!("runtime rule mutation did not produce a result"))
}

fn build_rules_with_disabled_state(config: &Mapping, state: &RuntimeRuleMutationState) -> Rules {
    let entries = runtime_rule_display_entries(config, state).unwrap_or_default();
    let mut display_config = config.clone();
    display_config.insert(
        Value::from("rules"),
        Value::Sequence(
            entries
                .iter()
                .map(|entry| Value::from(entry.line().to_string()))
                .collect(),
        ),
    );
    let mut rules = runtime_snapshot::build_rules_from_runtime_config(&display_config);
    for (index, entry) in entries.iter().enumerate() {
        if let Some(rule) = rules.rules.get_mut(index)
            && entry.is_disabled()
        {
            rule.extra = Some(RuleExtra {
                disabled: true,
                deleted: false,
                hit_count: 0,
                hit_at: String::new(),
                miss_count: 0,
                miss_at: String::new(),
            });
        }
    }
    rules
}

fn runtime_rule_display_entries(
    config: &Mapping,
    state: &RuntimeRuleMutationState,
) -> Result<Vec<RuntimeRuleDisplayEntry>> {
    let mut entries = profile_rule_lines(config)?
        .into_iter()
        .enumerate()
        .map(|(active_index, line)| RuntimeRuleDisplayEntry::Active { line, active_index })
        .collect::<Vec<_>>();
    let mut disabled = state
        .disabled_profile_rules
        .iter()
        .cloned()
        .enumerate()
        .collect::<Vec<_>>();
    disabled.sort_by_key(|(_, rule)| rule.original_index);
    for (disabled_index, rule) in disabled {
        let index = usize::try_from(rule.original_index).unwrap_or(entries.len());
        entries.insert(
            index.min(entries.len()),
            RuntimeRuleDisplayEntry::Disabled {
                line: rule.line,
                disabled_index,
            },
        );
    }
    Ok(entries)
}

fn profile_rule_lines(config: &Mapping) -> Result<Vec<String>> {
    let Some(rules) = config.get("rules") else {
        return Ok(Vec::new());
    };
    let rules = rules
        .as_sequence()
        .ok_or_else(|| anyhow!("runtime rules must be a sequence"))?;
    Ok(rules
        .iter()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect())
}

fn profile_rule_sequence_mut(config: &mut Mapping) -> Result<&mut Vec<Value>> {
    if !config.contains_key("rules") {
        config.insert(Value::from("rules"), Value::Sequence(Vec::new()));
    }
    config
        .get_mut("rules")
        .and_then(Value::as_sequence_mut)
        .ok_or_else(|| anyhow!("runtime rules must be a sequence"))
}

fn runtime_rule_line(rule_type: &str, payload: &str, proxy: &str) -> Result<String> {
    let rule_type = rule_type.trim().to_ascii_uppercase();
    let payload = payload.trim();
    let proxy = proxy.trim();
    let validation = validate_rule_spec(&rule_type, payload, proxy);
    if !validation.valid {
        bail!(
            "invalid runtime rule ({rule_type},{payload},{proxy}): {}",
            validation.error.unwrap_or_else(|| "invalid rule".into())
        );
    }
    if rule_type == "MATCH" {
        Ok(format!("MATCH,{proxy}"))
    } else {
        Ok(format!("{rule_type},{payload},{proxy}"))
    }
}

fn insertion_index(position: Option<&str>, len: usize) -> usize {
    let Some(position) = position.map(str::trim).filter(|value| !value.is_empty()) else {
        return len;
    };
    match position.to_ascii_lowercase().as_str() {
        "top" | "front" | "prepend" | "before" => 0,
        "bottom" | "back" | "append" | "after" => len,
        value => value.parse::<usize>().map(|index| index.min(len)).unwrap_or(len),
    }
}

fn load_runtime_rule_mutation_state() -> Result<RuntimeRuleMutationState> {
    let path = runtime_rule_mutation_state_path()?;
    if !path.exists() {
        return Ok(RuntimeRuleMutationState::default());
    }
    let content = fs::read_to_string(path)?;
    Ok(serde_yaml_ng::from_str(&content)?)
}

fn persist_runtime_rule_mutation_state(state: &RuntimeRuleMutationState) -> Result<()> {
    let path = runtime_rule_mutation_state_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_yaml_ng::to_string(state)?)?;
    Ok(())
}

fn runtime_rule_mutation_state_path() -> Result<PathBuf> {
    Ok(crate::utils::dirs::app_runtime_dir()?.join(RUNTIME_RULE_MUTATIONS_FILE))
}
