use anyhow::{Context, Result};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDiffReport {
    pub changed: bool,
    pub explanation: String,
    pub changes: Vec<ConfigDiffChange>,
    pub section_summaries: Vec<ConfigSectionSummary>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ConfigDiffChangeType {
    Added,
    Removed,
    Modified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigDiffChange {
    pub path: String,
    pub change_type: ConfigDiffChangeType,
    pub before_type: Option<String>,
    pub after_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigSectionSummary {
    pub path: String,
    pub before_count: Option<usize>,
    pub after_count: Option<usize>,
    pub delta: Option<isize>,
}

pub fn explain_config_diff(before_yaml: &str, after_yaml: &str) -> Result<ConfigDiffReport> {
    let before = parse_config_mapping(before_yaml).context("failed to parse before config")?;
    let after = parse_config_mapping(after_yaml).context("failed to parse after config")?;

    let mut changes = Vec::new();
    diff_mapping("", &before, &after, &mut changes);
    let section_summaries = summarize_sections(&before, &after);
    let changed = !changes.is_empty();
    let explanation = if changed {
        format!(
            "{} config path(s) changed; {} tracked section(s) summarized",
            changes.len(),
            section_summaries.len()
        )
    } else {
        "config has no semantic YAML diff".to_owned()
    };

    Ok(ConfigDiffReport {
        changed,
        explanation,
        changes,
        section_summaries,
    })
}

fn parse_config_mapping(yaml: &str) -> Result<Mapping> {
    let value: Value = serde_yaml_ng::from_str(yaml).context("YAML syntax error")?;
    value
        .as_mapping()
        .cloned()
        .context("config root must be a YAML mapping")
}

fn diff_mapping(path: &str, before: &Mapping, after: &Mapping, changes: &mut Vec<ConfigDiffChange>) {
    let before_keys = mapping_by_key(before);
    let after_keys = mapping_by_key(after);
    let keys: BTreeSet<&str> = before_keys
        .keys()
        .chain(after_keys.keys())
        .map(String::as_str)
        .collect();

    for key in keys {
        let child_path = join_path(path, key);
        match (before_keys.get(key), after_keys.get(key)) {
            (None, Some(after)) => changes.push(ConfigDiffChange {
                path: child_path,
                change_type: ConfigDiffChangeType::Added,
                before_type: None,
                after_type: Some(value_type(after).to_owned()),
            }),
            (Some(before), None) => changes.push(ConfigDiffChange {
                path: child_path,
                change_type: ConfigDiffChangeType::Removed,
                before_type: Some(value_type(before).to_owned()),
                after_type: None,
            }),
            (Some(before), Some(after)) if before != after => diff_value(child_path, before, after, changes),
            _ => {}
        }
    }
}

fn diff_value(path: String, before: &Value, after: &Value, changes: &mut Vec<ConfigDiffChange>) {
    match (before, after) {
        (Value::Mapping(before_map), Value::Mapping(after_map)) if path_depth(&path) < 3 => {
            diff_mapping(&path, before_map, after_map, changes);
        }
        _ => changes.push(ConfigDiffChange {
            path,
            change_type: ConfigDiffChangeType::Modified,
            before_type: Some(value_type(before).to_owned()),
            after_type: Some(value_type(after).to_owned()),
        }),
    }
}

fn summarize_sections(before: &Mapping, after: &Mapping) -> Vec<ConfigSectionSummary> {
    let before_value = Value::Mapping(before.clone());
    let after_value = Value::Mapping(after.clone());
    [
        "proxies",
        "proxy-groups",
        "rule-providers",
        "rules",
        "listeners",
        "dns.nameserver",
        "dns.fallback",
        "dns.nameserver-policy",
    ]
    .into_iter()
    .filter_map(|path| summarize_section(path, &before_value, &after_value))
    .collect()
}

fn summarize_section(path: &str, before: &Value, after: &Value) -> Option<ConfigSectionSummary> {
    let before_count = lookup_path(before, path).and_then(collection_len);
    let after_count = lookup_path(after, path).and_then(collection_len);

    if before_count.is_none() && after_count.is_none() {
        return None;
    }

    Some(ConfigSectionSummary {
        path: path.to_owned(),
        before_count,
        after_count,
        delta: match (before_count, after_count) {
            (Some(before), Some(after)) => Some(after as isize - before as isize),
            _ => None,
        },
    })
}

fn lookup_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(value, |current, segment| {
        current.as_mapping()?.get(Value::String(segment.to_owned()))
    })
}

fn collection_len(value: &Value) -> Option<usize> {
    match value {
        Value::Sequence(seq) => Some(seq.len()),
        Value::Mapping(map) => Some(map.len()),
        _ => None,
    }
}

fn mapping_by_key(map: &Mapping) -> BTreeMap<String, &Value> {
    map.iter().map(|(key, value)| (mapping_key_name(key), value)).collect()
}

fn mapping_key_name(key: &Value) -> String {
    key.as_str().map(str::to_owned).unwrap_or_else(|| {
        serde_yaml_ng::to_string(key)
            .unwrap_or_else(|_| "<non-string-key>".to_owned())
            .trim()
            .to_owned()
    })
}

fn join_path(parent: &str, key: &str) -> String {
    if parent.is_empty() {
        key.to_owned()
    } else {
        format!("{parent}.{key}")
    }
}

fn path_depth(path: &str) -> usize {
    path.split('.').count()
}

fn value_type(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "sequence",
        Value::Mapping(_) => "mapping",
        Value::Tagged(_) => "tagged",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_diff_reports_added_removed_and_modified_paths() {
        let before = r#"
mode: rule
mixed-port: 7890
proxies:
  - name: A
    type: ss
rules:
  - MATCH,DIRECT
"#;
        let after = r#"
mode: global
socks-port: 7891
proxies:
  - name: A
    type: ss
  - name: B
    type: direct
rules:
  - DOMAIN,example.com,Proxy
  - MATCH,DIRECT
"#;

        let report = explain_config_diff(before, after).unwrap();

        assert!(report.changed);
        assert!(report.changes.iter().any(|change| change.path == "mode"));
        assert!(
            report
                .changes
                .iter()
                .any(|change| change.path == "mixed-port" && change.change_type == ConfigDiffChangeType::Removed)
        );
        assert!(
            report
                .changes
                .iter()
                .any(|change| change.path == "socks-port" && change.change_type == ConfigDiffChangeType::Added)
        );
        assert_eq!(
            report
                .section_summaries
                .iter()
                .find(|summary| summary.path == "proxies")
                .and_then(|summary| summary.delta),
            Some(1)
        );
        assert_eq!(
            report
                .section_summaries
                .iter()
                .find(|summary| summary.path == "rules")
                .and_then(|summary| summary.delta),
            Some(1)
        );
    }

    #[test]
    fn config_diff_reports_no_semantic_change() {
        let yaml = "mode: rule\nproxies: []\n";
        let report = explain_config_diff(yaml, yaml).unwrap();

        assert!(!report.changed);
        assert!(report.changes.is_empty());
        assert_eq!(report.explanation, "config has no semantic YAML diff");
    }

    #[test]
    fn config_diff_rejects_non_mapping_roots() {
        let err = explain_config_diff("- rule\n", "mode: rule\n").unwrap_err();

        assert!(err.to_string().contains("before config"));
    }
}
