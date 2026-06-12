use super::{CmdResult, StringifyErr};
use crate::core::rule_engine::{ConnectionMeta, RuleMatchResult};

#[tauri::command]
pub async fn test_rule_match(rules: Vec<String>, connection: ConnectionMeta) -> CmdResult<RuleMatchResult> {
    let rule_refs: Vec<&str> = rules.iter().map(|s| s.as_str()).collect();
    let engine = crate::core::rule_engine::RuleEngine::from_rules_with_default_geo_data(&rule_refs).stringify_err()?;
    Ok(engine.match_connection(&connection))
}
