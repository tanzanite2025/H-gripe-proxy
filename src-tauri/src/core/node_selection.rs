use serde::{Deserialize, Serialize};
use smartstring::alias::String;

const DEFAULT_URL_TEST_TOLERANCE_MS: u32 = 0;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelectionPlanRequest {
    pub group_name: String,
    #[serde(default)]
    pub group_type: Option<String>,
    #[serde(default)]
    pub current: Option<String>,
    #[serde(default)]
    pub requested: Option<String>,
    #[serde(default)]
    pub tolerance_ms: Option<u32>,
    #[serde(default)]
    pub candidates: Vec<NodeSelectionCandidateInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelectionCandidateInput {
    pub name: String,
    #[serde(default)]
    pub proxy_type: Option<String>,
    #[serde(default)]
    pub alive: Option<bool>,
    #[serde(default)]
    pub delay_ms: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeSelectionPlanStatus {
    Ready,
    Noop,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelectionPlan {
    pub status: NodeSelectionPlanStatus,
    pub reason: String,
    pub group_name: String,
    pub group_type: String,
    pub selected: Option<String>,
    pub current: Option<String>,
    pub should_apply_runtime: bool,
    pub should_sync_tray: bool,
    pub candidates: Vec<NodeSelectionCandidatePlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeSelectionCandidatePlan {
    pub name: String,
    pub proxy_type: Option<String>,
    pub eligible: bool,
    pub reason: String,
    pub alive: Option<bool>,
    pub delay_ms: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SelectionStrategy {
    Manual,
    UrlTest,
    Fallback,
    LoadBalance,
    Relay,
    Unknown,
}

pub fn build_node_selection_plan(request: NodeSelectionPlanRequest) -> NodeSelectionPlan {
    let group_name = normalize_name(&request.group_name);
    let group_type = normalize_group_type(request.group_type.as_deref());
    let strategy = SelectionStrategy::from_group_type(&group_type);
    let current = request
        .current
        .as_deref()
        .map(normalize_name)
        .filter(|name| !name.is_empty());
    let requested = request
        .requested
        .as_deref()
        .map(normalize_name)
        .filter(|name| !name.is_empty());
    let candidates = normalize_candidates(request.candidates);

    if group_name.is_empty() {
        return rejected_plan("missing group name", group_name, group_type, current, candidates);
    }

    if candidates.is_empty() {
        return rejected_plan("no selectable candidates", group_name, group_type, current, candidates);
    }

    let selected = match requested.as_deref() {
        Some(name) if !candidates.iter().any(|candidate| candidate.name == name) => {
            return rejected_plan(
                format!("requested node `{name}` is not in group `{group_name}`"),
                group_name,
                group_type,
                current,
                candidates,
            );
        }
        Some(name) => name.into(),
        None => select_without_request(strategy, current.as_deref(), &candidates, request.tolerance_ms),
    };

    let candidate_plans = explain_candidates(strategy, &candidates, selected.as_str());
    let selected_is_current = current.as_ref() == Some(&selected);
    let reason = if selected_is_current {
        format!("group `{group_name}` already selects `{selected}`")
    } else if requested.is_some() {
        format!("apply requested node `{selected}` to group `{group_name}`")
    } else {
        format!(
            "recommended `{selected}` for `{}` group `{group_name}`",
            strategy.label()
        )
    };

    NodeSelectionPlan {
        status: if selected_is_current {
            NodeSelectionPlanStatus::Noop
        } else {
            NodeSelectionPlanStatus::Ready
        },
        reason: reason.into(),
        group_name,
        group_type,
        selected: Some(selected),
        current,
        should_apply_runtime: !selected_is_current,
        should_sync_tray: !selected_is_current,
        candidates: candidate_plans,
    }
}

fn rejected_plan(
    reason: impl Into<String>,
    group_name: String,
    group_type: String,
    current: Option<String>,
    candidates: Vec<NodeSelectionCandidateInput>,
) -> NodeSelectionPlan {
    NodeSelectionPlan {
        status: NodeSelectionPlanStatus::Rejected,
        reason: reason.into(),
        group_name,
        group_type,
        selected: None,
        current,
        should_apply_runtime: false,
        should_sync_tray: false,
        candidates: explain_candidates(SelectionStrategy::Unknown, &candidates, ""),
    }
}

fn select_without_request(
    strategy: SelectionStrategy,
    current: Option<&str>,
    candidates: &[NodeSelectionCandidateInput],
    tolerance_ms: Option<u32>,
) -> String {
    match strategy {
        SelectionStrategy::UrlTest => select_url_test(current, candidates, tolerance_ms),
        SelectionStrategy::Fallback => select_fallback(current, candidates),
        SelectionStrategy::LoadBalance
        | SelectionStrategy::Relay
        | SelectionStrategy::Manual
        | SelectionStrategy::Unknown => current
            .filter(|name| candidates.iter().any(|candidate| candidate.name == *name))
            .unwrap_or(candidates[0].name.as_str())
            .into(),
    }
}

fn select_url_test(
    current: Option<&str>,
    candidates: &[NodeSelectionCandidateInput],
    tolerance_ms: Option<u32>,
) -> String {
    let alive_candidates = candidates
        .iter()
        .filter(|candidate| candidate.alive.unwrap_or(true))
        .collect::<Vec<_>>();
    let pool = if alive_candidates.is_empty() {
        candidates.iter().collect::<Vec<_>>()
    } else {
        alive_candidates
    };
    let fastest = pool
        .iter()
        .min_by_key(|candidate| candidate.delay_ms.unwrap_or(u32::MAX))
        .copied()
        .unwrap_or(&candidates[0]);
    let tolerance = tolerance_ms.unwrap_or(DEFAULT_URL_TEST_TOLERANCE_MS);

    if let Some(current) = current
        && let Some(current_candidate) = pool.iter().find(|candidate| candidate.name == current)
    {
        let current_delay = current_candidate.delay_ms.unwrap_or(u32::MAX);
        let fastest_delay = fastest.delay_ms.unwrap_or(u32::MAX);
        if current_delay <= fastest_delay.saturating_add(tolerance) {
            return current.into();
        }
    }

    fastest.name.clone()
}

fn select_fallback(current: Option<&str>, candidates: &[NodeSelectionCandidateInput]) -> String {
    if let Some(current) = current
        && let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.name == current && candidate.alive.unwrap_or(true))
    {
        return candidate.name.clone();
    }

    candidates
        .iter()
        .find(|candidate| candidate.alive.unwrap_or(true))
        .unwrap_or(&candidates[0])
        .name
        .clone()
}

fn explain_candidates(
    strategy: SelectionStrategy,
    candidates: &[NodeSelectionCandidateInput],
    selected: &str,
) -> Vec<NodeSelectionCandidatePlan> {
    candidates
        .iter()
        .map(|candidate| {
            let alive = candidate.alive.unwrap_or(true);
            let eligible = strategy == SelectionStrategy::Manual || alive || candidate.name == selected;
            let reason = if candidate.name == selected {
                "selected"
            } else if !alive {
                "not alive for strategy selection"
            } else {
                "candidate"
            };

            NodeSelectionCandidatePlan {
                name: candidate.name.clone(),
                proxy_type: candidate.proxy_type.clone(),
                eligible,
                reason: reason.into(),
                alive: candidate.alive,
                delay_ms: candidate.delay_ms,
            }
        })
        .collect()
}

fn normalize_candidates(candidates: Vec<NodeSelectionCandidateInput>) -> Vec<NodeSelectionCandidateInput> {
    let mut normalized = Vec::new();

    for candidate in candidates {
        let name = normalize_name(&candidate.name);
        if name.is_empty()
            || normalized
                .iter()
                .any(|item: &NodeSelectionCandidateInput| item.name == name)
        {
            continue;
        }

        normalized.push(NodeSelectionCandidateInput {
            name,
            proxy_type: candidate.proxy_type.map(|proxy_type| normalize_name(&proxy_type)),
            alive: candidate.alive,
            delay_ms: candidate.delay_ms,
        });
    }

    normalized
}

fn normalize_name(name: &str) -> String {
    name.trim().into()
}

fn normalize_group_type(group_type: Option<&str>) -> String {
    group_type
        .map(|group_type| group_type.trim().replace('-', "").replace('_', "").to_ascii_lowercase())
        .filter(|group_type| !group_type.is_empty())
        .unwrap_or_else(|| "select".into())
        .into()
}

impl SelectionStrategy {
    fn from_group_type(group_type: &str) -> Self {
        match group_type {
            "select" | "selector" => Self::Manual,
            "urltest" => Self::UrlTest,
            "fallback" => Self::Fallback,
            "loadbalance" => Self::LoadBalance,
            "relay" => Self::Relay,
            _ => Self::Unknown,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::UrlTest => "url-test",
            Self::Fallback => "fallback",
            Self::LoadBalance => "load-balance",
            Self::Relay => "relay",
            Self::Unknown => "unknown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(name: &str, alive: bool, delay_ms: u32) -> NodeSelectionCandidateInput {
        NodeSelectionCandidateInput {
            name: name.into(),
            proxy_type: Some("ss".into()),
            alive: Some(alive),
            delay_ms: Some(delay_ms),
        }
    }

    fn request(group_type: &str) -> NodeSelectionPlanRequest {
        NodeSelectionPlanRequest {
            group_name: "GLOBAL".into(),
            group_type: Some(group_type.into()),
            current: None,
            requested: None,
            tolerance_ms: None,
            candidates: vec![candidate("a", true, 90), candidate("b", true, 40)],
        }
    }

    #[test]
    fn applies_requested_candidate_when_present() {
        let mut plan_request = request("select");
        plan_request.current = Some("a".into());
        plan_request.requested = Some("b".into());

        let plan = build_node_selection_plan(plan_request);

        assert_eq!(plan.status, NodeSelectionPlanStatus::Ready);
        assert_eq!(plan.selected.as_deref(), Some("b"));
        assert!(plan.should_apply_runtime);
        assert!(plan.should_sync_tray);
    }

    #[test]
    fn rejects_missing_requested_candidate() {
        let mut plan_request = request("select");
        plan_request.requested = Some("missing".into());

        let plan = build_node_selection_plan(plan_request);

        assert_eq!(plan.status, NodeSelectionPlanStatus::Rejected);
        assert_eq!(plan.selected, None);
        assert!(!plan.should_apply_runtime);
    }

    #[test]
    fn keeps_url_test_current_within_tolerance() {
        let mut plan_request = request("url-test");
        plan_request.current = Some("a".into());
        plan_request.tolerance_ms = Some(60);

        let plan = build_node_selection_plan(plan_request);

        assert_eq!(plan.status, NodeSelectionPlanStatus::Noop);
        assert_eq!(plan.selected.as_deref(), Some("a"));
        assert!(!plan.should_apply_runtime);
    }

    #[test]
    fn url_test_selects_lowest_alive_delay_without_tolerance() {
        let mut plan_request = request("url-test");
        plan_request.current = Some("a".into());

        let plan = build_node_selection_plan(plan_request);

        assert_eq!(plan.status, NodeSelectionPlanStatus::Ready);
        assert_eq!(plan.selected.as_deref(), Some("b"));
    }

    #[test]
    fn fallback_uses_first_alive_candidate() {
        let mut plan_request = request("fallback");
        plan_request.candidates = vec![candidate("a", false, 10), candidate("b", true, 80)];

        let plan = build_node_selection_plan(plan_request);

        assert_eq!(plan.selected.as_deref(), Some("b"));
        assert_eq!(plan.candidates[0].eligible, false);
    }

    #[test]
    fn rejects_empty_candidate_list() {
        let mut plan_request = request("select");
        plan_request.candidates.clear();

        let plan = build_node_selection_plan(plan_request);

        assert_eq!(plan.status, NodeSelectionPlanStatus::Rejected);
        assert!(plan.reason.contains("no selectable candidates"));
    }
}
