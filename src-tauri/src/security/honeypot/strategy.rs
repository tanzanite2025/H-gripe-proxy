use super::ConfigDecoy;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoyDeploymentPlan {
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoyAccessResult {
    pub path: String,
    pub accessed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoyBatchResult {
    pub total: usize,
    pub succeeded: usize,
    pub failed: Vec<String>,
    pub accessed: Vec<DecoyAccessResult>,
}

impl DecoyBatchResult {
    fn empty(total: usize) -> Self {
        Self {
            total,
            succeeded: 0,
            failed: Vec::new(),
            accessed: Vec::new(),
        }
    }
}

pub fn deploy_decoy_plan(plan: DecoyDeploymentPlan) -> Result<DecoyBatchResult, String> {
    let paths = normalize_plan_paths(plan.paths);
    let mut result = DecoyBatchResult::empty(paths.len());

    for path in paths {
        let decoy = ConfigDecoy::new(PathBuf::from(&path));
        match decoy.deploy() {
            Ok(()) => result.succeeded += 1,
            Err(error) => result.failed.push(format!("{}: {}", path, error)),
        }
    }

    Ok(result)
}

pub fn cleanup_decoy_plan(plan: DecoyDeploymentPlan) -> Result<DecoyBatchResult, String> {
    let paths = normalize_plan_paths(plan.paths);
    let mut result = DecoyBatchResult::empty(paths.len());

    for path in paths {
        let decoy = ConfigDecoy::new(PathBuf::from(&path));
        match decoy.cleanup() {
            Ok(()) => result.succeeded += 1,
            Err(error) => result.failed.push(format!("{}: {}", path, error)),
        }
    }

    Ok(result)
}

pub fn check_decoy_plan_access(plan: DecoyDeploymentPlan) -> Result<DecoyBatchResult, String> {
    let paths = normalize_plan_paths(plan.paths);
    let mut result = DecoyBatchResult::empty(paths.len());

    for path in paths {
        let decoy = ConfigDecoy::new(PathBuf::from(&path));
        let accessed = decoy.check_access();
        result.succeeded += 1;
        result.accessed.push(DecoyAccessResult { path, accessed });
    }

    Ok(result)
}

fn normalize_plan_paths(paths: Vec<String>) -> Vec<String> {
    let mut normalized = Vec::new();

    for path in paths {
        let path = path.trim();
        if !path.is_empty() && !normalized.iter().any(|existing| existing == path) {
            normalized.push(path.to_string());
        }
    }

    normalized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_empty_and_duplicate_paths() {
        let paths = normalize_plan_paths(vec![
            "config_decoy.yaml".to_string(),
            " ".to_string(),
            "config_decoy.yaml".to_string(),
            "profiles/config_decoy.yaml".to_string(),
        ]);

        assert_eq!(
            paths,
            vec![
                "config_decoy.yaml".to_string(),
                "profiles/config_decoy.yaml".to_string(),
            ]
        );
    }
}
