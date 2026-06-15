use crate::{config::PrfOption, subscription::orchestration::update_subscription_profile};
use anyhow::Result;
use smartstring::alias::String;

pub async fn update_profile(
    uid: &String,
    option: Option<&PrfOption>,
    auto_refresh: bool,
    ignore_auto_update: bool,
    is_manual_trigger: bool,
) -> Result<()> {
    update_subscription_profile(uid, option, auto_refresh, ignore_auto_update, is_manual_trigger).await
}
