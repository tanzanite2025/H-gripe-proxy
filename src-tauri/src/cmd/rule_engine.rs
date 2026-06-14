use super::{CmdResult, StringifyErr};
use crate::core::rule_engine::{ConnectionMeta, RuleMatchResult, RuleProviderConfig, RuleSetData, SubRuleData};
use std::collections::HashMap;

#[tauri::command]
pub async fn test_rule_match(
    rules: Vec<String>,
    connection: ConnectionMeta,
    rule_providers: Option<HashMap<String, RuleProviderConfig>>,
    sub_rules: Option<HashMap<String, Vec<String>>>,
) -> CmdResult<RuleMatchResult> {
    let rule_refs: Vec<&str> = rules.iter().map(|s| s.as_str()).collect();
    let rule_sets = RuleSetData::from_rule_providers(rule_providers.unwrap_or_default()).stringify_err()?;
    let sub_rules = SubRuleData::from_sub_rules(sub_rules.unwrap_or_default()).stringify_err()?;
    let engine = crate::core::rule_engine::RuleEngine::from_rules_with_default_geo_data_rule_sets_and_sub_rules(
        &rule_refs, rule_sets, sub_rules,
    )
    .stringify_err()?;
    Ok(engine.match_connection(&connection))
}
