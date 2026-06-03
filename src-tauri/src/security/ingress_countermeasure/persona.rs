use serde::{Deserialize, Serialize};

use super::classifier::ThreatLevel;
use super::config::{PersonaProfile, PersonaTone, SurfaceBias};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeaderOrderProfile {
    Chromium,
    Firefox,
    CurlLike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimingJitterProfile {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SizeShapingLevel {
    None,
    Moderate,
    Aggressive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePersonaProfile {
    pub name: String,
    pub tls_fingerprint: String,
    pub ua_family: String,
    pub header_order_profile: HeaderOrderProfile,
    pub timing_jitter_profile: TimingJitterProfile,
    pub size_shaping_level: SizeShapingLevel,
    pub eligible_levels: Vec<ThreatLevel>,
}

impl RuntimePersonaProfile {
    pub fn supports(&self, level: ThreatLevel) -> bool {
        self.eligible_levels.contains(&level)
    }
}

pub fn default_persona_profiles() -> Vec<RuntimePersonaProfile> {
    vec![
        RuntimePersonaProfile {
            name: "normal-browser".to_string(),
            tls_fingerprint: "chrome".to_string(),
            ua_family: "chrome-stable".to_string(),
            header_order_profile: HeaderOrderProfile::Chromium,
            timing_jitter_profile: TimingJitterProfile::Low,
            size_shaping_level: SizeShapingLevel::None,
            eligible_levels: vec![ThreatLevel::Normal],
        },
        RuntimePersonaProfile {
            name: "guarded-browser".to_string(),
            tls_fingerprint: "random".to_string(),
            ua_family: "firefox-esr".to_string(),
            header_order_profile: HeaderOrderProfile::Firefox,
            timing_jitter_profile: TimingJitterProfile::Medium,
            size_shaping_level: SizeShapingLevel::Moderate,
            eligible_levels: vec![ThreatLevel::Suspicious],
        },
        RuntimePersonaProfile {
            name: "burner-browser".to_string(),
            tls_fingerprint: "randomized".to_string(),
            ua_family: "safari-mobile".to_string(),
            header_order_profile: HeaderOrderProfile::CurlLike,
            timing_jitter_profile: TimingJitterProfile::High,
            size_shaping_level: SizeShapingLevel::Aggressive,
            eligible_levels: vec![ThreatLevel::Hostile],
        },
    ]
}

pub fn runtime_persona_profiles(configured: &[PersonaProfile]) -> Vec<RuntimePersonaProfile> {
    if configured.is_empty() {
        return default_persona_profiles();
    }

    let normal = default_persona_profiles()
        .into_iter()
        .find(|persona| persona.supports(ThreatLevel::Normal))
        .expect("default normal persona must exist");

    let suspicious_source = configured
        .iter()
        .find(|profile| !matches!(profile.surface_bias, SurfaceBias::Decoy))
        .or_else(|| configured.first());
    let hostile_source = configured
        .iter()
        .find(|profile| matches!(profile.surface_bias, SurfaceBias::Decoy))
        .or_else(|| configured.first());

    let mut personas = vec![normal];

    if let Some(profile) = suspicious_source {
        personas.push(configured_profile_to_runtime(profile, ThreatLevel::Suspicious));
    }

    if let Some(profile) = hostile_source {
        personas.push(configured_profile_to_runtime(profile, ThreatLevel::Hostile));
    }

    personas
}

pub fn select_persona(level: ThreatLevel, personas: &[RuntimePersonaProfile]) -> Option<&RuntimePersonaProfile> {
    personas
        .iter()
        .find(|persona| persona.supports(level))
        .or_else(|| personas.iter().find(|persona| persona.supports(ThreatLevel::Normal)))
}

fn configured_profile_to_runtime(profile: &PersonaProfile, level: ThreatLevel) -> RuntimePersonaProfile {
    RuntimePersonaProfile {
        name: profile.id.clone(),
        tls_fingerprint: tls_fingerprint_for(profile, level).to_string(),
        ua_family: ua_family_for(profile, level).to_string(),
        header_order_profile: header_order_for(profile, level),
        timing_jitter_profile: timing_jitter_for(profile, level),
        size_shaping_level: size_shaping_for(profile, level),
        eligible_levels: vec![level],
    }
}

fn tls_fingerprint_for(profile: &PersonaProfile, level: ThreatLevel) -> &'static str {
    match (level, &profile.surface_bias) {
        (ThreatLevel::Hostile, SurfaceBias::Decoy) => "randomized",
        (ThreatLevel::Hostile, _) => "random",
        (ThreatLevel::Suspicious, SurfaceBias::Production) => "chrome",
        (ThreatLevel::Suspicious, _) => "random",
        _ => "chrome",
    }
}

fn ua_family_for(profile: &PersonaProfile, level: ThreatLevel) -> &'static str {
    match (level, &profile.tone, &profile.surface_bias) {
        (ThreatLevel::Hostile, _, SurfaceBias::Decoy) => "safari-mobile",
        (_, PersonaTone::Helpful, _) => "firefox-esr",
        (_, _, SurfaceBias::Production) => "chrome-stable",
        _ => "firefox-esr",
    }
}

fn header_order_for(profile: &PersonaProfile, level: ThreatLevel) -> HeaderOrderProfile {
    match (level, &profile.surface_bias) {
        (ThreatLevel::Hostile, SurfaceBias::Decoy) => HeaderOrderProfile::CurlLike,
        (_, SurfaceBias::Production) => HeaderOrderProfile::Chromium,
        _ => HeaderOrderProfile::Firefox,
    }
}

fn timing_jitter_for(profile: &PersonaProfile, level: ThreatLevel) -> TimingJitterProfile {
    match (level, &profile.tone) {
        (ThreatLevel::Hostile, _) => TimingJitterProfile::High,
        (_, PersonaTone::Restrained) => TimingJitterProfile::Medium,
        (_, PersonaTone::Helpful) => TimingJitterProfile::Medium,
        _ => TimingJitterProfile::Low,
    }
}

fn size_shaping_for(profile: &PersonaProfile, level: ThreatLevel) -> SizeShapingLevel {
    match (level, &profile.surface_bias) {
        (ThreatLevel::Hostile, SurfaceBias::Decoy) => SizeShapingLevel::Aggressive,
        (_, SurfaceBias::Production) => SizeShapingLevel::None,
        _ => SizeShapingLevel::Moderate,
    }
}
