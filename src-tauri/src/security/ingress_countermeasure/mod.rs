pub mod classifier;
pub mod config;
pub mod deception;
pub mod persona;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, VecDeque};

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

const DEFAULT_MAX_TRACKED_SOURCES: usize = 256;

#[derive(Debug)]
pub struct IngressCountermeasureRuntime {
    config: RwLock<config::IngressCountermeasureConfig>,
    recent_signals: RwLock<RecentSignalStore>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EgressSupportPolicy {
    pub minimize_drift: bool,
    pub strongest_threat_level: classifier::ThreatLevel,
    pub rebind_grace_period_ms: u64,
    pub connection_warmup_ms: u64,
}

impl IngressCountermeasureRuntime {
    pub fn new(config: config::IngressCountermeasureConfig) -> Self {
        Self {
            config: RwLock::new(config),
            recent_signals: RwLock::new(RecentSignalStore::new(DEFAULT_MAX_TRACKED_SOURCES)),
        }
    }

    pub async fn record_signal(&self, source: impl AsRef<str>, reason: classifier::ThreatReason) {
        self.recent_signals.write().record(source.as_ref(), reason);
    }

    pub async fn snapshot_for_source(&self, source: impl AsRef<str>) -> classifier::IngressSignalSnapshot {
        self.recent_signals.read().snapshot_for(source.as_ref())
    }

    pub fn update_config(&self, config: config::IngressCountermeasureConfig) {
        *self.config.write() = config;
    }

    pub fn get_config(&self) -> config::IngressCountermeasureConfig {
        self.config.read().clone()
    }

    pub async fn classify_source(&self, source: impl AsRef<str>) -> classifier::ClassificationResult {
        let snapshot = self.snapshot_for_source(source).await;
        let thresholds = self.get_config().classifier_thresholds;
        let classifier = classifier::IngressThreatClassifier::new(thresholds);
        classifier.classify(snapshot)
    }

    pub fn route_for_level(&self, level: classifier::ThreatLevel) -> deception::ResponsePlan {
        deception::route_for_level(level, &self.get_config())
    }

    pub async fn plan_for_source(&self, source: impl AsRef<str>) -> deception::ResponsePlan {
        let classification = self.classify_source(source).await;
        self.route_for_level(classification.level)
    }

    pub fn current_egress_support_policy(&self) -> EgressSupportPolicy {
        let config = self.get_config();
        let support = config.egress_stability_support.clone();

        if !config.enabled || !support.enabled {
            return EgressSupportPolicy {
                minimize_drift: false,
                strongest_threat_level: classifier::ThreatLevel::Normal,
                rebind_grace_period_ms: support.rebind_grace_period_ms,
                connection_warmup_ms: support.connection_warmup_ms,
            };
        }

        let classifier = classifier::IngressThreatClassifier::new(config.classifier_thresholds);
        let strongest_threat_level = self
            .recent_signals
            .read()
            .strongest_threat_level(&classifier)
            .unwrap_or(classifier::ThreatLevel::Normal);

        EgressSupportPolicy {
            minimize_drift: strongest_threat_level != classifier::ThreatLevel::Normal,
            strongest_threat_level,
            rebind_grace_period_ms: support.rebind_grace_period_ms,
            connection_warmup_ms: support.connection_warmup_ms,
        }
    }
}

#[derive(Debug)]
struct RecentSignalStore {
    by_source: HashMap<String, classifier::IngressSignalSnapshot>,
    source_order: VecDeque<String>,
    max_sources: usize,
}

impl RecentSignalStore {
    fn new(max_sources: usize) -> Self {
        Self {
            by_source: HashMap::new(),
            source_order: VecDeque::new(),
            max_sources: max_sources.max(1),
        }
    }

    fn record(&mut self, source: &str, reason: classifier::ThreatReason) {
        let source_key = source.to_string();
        let is_new_source = !self.by_source.contains_key(source);
        let snapshot = self.by_source.entry(source_key.clone()).or_default();

        match reason {
            classifier::ThreatReason::AntiProbeFailure => snapshot.anti_probe_failed = true,
            classifier::ThreatReason::HoneypotTriggered => snapshot.honeypot_triggered = true,
            classifier::ThreatReason::SuspiciousHeaders => {
                snapshot.suspicious_header_count = snapshot.suspicious_header_count.saturating_add(1);
            }
            classifier::ThreatReason::RepeatedBurst => {
                snapshot.repeated_burst_count = snapshot.repeated_burst_count.saturating_add(1);
            }
        }

        self.touch_source(&source_key, is_new_source);
        self.evict_if_needed();
    }

    fn snapshot_for(&self, source: &str) -> classifier::IngressSignalSnapshot {
        self.by_source.get(source).copied().unwrap_or_default()
    }

    fn strongest_threat_level(
        &self,
        classifier: &classifier::IngressThreatClassifier,
    ) -> Option<classifier::ThreatLevel> {
        self.by_source
            .values()
            .map(|snapshot| classifier.classify(*snapshot).level)
            .max_by_key(|level| threat_level_rank(*level))
    }

    fn evict_if_needed(&mut self) {
        while self.by_source.len() > self.max_sources {
            if let Some(oldest_source) = self.source_order.pop_front() {
                self.by_source.remove(&oldest_source);
            } else {
                break;
            }
        }
    }

    fn touch_source(&mut self, source: &str, is_new_source: bool) {
        if !is_new_source {
            self.source_order.retain(|existing| existing != source);
        }

        self.source_order.push_back(source.to_string());
    }
}

fn threat_level_rank(level: classifier::ThreatLevel) -> u8 {
    match level {
        classifier::ThreatLevel::Normal => 0,
        classifier::ThreatLevel::Suspicious => 1,
        classifier::ThreatLevel::Hostile => 2,
    }
}

#[allow(unused_imports)]
pub use classifier::{ClassificationResult, IngressSignalSnapshot, IngressThreatClassifier, ThreatLevel, ThreatReason};
#[allow(unused_imports)]
pub use config::{
    ClassifierThresholds, DeceptionMode, EgressStabilitySupportConfig, FakeSurfacePolicy, IngressCountermeasureConfig,
    PersonaProfile, PersonaTone, ResponseDelayRanges, SurfaceBias,
};
#[allow(unused_imports)]
pub use deception::{DelayWindow, ResponseMode, ResponsePlan};
#[allow(unused_imports)]
pub use persona::{
    HeaderOrderProfile, RuntimePersonaProfile, SizeShapingLevel, TimingJitterProfile, default_persona_profiles,
    runtime_persona_profiles, select_persona,
};
