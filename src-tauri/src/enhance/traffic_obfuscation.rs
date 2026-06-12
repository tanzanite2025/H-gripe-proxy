use serde_yaml_ng::{Mapping, Value};

use crate::config::advanced::AdvancedConfig;
use crate::traffic::{
    TrafficObfuscationConfig,
    direction::{DirectionMode, DirectionObfuscationConfig},
    padding::{
        FrequencyType, PaddingFrequency, PaddingIntensity, PaddingTiming, PerformanceControl, TrafficPaddingConfig,
    },
    timing_jitter::{JitterMode, TimingJitterConfig},
};

fn load_advanced_config() -> AdvancedConfig {
    crate::core::coordinator::get_coordinator().get_advanced_config()
}

pub fn apply_traffic_obfuscation_config(config: Mapping) -> Mapping {
    let advanced = load_advanced_config();
    apply_traffic_obfuscation_config_with_advanced(config, &advanced)
}

pub fn apply_traffic_obfuscation_config_with_advanced(mut config: Mapping, advanced: &AdvancedConfig) -> Mapping {
    let obfuscation = effective_obfuscation_config(advanced);
    if !obfuscation.enabled {
        let key = Value::from("obfuscation");
        config.remove(&key);
        return config;
    }

    config.insert(
        "obfuscation".into(),
        Value::Mapping(to_obfuscation_mapping(&obfuscation)),
    );
    config
}

fn effective_obfuscation_config(advanced: &AdvancedConfig) -> TrafficObfuscationConfig {
    if advanced.traffic_obfuscation.enabled {
        advanced.traffic_obfuscation.clone()
    } else if advanced.traffic_padding.enabled {
        TrafficObfuscationConfig::from_legacy_padding(&advanced.traffic_padding)
    } else {
        advanced.traffic_obfuscation.clone()
    }
}

fn to_obfuscation_mapping(config: &TrafficObfuscationConfig) -> Mapping {
    let (padding, timing, direction) = config.effective_configs();
    let mut mapping = Mapping::new();
    mapping.insert("enabled".into(), Value::Bool(true));
    mapping.insert("profile".into(), Value::from(profile_name(config)));
    mapping.insert("padding".into(), Value::Mapping(to_padding_mapping(&padding)));
    mapping.insert("timing".into(), Value::Mapping(to_timing_mapping(&timing)));
    mapping.insert("direction".into(), Value::Mapping(to_direction_mapping(&direction)));
    mapping
}

fn profile_name(config: &TrafficObfuscationConfig) -> &'static str {
    use crate::traffic::ObfuscationProfile;

    match config.profile {
        ObfuscationProfile::None => "none",
        ObfuscationProfile::Conservative => "conservative",
        ObfuscationProfile::Aggressive => "aggressive",
        ObfuscationProfile::Custom => "custom",
    }
}

fn to_padding_mapping(config: &TrafficPaddingConfig) -> Mapping {
    let mut mapping = Mapping::new();
    mapping.insert("enabled".into(), Value::Bool(config.enabled));
    mapping.insert("min-size".into(), Value::from(config.min_size as u64));
    mapping.insert("max-size".into(), Value::from(config.max_size as u64));
    mapping.insert("encrypt".into(), Value::Bool(config.encrypt));
    mapping.insert(
        "intensity".into(),
        Value::from(padding_intensity_name(&config.intensity)),
    );
    mapping.insert(
        "intensity-multiplier".into(),
        Value::from(config.intensity.as_multiplier() as f64),
    );
    mapping.insert(
        "frequency".into(),
        Value::Mapping(to_frequency_mapping(&config.frequency)),
    );
    mapping.insert("timing".into(), Value::from(padding_timing_name(&config.timing)));
    mapping.insert("smart-padding".into(), Value::Bool(config.smart_padding));
    mapping.insert(
        "performance-control".into(),
        Value::Mapping(to_performance_control_mapping(&config.performance_control)),
    );
    mapping
}

fn padding_intensity_name(intensity: &PaddingIntensity) -> &'static str {
    match intensity {
        PaddingIntensity::Low => "low",
        PaddingIntensity::Medium => "medium",
        PaddingIntensity::High => "high",
        PaddingIntensity::Custom(_) => "custom",
    }
}

fn to_frequency_mapping(frequency: &PaddingFrequency) -> Mapping {
    let mut mapping = Mapping::new();
    mapping.insert("type".into(), Value::from(frequency_type_name(&frequency.freq_type)));
    mapping.insert("interval".into(), Value::from(frequency.interval));
    mapping
}

fn frequency_type_name(freq_type: &FrequencyType) -> &'static str {
    match freq_type {
        FrequencyType::Time => "time",
        FrequencyType::Request => "request",
        FrequencyType::Random => "random",
    }
}

