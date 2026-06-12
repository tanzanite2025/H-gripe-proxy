use crate::{
    core::{
        CoreManager, handle,
        validate::{ValidationNoticeTarget, ValidationOutcome, handle_validation_notice},
    },
    utils::{dirs, tmpl},
};
use anyhow::{Context as _, Result, bail};
use clash_verge_logging::{Type, logging};
use serde_yaml_ng::{Mapping, Sequence, Value};
use std::{collections::HashSet, path::PathBuf};
use tokio::fs;

const CHINA_RULES_FILE_TYPE: &str = "china rules";

async fn ensure_china_rules_file() -> Result<PathBuf> {
    let path = dirs::china_rules_path()?;

    if !path.exists() {
        fs::write(&path, tmpl::CHINA_RULES_TEMPLATE).await?;
        logging!(info, Type::Config, "Created built-in china rules file at {:?}", path);
    }

    Ok(path)
}

fn parse_china_rules_text(file_data: &str) -> Result<Mapping> {
    let trimmed = file_data.trim();
    let mapping = if trimmed.is_empty() {
        Mapping::new()
    } else {
        serde_yaml_ng::from_str::<Mapping>(file_data).context("china-rules.yaml must be a YAML mapping")?
    };

    for key in mapping.keys() {
        let Some(key) = key.as_str() else {
            bail!("china-rules.yaml top-level keys must be strings");
        };

        if key != "rules" {
            bail!("china-rules.yaml only supports top-level `rules`");
        }
    }

    if let Some(rules) = mapping.get("rules") {
        if !rules.is_sequence() {
            bail!("`rules` in china-rules.yaml must be a sequence");
        }
        // Validate each rule through the Rust rule engine — single source of truth
        if let Some(seq) = rules.as_sequence() {
            for (i, item) in seq.iter().enumerate() {
                if let Some(rule_str) = item.as_str() {
                    let v = crate::core::rule_engine::validate_rule(rule_str);
                    if !v.valid {
                        bail!(
                            "china-rules.yaml rules[{i}]: {}",
                            v.error.unwrap_or_else(|| "invalid rule".into())
                        );
                    }
                }
            }
        }
    }

    Ok(mapping)
}

fn is_terminal_rule(rule: &str) -> bool {
    matches!(
        rule.split(',').next().map(|part| part.trim().to_ascii_uppercase()),
        Some(head) if head == "MATCH" || head == "FINAL"
    )
}

fn dedupe_rule_values(existing_rules: &Sequence, china_rules: Sequence) -> Sequence {
    let mut seen = existing_rules
        .iter()
        .filter_map(Value::as_str)
        .map(|rule| rule.trim().to_owned())
        .collect::<HashSet<_>>();

    china_rules
        .into_iter()
        .filter(|rule| {
            let Some(rule) = rule.as_str() else {
                return true;
            };

            seen.insert(rule.trim().to_owned())
        })
        .collect()
}

fn merge_china_rules_sequence(existing_rules: Sequence, china_rules: Sequence) -> Sequence {
    let china_rules = dedupe_rule_values(&existing_rules, china_rules);
    if china_rules.is_empty() {
        return existing_rules;
    }

    let insert_at = existing_rules
        .iter()
        .position(|rule| rule.as_str().is_some_and(is_terminal_rule))
        .unwrap_or(existing_rules.len());

    let mut merged = Sequence::new();
    merged.extend(existing_rules.iter().take(insert_at).cloned());
    merged.extend(china_rules);
    merged.extend(existing_rules.into_iter().skip(insert_at));
    merged
}

pub async fn read_china_rules_file() -> Result<String> {
    let path = ensure_china_rules_file().await?;
    Ok(fs::read_to_string(path).await?)
}

