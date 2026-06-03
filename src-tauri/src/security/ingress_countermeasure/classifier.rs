use serde::{Deserialize, Serialize};

use super::config::ClassifierThresholds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThreatLevel {
    Normal,
    Suspicious,
    Hostile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ThreatReason {
    AntiProbeFailure,
    HoneypotTriggered,
    SuspiciousHeaders,
    RepeatedBurst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct IngressSignalSnapshot {
    #[serde(default)]
    pub anti_probe_failed: bool,
    #[serde(default)]
    pub honeypot_triggered: bool,
    #[serde(default)]
    pub suspicious_header_count: u32,
    #[serde(default)]
    pub repeated_burst_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClassificationResult {
    pub level: ThreatLevel,
    pub reasons: Vec<ThreatReason>,
}

#[derive(Debug, Clone)]
pub struct IngressThreatClassifier {
    thresholds: ClassifierThresholds,
}

impl IngressThreatClassifier {
    pub fn new(thresholds: ClassifierThresholds) -> Self {
        Self {
            thresholds: normalize_thresholds(thresholds),
        }
    }

    pub fn classify(&self, snapshot: IngressSignalSnapshot) -> ClassificationResult {
        let mut reasons = Vec::new();

        if snapshot.honeypot_triggered {
            reasons.push(ThreatReason::HoneypotTriggered);
        }

        if snapshot.anti_probe_failed {
            reasons.push(ThreatReason::AntiProbeFailure);
        }

        if snapshot.suspicious_header_count > 0 {
            reasons.push(ThreatReason::SuspiciousHeaders);
        }

        if snapshot.repeated_burst_count > 0 {
            reasons.push(ThreatReason::RepeatedBurst);
        }

        if snapshot.honeypot_triggered {
            return ClassificationResult {
                level: ThreatLevel::Hostile,
                reasons,
            };
        }

        let mut score = 0.0_f32;

        if snapshot.anti_probe_failed {
            score += self.thresholds.low_confidence;
        }

        score += self.thresholds.low_confidence * snapshot.suspicious_header_count as f32;
        score += self.thresholds.low_confidence * snapshot.repeated_burst_count as f32;

        let level = if snapshot.anti_probe_failed && snapshot.repeated_burst_count > 0 {
            if score >= self.thresholds.high_confidence {
                ThreatLevel::Hostile
            } else {
                ThreatLevel::Suspicious
            }
        } else if score >= self.thresholds.high_confidence {
            ThreatLevel::Hostile
        } else if score >= self.thresholds.medium_confidence {
            ThreatLevel::Suspicious
        } else {
            ThreatLevel::Normal
        };

        ClassificationResult { level, reasons }
    }
}

fn normalize_thresholds(thresholds: ClassifierThresholds) -> ClassifierThresholds {
    let low_confidence = thresholds.low_confidence.max(0.0);
    let medium_confidence = thresholds.medium_confidence.max(low_confidence);
    let high_confidence = thresholds.high_confidence.max(medium_confidence);

    ClassifierThresholds {
        low_confidence,
        medium_confidence,
        high_confidence,
    }
}
