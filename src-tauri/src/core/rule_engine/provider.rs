use super::*;
use crate::utils::dirs;
use serde::Deserialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

const RULE_SET_INTERNAL_TARGET: &str = "__RULE_SET_MATCH__";

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RuleProviderBehavior {
    Domain,
    Ipcidr,
    Classical,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RuleProviderConfig {
    #[serde(default, rename = "type")]
    pub provider_type: String,
    pub behavior: RuleProviderBehavior,
    #[serde(default)]
    pub path: Option<PathBuf>,
    #[serde(default)]
    pub payload: Vec<String>,
    #[serde(default)]
    pub format: Option<String>,
}

#[derive(Default)]
pub struct RuleSetData {
    sets: HashMap<String, RuleSetMatcher>,
}

impl RuleSetData {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_rule_providers(providers: HashMap<String, RuleProviderConfig>) -> Result<Self> {
        let mut sets = HashMap::new();
        for (name, provider) in providers {
            if let Some(matcher) = RuleSetMatcher::from_provider(&provider)
                .with_context(|| format!("failed to load rule provider {name}"))?
            {
                sets.insert(name, matcher);
            }
        }
        Ok(Self { sets })
    }

    pub(super) fn matches(&self, name: &str, meta: &ConnectionMeta) -> bool {
        self.sets.get(name).is_some_and(|matcher| matcher.matches(meta))
    }

    pub(super) fn explain(&self, name: &str, meta: &ConnectionMeta) -> Option<RuleMatchResult> {
        self.sets.get(name).map(|matcher| matcher.explain(meta))
    }
}

#[derive(Default)]
pub struct SubRuleData {
    sets: HashMap<String, SubRuleMatcher>,
}

impl SubRuleData {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn from_sub_rules(sub_rules: HashMap<String, Vec<String>>) -> Result<Self> {
        let mut sets = HashMap::new();
        for (name, raw_rules) in sub_rules {
            if name.is_empty() {
                bail!("sub-rule name cannot be empty");
            }
            let matcher =
                SubRuleMatcher::from_rules(&raw_rules).with_context(|| format!("failed to load sub-rule {name}"))?;
            sets.insert(name, matcher);
        }
        let data = Self { sets };
        data.validate_references()?;
        Ok(data)
    }

    pub(super) fn match_named<'a>(
        &'a self,
        name: &str,
        meta: &ConnectionMeta,
        host_lower: &str,
        geo_data: &'a RuleGeoData,
        rule_sets: &'a RuleSetData,
        visited: &mut HashSet<String>,
    ) -> Option<&'a str> {
        if !visited.insert(name.to_owned()) {
            return None;
        }
        let result = self
            .sets
            .get(name)
            .and_then(|matcher| matcher.matches(meta, host_lower, geo_data, rule_sets, self, visited));
        visited.remove(name);
        result
    }

    pub(super) fn explain_match_named(
        &self,
        name: &str,
        meta: &ConnectionMeta,
        host_lower: &str,
        geo_data: &RuleGeoData,
        rule_sets: &RuleSetData,
        visited: &mut HashSet<String>,
    ) -> Option<SubRuleMatchTrace> {
        if !visited.insert(name.to_owned()) {
            return None;
        }
        let result = self
            .sets
            .get(name)
            .and_then(|matcher| matcher.explain_match(meta, host_lower, geo_data, rule_sets, self, visited));
        visited.remove(name);
        result
    }

    fn validate_references(&self) -> Result<()> {
        for name in self.sets.keys() {
            self.validate_sub_rule_references(name, &mut Vec::new())?;
        }
        Ok(())
    }

    fn validate_sub_rule_references(&self, name: &str, stack: &mut Vec<String>) -> Result<()> {
        if stack.iter().any(|existing| existing == name) {
            stack.push(name.to_owned());
            bail!("sub-rule circular reference: {}", stack.join("->"));
        }
        let Some(matcher) = self.sets.get(name) else {
            bail!("sub-rule {name} not found");
        };
        stack.push(name.to_owned());
        for reference in matcher.sub_rule_references() {
            if !self.sets.contains_key(reference) {
                bail!("sub-rule {reference} not found");
            }
            self.validate_sub_rule_references(reference, stack)?;
        }
        stack.pop();
        Ok(())
    }
}