fn padding_timing_name(timing: &PaddingTiming) -> &'static str {
    match timing {
        PaddingTiming::Before => "before",
        PaddingTiming::After => "after",
        PaddingTiming::Random => "random",
    }
}

fn to_performance_control_mapping(config: &PerformanceControl) -> Mapping {
    let mut mapping = Mapping::new();
    mapping.insert("max-bandwidth".into(), Value::from(config.max_bandwidth as u64));
    mapping.insert("max-cpu-usage".into(), Value::from(config.max_cpu_usage as f64));
    mapping.insert("max-memory".into(), Value::from(config.max_memory as u64));
    mapping.insert("auto-downgrade".into(), Value::Bool(config.auto_downgrade));
    mapping
}

fn to_timing_mapping(config: &TimingJitterConfig) -> Mapping {
    let mut mapping = Mapping::new();
    mapping.insert("enabled".into(), Value::Bool(config.enabled));
    mapping.insert("mode".into(), Value::from(jitter_mode_name(&config.mode)));
    mapping.insert("min-delay-ms".into(), Value::from(config.min_delay_ms));
    mapping.insert("max-delay-ms".into(), Value::from(config.max_delay_ms));
    mapping.insert("batch-window-ms".into(), Value::from(config.batch_window_ms));
    mapping
}

fn jitter_mode_name(mode: &JitterMode) -> &'static str {
    match mode {
        JitterMode::Uniform => "uniform",
        JitterMode::Gaussian => "gaussian",
        JitterMode::Pareto => "pareto",
    }
}

fn to_direction_mapping(config: &DirectionObfuscationConfig) -> Mapping {
    let mut mapping = Mapping::new();
    mapping.insert("enabled".into(), Value::Bool(config.enabled));
    mapping.insert("mode".into(), Value::from(direction_mode_name(&config.mode)));
    mapping.insert("mirror-ratio".into(), Value::from(config.mirror_ratio as f64));
    mapping.insert("pad-to-size".into(), Value::from(config.pad_to_size as u64));
    mapping
}

fn direction_mode_name(mode: &DirectionMode) -> &'static str {
    match mode {
        DirectionMode::Mirror => "mirror",
        DirectionMode::Pad => "pad",
        DirectionMode::Random => "random",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traffic::{ObfuscationProfile, TrafficPaddingConfig as LegacyTrafficPaddingConfig};

    #[test]
    fn injects_effective_traffic_obfuscation_config() {
        let advanced = AdvancedConfig {
            traffic_obfuscation: TrafficObfuscationConfig {
                enabled: true,
                profile: ObfuscationProfile::Conservative,
                ..TrafficObfuscationConfig::default()
            },
            ..AdvancedConfig::default()
        };

        let config = apply_traffic_obfuscation_config_with_advanced(Mapping::new(), &advanced);
        let obfuscation = config.get("obfuscation").and_then(Value::as_mapping).unwrap();
        let padding = obfuscation.get("padding").and_then(Value::as_mapping).unwrap();
        let timing = obfuscation.get("timing").and_then(Value::as_mapping).unwrap();
        let direction = obfuscation.get("direction").and_then(Value::as_mapping).unwrap();

        assert_eq!(obfuscation.get("enabled").and_then(Value::as_bool), Some(true));
        assert_eq!(obfuscation.get("profile").and_then(Value::as_str), Some("conservative"));
        assert_eq!(padding.get("enabled").and_then(Value::as_bool), Some(true));
        assert_eq!(timing.get("enabled").and_then(Value::as_bool), Some(true));
        assert_eq!(direction.get("enabled").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn migrates_legacy_padding_config_into_obfuscation_mapping() {
        let advanced = AdvancedConfig {
            traffic_padding: LegacyTrafficPaddingConfig {
                enabled: true,
                min_size: 128,
                max_size: 512,
                ..LegacyTrafficPaddingConfig::default()
            },
            ..AdvancedConfig::default()
        };

        let config = apply_traffic_obfuscation_config_with_advanced(Mapping::new(), &advanced);
        let obfuscation = config.get("obfuscation").and_then(Value::as_mapping).unwrap();
        let padding = obfuscation.get("padding").and_then(Value::as_mapping).unwrap();

        assert_eq!(obfuscation.get("profile").and_then(Value::as_str), Some("custom"));
        assert_eq!(padding.get("enabled").and_then(Value::as_bool), Some(true));
        assert_eq!(padding.get("min-size").and_then(Value::as_i64), Some(128));
        assert_eq!(padding.get("max-size").and_then(Value::as_i64), Some(512));
    }

    #[test]
    fn removes_stale_obfuscation_mapping_when_disabled() {
        let mut config = Mapping::new();
        config.insert("obfuscation".into(), Value::Mapping(Mapping::new()));

        let config = apply_traffic_obfuscation_config_with_advanced(config, &AdvancedConfig::default());

        assert!(config.get("obfuscation").is_none());
    }
}
