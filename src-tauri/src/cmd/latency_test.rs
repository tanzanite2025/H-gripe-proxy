use super::CmdResult;
use crate::core::latency_test::{LatencyTestPlan, LatencyTestPlanRequest, build_latency_test_plan};

#[tauri::command]
pub async fn plan_latency_test(request: LatencyTestPlanRequest) -> CmdResult<LatencyTestPlan> {
    Ok(build_latency_test_plan(request))
}