struct SubRuleMatcher {
    rules: Vec<(ParsedRule, String)>,
}

pub(super) struct SubRuleMatchTrace {
    pub(super) rule_raw: String,
    pub(super) rule_type: String,
    pub(super) target: String,
}

impl SubRuleMatcher {
    fn from_rules(raw_rules: &[String]) -> Result<Self> {
        let mut rules = Vec::with_capacity(raw_rules.len());
        for raw in raw_rules {
            rules.push((parse_rule(raw)?, raw.to_owned()));
        }
        Ok(Self { rules })
    }

    fn matches<'a>(
        &'a self,
        meta: &ConnectionMeta,
        host_lower: &str,
        geo_data: &'a RuleGeoData,
        rule_sets: &'a RuleSetData,
        sub_rules: &'a SubRuleData,
        visited: &mut HashSet<String>,
    ) -> Option<&'a str> {
        self.rules
            .iter()
            .find_map(|(rule, _)| rule_matches_inner(rule, meta, host_lower, geo_data, rule_sets, sub_rules, visited))
    }

    fn explain_match(
        &self,
        meta: &ConnectionMeta,
        host_lower: &str,
        geo_data: &RuleGeoData,
        rule_sets: &RuleSetData,
        sub_rules: &SubRuleData,
        visited: &mut HashSet<String>,
    ) -> Option<SubRuleMatchTrace> {
        self.rules.iter().find_map(|(rule, raw)| {
            rule_matches_inner(rule, meta, host_lower, geo_data, rule_sets, sub_rules, visited).map(|target| {
                SubRuleMatchTrace {
                    rule_raw: raw.clone(),
                    rule_type: rule_type_name(rule).to_owned(),
                    target: target.to_owned(),
                }
            })
        })
    }

    fn sub_rule_references(&self) -> Vec<&str> {
        let mut references = Vec::new();
        for (rule, _) in &self.rules {
            collect_sub_rule_references(rule, &mut references);
        }
        references
    }
}

fn collect_sub_rule_references<'a>(rule: &'a ParsedRule, references: &mut Vec<&'a str>) {
    match rule {
        ParsedRule::SubRule { condition, name } => {
            collect_sub_rule_references(condition, references);
            references.push(name);
        }
        ParsedRule::Logical { rules, .. } => {
            for rule in rules {
                collect_sub_rule_references(rule, references);
            }
        }
        _ => {}
    }
}

struct RuleSetMatcher {
    engine: RuleEngine,
}

impl RuleSetMatcher {
    fn from_provider(provider: &RuleProviderConfig) -> Result<Option<Self>> {
        let items = load_rule_provider_items(provider)?;
        let rules = items
            .iter()
            .filter_map(|item| match normalize_rule_set_item(provider.behavior, item) {
                Ok(Some(rule)) => Some(Ok(rule)),
                Ok(None) => None,
                Err(err) => Some(Err(err)),
            })
            .collect::<Result<Vec<_>>>()?;
        if rules.is_empty() {
            return Ok(None);
        }
        let rule_refs = rules.iter().map(String::as_str).collect::<Vec<_>>();
        let engine = RuleEngine::from_rules(&rule_refs)?;
        Ok(Some(Self { engine }))
    }

    fn matches(&self, meta: &ConnectionMeta) -> bool {
        self.engine.match_connection(meta).matched
    }

    fn explain(&self, meta: &ConnectionMeta) -> RuleMatchResult {
        self.engine.match_connection(meta)
    }
}

