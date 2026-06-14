use serde::{Deserialize, Serialize};
use smartstring::alias::String;

pub const DEFAULT_LATENCY_TEST_URL: &str = "https://cp.cloudflare.com/generate_204";
const DEFAULT_LATENCY_TIMEOUT_MS: u32 = 10_000;
const GOOD_NETWORK_TIMEOUT_MS: u32 = 5_000;
const POOR_NETWORK_TIMEOUT_MS: u32 = 10_000;
const GOOD_NETWORK_CONCURRENCY: usize = 10;
const POOR_NETWORK_CONCURRENCY: usize = 3;
const MAX_LATENCY_CONCURRENCY: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LatencyNetworkQuality {
    Good,
    Poor,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum LatencyTestPlanStatus {
    Ready,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyTestPlanRequest {
    #[serde(default)]
    pub proxy_names: Vec<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u32>,
    #[serde(default)]
    pub concurrency: Option<usize>,
    #[serde(default)]
    pub network_quality: Option<LatencyNetworkQuality>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LatencyTestPlan {
    pub status: LatencyTestPlanStatus,
    pub reason: String,
    pub group: Option<String>,
    pub normalized_url: String,
    pub timeout_ms: u32,
    pub requested_count: usize,
    pub scheduled_count: usize,
    pub concurrency: usize,
    pub estimated_max_duration_ms: Option<u32>,
    pub proxy_names: Vec<String>,
}

pub fn normalize_latency_test_url(url: &str) -> String {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return DEFAULT_LATENCY_TEST_URL.into();
    }

    if trimmed.starts_with("http://") && trimmed.contains("/generate_204") {
        return format!("https://{}", &trimmed["http://".len()..]).into();
    }

    trimmed.into()
}

pub fn build_latency_test_plan(request: LatencyTestPlanRequest) -> LatencyTestPlan {
    let proxy_names = request
        .proxy_names
        .into_iter()
        .map(|name| name.trim().into())
        .filter(|name: &String| !name.is_empty())
        .collect::<Vec<_>>();
    let requested_count = proxy_names.len();
    let normalized_url = normalize_latency_test_url(request.url.as_deref().unwrap_or_default());

    if request.network_quality == Some(LatencyNetworkQuality::Offline) {
        return LatencyTestPlan {
            status: LatencyTestPlanStatus::Skipped,
            reason: "network is offline".into(),
            group: request.group,
            normalized_url,
            timeout_ms: 0,
            requested_count,
            scheduled_count: 0,
            concurrency: 0,
            estimated_max_duration_ms: None,
            proxy_names,
        };
    }

    if proxy_names.is_empty() {
        return LatencyTestPlan {
            status: LatencyTestPlanStatus::Skipped,
            reason: "no proxy candidates".into(),
            group: request.group,
            normalized_url,
            timeout_ms: resolved_timeout_ms(request.timeout_ms, request.network_quality),
            requested_count,
            scheduled_count: 0,
            concurrency: 0,
            estimated_max_duration_ms: None,
            proxy_names,
        };
    }

    let timeout_ms = resolved_timeout_ms(request.timeout_ms, request.network_quality);
    let concurrency = resolved_concurrency(request.concurrency, request.network_quality, requested_count);
    let batches = requested_count.div_ceil(concurrency);

    LatencyTestPlan {
        status: LatencyTestPlanStatus::Ready,
        reason: format!("scheduled {requested_count} proxy latency test(s) across {batches} batch(es)").into(),
        group: request.group,
        normalized_url,
        timeout_ms,
        requested_count,
        scheduled_count: requested_count,
        concurrency,
        estimated_max_duration_ms: Some(timeout_ms.saturating_mul(batches as u32)),
        proxy_names,
    }
}

fn resolved_timeout_ms(timeout_ms: Option<u32>, quality: Option<LatencyNetworkQuality>) -> u32 {
    if let Some(timeout_ms) = timeout_ms.filter(|timeout| *timeout > 0) {
        return timeout_ms;
    }

    match quality {
        Some(LatencyNetworkQuality::Good) => GOOD_NETWORK_TIMEOUT_MS,
        Some(LatencyNetworkQuality::Poor) => POOR_NETWORK_TIMEOUT_MS,
        Some(LatencyNetworkQuality::Offline) => 0,
        None => DEFAULT_LATENCY_TIMEOUT_MS,
    }
}

fn resolved_concurrency(
    concurrency: Option<usize>,
    quality: Option<LatencyNetworkQuality>,
    requested_count: usize,
) -> usize {
    let configured = concurrency.filter(|value| *value > 0).unwrap_or(match quality {
        Some(LatencyNetworkQuality::Good) => GOOD_NETWORK_CONCURRENCY,
        Some(LatencyNetworkQuality::Poor) => POOR_NETWORK_CONCURRENCY,
        Some(LatencyNetworkQuality::Offline) => 0,
        None => GOOD_NETWORK_CONCURRENCY,
    });

    configured.min(MAX_LATENCY_CONCURRENCY).min(requested_count).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(proxy_names: Vec<&str>) -> LatencyTestPlanRequest {
        LatencyTestPlanRequest {
            proxy_names: proxy_names.into_iter().map(Into::into).collect(),
            group: Some("GLOBAL".into()),
            url: None,
            timeout_ms: None,
            concurrency: None,
            network_quality: None,
        }
    }

    #[test]
    fn normalizes_empty_and_generate_204_urls() {
        assert_eq!(normalize_latency_test_url(""), DEFAULT_LATENCY_TEST_URL);
        assert_eq!(
            normalize_latency_test_url("http://cp.cloudflare.com/generate_204"),
            "https://cp.cloudflare.com/generate_204"
        );
        assert_eq!(
            normalize_latency_test_url("https://example.com/ping"),
            "https://example.com/ping"
        );
    }

    #[test]
    fn plans_batch_concurrency_with_defaults_and_cap() {
        let mut plan_request = request(vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k"]);
        plan_request.concurrency = Some(50);
        plan_request.timeout_ms = Some(7_000);

        let plan = build_latency_test_plan(plan_request);

        assert_eq!(plan.status, LatencyTestPlanStatus::Ready);
        assert_eq!(plan.timeout_ms, 7_000);
        assert_eq!(plan.concurrency, 10);
        assert_eq!(plan.requested_count, 11);
        assert_eq!(plan.estimated_max_duration_ms, Some(14_000));
    }

    #[test]
    fn skips_offline_network() {
        let mut plan_request = request(vec!["a", "b"]);
        plan_request.network_quality = Some(LatencyNetworkQuality::Offline);

        let plan = build_latency_test_plan(plan_request);

        assert_eq!(plan.status, LatencyTestPlanStatus::Skipped);
        assert_eq!(plan.concurrency, 0);
        assert_eq!(plan.scheduled_count, 0);
        assert_eq!(plan.timeout_ms, 0);
    }

    #[test]
    fn skips_empty_candidate_list() {
        let plan = build_latency_test_plan(request(vec!["", "  "]));

        assert_eq!(plan.status, LatencyTestPlanStatus::Skipped);
        assert_eq!(plan.reason, "no proxy candidates");
        assert_eq!(plan.requested_count, 0);
    }

    #[test]
    fn poor_network_uses_lower_default_concurrency() {
        let mut plan_request = request(vec!["a", "b", "c", "d"]);
        plan_request.network_quality = Some(LatencyNetworkQuality::Poor);

        let plan = build_latency_test_plan(plan_request);

        assert_eq!(plan.timeout_ms, POOR_NETWORK_TIMEOUT_MS);
        assert_eq!(plan.concurrency, POOR_NETWORK_CONCURRENCY);
        assert_eq!(plan.estimated_max_duration_ms, Some(20_000));
    }
}
