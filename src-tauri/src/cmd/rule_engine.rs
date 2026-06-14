use super::{CmdResult, StringifyErr};
use crate::core::rule_engine::{ConnectionMeta, RuleMatchResult, RuleProviderConfig, RuleSetData};
use std::collections::HashMap;

#[tauri::command]
pub async fn test_rule_match(
    rules: Vec<String>,
    connection: ConnectionMeta,
    rule_providers: Option<HashMap<String, RuleProviderConfig>>,
) -> CmdResult<RuleMatchResult> {
    let rule_refs: Vec<&str> = rules.iter().map(|s| s.as_str()).collect();
    let rule_sets = RuleSetData::from_rule_providers(rule_providers.unwrap_or_default()).stringify_err()?;
    let engine =
        crate::core::rule_engine::RuleEngine::from_rules_with_default_geo_data_and_rule_sets(&rule_refs, rule_sets)
            .stringify_err()?;
    Ok(engine.match_connection(&connection))
}
