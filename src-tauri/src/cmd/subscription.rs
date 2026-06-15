use super::{CmdResult, StringifyErr as _};
use crate::subscription::{
    model::{SubscriptionSourceState, SubscriptionStateDocument},
    persist::{read_subscription_source_state, read_subscription_state_document},
};

#[tauri::command]
pub async fn get_subscription_state() -> CmdResult<SubscriptionStateDocument> {
    read_subscription_state_document().await.stringify_err()
}

#[tauri::command]
pub async fn get_subscription_source_state(
    source_id: String,
) -> CmdResult<Option<SubscriptionSourceState>> {
    read_subscription_source_state(source_id.as_str()).await.stringify_err()
}
