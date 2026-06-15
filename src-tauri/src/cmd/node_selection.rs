use super::CmdResult;
use crate::core::node_selection::{NodeSelectionPlan, NodeSelectionPlanRequest, build_node_selection_plan};

#[tauri::command]
pub async fn plan_node_selection(request: NodeSelectionPlanRequest) -> CmdResult<NodeSelectionPlan> {
    Ok(build_node_selection_plan(request))
}
