use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngressCountermeasureConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub classifier_thresholds: ClassifierThresholds,
    #[serde(default = "default_persona_profiles")]
    pub persona_profiles: Vec<PersonaProfile>,
    #[serde(default)]
    pub deception_mode: DeceptionMode,
    #[serde(default)]
    pub response_delay_ranges: ResponseDelayRanges,
    #[serde(default = "default_fake_surface_policies")]
    pub fake_surface_policies: Vec<FakeSurfacePolicy>,
    #[serde(default)]
    pub egress_stability_support: EgressStabilitySupportConfig,
}

impl IngressCountermeasureConfig {
    pub fn recommended() -> Self {
        Self::default()
    }
}

impl Default for IngressCountermeasureConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            classifier_thresholds: ClassifierThresholds::default(),
            persona_profiles: default_persona_profiles(),
            deception_mode: DeceptionMode::default(),
            response_delay_ranges: ResponseDelayRanges::default(),
            fake_surface_policies: default_fake_surface_policies(),
            egress_stability_support: EgressStabilitySupportConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassifierThresholds {
    #[serde(default = "default_low_confidence_threshold")]
    pub low_confidence: f32,
    #[serde(default = "default_medium_confidence_threshold")]
    pub medium_confidence: f32,
    #[serde(default = "default_high_confidence_threshold")]
    pub high_confidence: f32,
}

impl Default for ClassifierThresholds {
    fn default() -> Self {
        Self {
            low_confidence: default_low_confidence_threshold(),
            medium_confidence: default_medium_confidence_threshold(),
            high_confidence: default_high_confidence_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonaProfile {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub tone: PersonaTone,
    #[serde(default)]
    pub surface_bias: SurfaceBias,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PersonaTone {
    Restrained,
    Neutral,
    Helpful,
}

impl Default for PersonaTone {
    fn default() -> Self {
        Self::Neutral
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SurfaceBias {
    Decoy,
    Balanced,
    Production,
}

impl Default for SurfaceBias {
    fn default() -> Self {
        Self::Balanced
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DeceptionMode {
    Disabled,
    ObserveOnly,
    DecoyPreferred,
    DecoyOnly,
}

impl Default for DeceptionMode {
    fn default() -> Self {
        Self::DecoyPreferred
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseDelayRanges {
    #[serde(default = "default_soft_delay_min_ms")]
    pub soft_delay_min_ms: u64,
    #[serde(default = "default_soft_delay_max_ms")]
    pub soft_delay_max_ms: u64,
    #[serde(default = "default_hard_delay_min_ms")]
    pub hard_delay_min_ms: u64,
    #[serde(default = "default_hard_delay_max_ms")]
    pub hard_delay_max_ms: u64,
}

impl Default for ResponseDelayRanges {
    fn default() -> Self {
        Self {
            soft_delay_min_ms: default_soft_delay_min_ms(),
            soft_delay_max_ms: default_soft_delay_max_ms(),
            hard_delay_min_ms: default_hard_delay_min_ms(),
            hard_delay_max_ms: default_hard_delay_max_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FakeSurfacePolicy {
    pub surface: String,
    #[serde(default)]
    pub priority: u8,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EgressStabilitySupportConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_rebind_grace_period_ms")]
    pub rebind_grace_period_ms: u64,
    #[serde(default = "default_connection_warmup_ms")]
    pub connection_warmup_ms: u64,
}

impl Default for EgressStabilitySupportConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            rebind_grace_period_ms: default_rebind_grace_period_ms(),
            connection_warmup_ms: default_connection_warmup_ms(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_low_confidence_threshold() -> f32 {
    0.35
}

fn default_medium_confidence_threshold() -> f32 {
    0.6
}

fn default_high_confidence_threshold() -> f32 {
    0.82
}

fn default_soft_delay_min_ms() -> u64 {
    120
}

fn default_soft_delay_max_ms() -> u64 {
    400
}

fn default_hard_delay_min_ms() -> u64 {
    900
}

fn default_hard_delay_max_ms() -> u64 {
    1_800
}

fn default_rebind_grace_period_ms() -> u64 {
    1_500
}

fn default_connection_warmup_ms() -> u64 {
    750
}

fn default_persona_profiles() -> Vec<PersonaProfile> {
    vec![
        PersonaProfile {
            id: "operator-decoy".to_string(),
            label: "Operator Decoy".to_string(),
            tone: PersonaTone::Restrained,
            surface_bias: SurfaceBias::Decoy,
        },
        PersonaProfile {
            id: "support-decoy".to_string(),
            label: "Support Decoy".to_string(),
            tone: PersonaTone::Helpful,
            surface_bias: SurfaceBias::Balanced,
        },
    ]
}

fn default_fake_surface_policies() -> Vec<FakeSurfacePolicy> {
    vec![
        FakeSurfacePolicy {
            surface: "login".to_string(),
            priority: 100,
            enabled: true,
        },
        FakeSurfacePolicy {
            surface: "dashboard".to_string(),
            priority: 80,
            enabled: true,
        },
    ]
}