fn load_rule_provider_items(provider: &RuleProviderConfig) -> Result<Vec<String>> {
    if !provider.payload.is_empty() {
        return Ok(provider.payload.clone());
    }
    let provider_type = provider.provider_type.to_ascii_lowercase();
    if provider_type == "inline" {
        return Ok(Vec::new());
    }
    if !provider_type.is_empty() && provider_type != "file" && provider_type != "http" {
        return Ok(Vec::new());
    }
    let Some(path) = provider.path.as_deref().and_then(resolve_provider_path) else {
        return Ok(Vec::new());
    };
    let content =
        fs::read_to_string(&path).with_context(|| format!("failed to read rule provider file {}", path.display()))?;
    if provider
        .format
        .as_deref()
        .is_some_and(|format| format.eq_ignore_ascii_case("text"))
    {
        return Ok(parse_rule_provider_text(&content));
    }
    parse_rule_provider_file(&content)
}

fn resolve_provider_path(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }

    let mut roots = Vec::new();
    if let Ok(current_dir) = std::env::current_dir() {
        roots.push(current_dir);
    }
    if let Ok(app_home) = dirs::app_home_dir() {
        roots.push(app_home);
    }
    if let Ok(resources_dir) = dirs::app_resources_dir() {
        roots.push(resources_dir);
    }

    roots
        .into_iter()
        .map(|root| root.join(path))
        .find(|candidate| candidate.is_file())
}

#[derive(Deserialize)]
struct RuleProviderFile {
    payload: Vec<String>,
}

fn parse_rule_provider_file(content: &str) -> Result<Vec<String>> {
    if let Ok(file) = serde_yaml_ng::from_str::<RuleProviderFile>(content) {
        return Ok(file.payload);
    }
    if let Ok(payload) = serde_yaml_ng::from_str::<Vec<String>>(content) {
        return Ok(payload);
    }
    Ok(parse_rule_provider_text(content))
}

fn parse_rule_provider_text(content: &str) -> Vec<String> {
    content.lines().map(str::to_owned).collect()
}

fn normalize_rule_set_item(behavior: RuleProviderBehavior, item: &str) -> Result<Option<String>> {
    let item = item.trim();
    if item.is_empty() || item.starts_with('#') {
        return Ok(None);
    }

    let rule = match behavior {
        RuleProviderBehavior::Domain => normalize_domain_provider_item(item),
        RuleProviderBehavior::Ipcidr => normalize_ipcidr_provider_item(item),
        RuleProviderBehavior::Classical => normalize_classical_provider_item(item)?,
    };

    let parts = parse_rule_payload(&rule, true)?;
    if parts.rule_type.eq_ignore_ascii_case("RULE-SET") {
        bail!("nested RULE-SET providers are not supported");
    }
    parse_rule(&rule)?;
    Ok(Some(rule))
}

fn normalize_domain_provider_item(item: &str) -> String {
    if item.contains(',') {
        append_or_replace_rule_target(item)
    } else {
        format!("DOMAIN-SUFFIX,{item},{RULE_SET_INTERNAL_TARGET}")
    }
}

fn normalize_ipcidr_provider_item(item: &str) -> String {
    if item.contains(',') {
        append_or_replace_rule_target(item)
    } else {
        format!("IP-CIDR,{item},{RULE_SET_INTERNAL_TARGET}")
    }
}

fn normalize_classical_provider_item(item: &str) -> Result<String> {
    if !item.contains(',') {
        bail!("classical RULE-SET item must include a rule type and payload");
    }
    Ok(append_or_replace_rule_target(item))
}

fn append_or_replace_rule_target(item: &str) -> String {
    let parts = parse_rule_payload(item, true).unwrap_or_else(|_| RuleParts {
        rule_type: item.to_owned(),
        payload: String::new(),
        target: String::new(),
        params: Vec::new(),
    });
    if parts.rule_type == "MATCH" {
        return format!("MATCH,{RULE_SET_INTERNAL_TARGET}");
    }
    let mut rule = format!("{},{},{}", parts.rule_type, parts.payload, RULE_SET_INTERNAL_TARGET);
    if !parts.params.is_empty() {
        rule.push(',');
        rule.push_str(&parts.params.join(","));
    }
    rule
}
