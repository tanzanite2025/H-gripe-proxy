use serde::{Deserialize, Serialize};

use super::classifier::ThreatLevel;
use super::config::{DeceptionMode, IngressCountermeasureConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResponseMode {
    Real,
    Mimic,
    Deception,
    LimitedReject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelayWindow {
    pub min_ms: u64,
    pub max_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsePlan {
    pub level: ThreatLevel,
    pub mode: ResponseMode,
    pub fake_surfaces: Vec<String>,
    pub delay_window: Option<DelayWindow>,
}

pub fn route_for_level(level: ThreatLevel, config: &IngressCountermeasureConfig) -> ResponsePlan {
    let enabled_surfaces = enabled_fake_surfaces(config);

    if !config.enabled {
        return match level {
            ThreatLevel::Hostile => ResponsePlan {
                level,
                mode: ResponseMode::LimitedReject,
                fake_surfaces: Vec::new(),
                delay_window: Some(hard_delay_window(config)),
            },
            _ => ResponsePlan {
                level,
                mode: ResponseMode::Real,
                fake_surfaces: Vec::new(),
                delay_window: None,
            },
        };
    }

    match level {
        ThreatLevel::Normal => ResponsePlan {
            level,
            mode: ResponseMode::Real,
            fake_surfaces: Vec::new(),
            delay_window: None,
        },
        ThreatLevel::Suspicious => ResponsePlan {
            level,
            mode: ResponseMode::Mimic,
            fake_surfaces: Vec::new(),
            delay_window: Some(soft_delay_window(config)),
        },
        ThreatLevel::Hostile => {
            let deception_available = !enabled_surfaces.is_empty()
                && matches!(
                    config.deception_mode,
                    DeceptionMode::DecoyPreferred | DeceptionMode::DecoyOnly
                );

            if deception_available {
                ResponsePlan {
                    level,
                    mode: ResponseMode::Deception,
                    fake_surfaces: enabled_surfaces,
                    delay_window: Some(hard_delay_window(config)),
                }
            } else {
                ResponsePlan {
                    level,
                    mode: ResponseMode::LimitedReject,
                    fake_surfaces: Vec::new(),
                    delay_window: Some(hard_delay_window(config)),
                }
            }
        }
    }
}

fn enabled_fake_surfaces(config: &IngressCountermeasureConfig) -> Vec<String> {
    let mut policies = config
        .fake_surface_policies
        .iter()
        .filter(|policy| policy.enabled)
        .collect::<Vec<_>>();

    policies.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.surface.cmp(&right.surface))
    });

    policies.into_iter().map(|policy| policy.surface.clone()).collect()
}

fn soft_delay_window(config: &IngressCountermeasureConfig) -> DelayWindow {
    DelayWindow {
        min_ms: config.response_delay_ranges.soft_delay_min_ms,
        max_ms: config.response_delay_ranges.soft_delay_max_ms,
    }
}

fn hard_delay_window(config: &IngressCountermeasureConfig) -> DelayWindow {
    DelayWindow {
        min_ms: config.response_delay_ranges.hard_delay_min_ms,
        max_ms: config.response_delay_ranges.hard_delay_max_ms,
    }
}