pub async fn save_china_rules_file(file_data: Option<String>) -> Result<ValidationOutcome> {
    let file_data = match file_data {
        Some(file_data) => file_data,
        None => return Ok(ValidationOutcome::Valid),
    };

    parse_china_rules_text(&file_data)?;

    let path = ensure_china_rules_file().await?;
    let original_content = fs::read_to_string(&path).await.unwrap_or_default();

    fs::write(&path, &file_data).await?;

    match CoreManager::global().update_config_forced().await {
        Ok(outcome) if outcome.is_valid() => {
            handle::Handle::refresh_clash();
            Ok(ValidationOutcome::Valid)
        }
        Ok(outcome) => {
            fs::write(&path, &original_content).await?;
            handle_validation_notice(&outcome, ValidationNoticeTarget::Runtime, CHINA_RULES_FILE_TYPE);
            Ok(outcome)
        }
        Err(err) => {
            fs::write(&path, &original_content).await?;
            Err(err)
        }
    }
}

pub async fn apply_global_china_rules(mut config: Mapping) -> Mapping {
    let path = match ensure_china_rules_file().await {
        Ok(path) => path,
        Err(err) => {
            logging!(warn, Type::Config, "Failed to resolve china rules file: {}", err);
            return config;
        }
    };

    let data = match fs::read_to_string(&path).await {
        Ok(data) => data,
        Err(err) => {
            logging!(warn, Type::Config, "Failed to read china rules file: {}", err);
            return config;
        }
    };

    let mapping = match parse_china_rules_text(&data) {
        Ok(mapping) => mapping,
        Err(err) => {
            logging!(warn, Type::Config, "Ignored invalid china rules file: {}", err);
            return config;
        }
    };

    let Some(china_rules) = mapping.get("rules").and_then(Value::as_sequence).cloned() else {
        return config;
    };

    if china_rules.is_empty() {
        return config;
    }

    let existing_rules = config
        .get("rules")
        .and_then(Value::as_sequence)
        .cloned()
        .unwrap_or_default();

    config.insert(
        "rules".into(),
        Value::Sequence(merge_china_rules_sequence(existing_rules, china_rules)),
    );
    config
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::tmpl;

    #[test]
    fn merge_china_rules_inserts_before_terminal_rule() {
        let existing = Sequence::from(vec![
            Value::from("DOMAIN-SUFFIX,openai.com,Proxy"),
            Value::from("MATCH,GLOBAL"),
        ]);
        let china_rules = Sequence::from(vec![
            Value::from("GEOSITE,CN,DIRECT"),
            Value::from("GEOIP,CN,DIRECT,no-resolve"),
        ]);

        let merged = merge_china_rules_sequence(existing, china_rules);
        let merged = merged.iter().filter_map(Value::as_str).collect::<Vec<_>>();

        assert_eq!(
            merged,
            vec![
                "DOMAIN-SUFFIX,openai.com,Proxy",
                "GEOSITE,CN,DIRECT",
                "GEOIP,CN,DIRECT,no-resolve",
                "MATCH,GLOBAL",
            ]
        );
    }

    #[test]
    fn parse_china_rules_rejects_non_rules_top_level_keys() {
        assert!(parse_china_rules_text("proxies:\n  - a").is_err());
    }

    #[test]
    fn china_rules_template_stays_domestic_only() {
        let mapping = parse_china_rules_text(tmpl::CHINA_RULES_TEMPLATE).unwrap();
        let rules = mapping
            .get("rules")
            .and_then(Value::as_sequence)
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .collect::<Vec<_>>();

        assert!(rules.contains(&"DOMAIN-SUFFIX,bilibili.com,DIRECT"));
        assert!(rules.contains(&"DOMAIN-SUFFIX,unionpay.com,DIRECT"));
        assert!(rules.contains(&"GEOSITE,CN,DIRECT"));
        assert!(rules.contains(&"GEOIP,CN,DIRECT,no-resolve"));

        for forbidden in ["google", "openai", "youtube", "telegram", "github"] {
            assert!(
                !tmpl::CHINA_RULES_TEMPLATE.to_ascii_lowercase().contains(forbidden),
                "china rules template must not contain overseas routing target: {forbidden}"
            );
        }
    }
}
