use crate::{
    config::{Config, PrfItem},
    subscription::transport::{TransportKind, transport_kind_from_option},
};
use anyhow::Result;
use serde::Serialize;
use smartstring::alias::String;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SubscriptionSource {
    pub source_id: String,
    pub name: Option<String>,
    pub url: Option<String>,
    pub home: Option<String>,
    pub description: Option<String>,
    pub updated: Option<usize>,
    pub update_interval: Option<u64>,
    pub allow_auto_update: bool,
    pub preferred_transport: TransportKind,
    pub timeout_seconds: Option<u64>,
    pub danger_accept_invalid_certs: bool,
    pub is_current: bool,
}

pub async fn list_subscription_sources() -> Result<Vec<SubscriptionSource>> {
    let profiles = Config::profiles().await;
    let profiles = profiles.latest_arc();
    let current_source_id = profiles.get_current().map(String::as_str);
    let sources = profiles
        .get_items()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| subscription_source_from_profile_item(item, current_source_id))
                .collect()
        })
        .unwrap_or_default();

    Ok(sources)
}

pub async fn get_subscription_source(source_id: &str) -> Result<Option<SubscriptionSource>> {
    Ok(list_subscription_sources()
        .await?
        .into_iter()
        .find(|source| source.source_id.as_str() == source_id))
}

pub fn subscription_source_from_profile_item(
    item: &PrfItem,
    current_source_id: Option<&str>,
) -> Option<SubscriptionSource> {
    if !item.itype.as_deref().is_some_and(|item_type| item_type == "remote") {
        return None;
    }

    let source_id = item.uid.clone()?;
    let option = item.option.as_ref();
    let is_current = current_source_id == Some(source_id.as_str());

    Some(SubscriptionSource {
        source_id,
        name: item.name.clone(),
        url: item.url.clone(),
        home: item.home.clone(),
        description: item.desc.clone(),
        updated: item.updated,
        update_interval: option.and_then(|option| option.update_interval),
        allow_auto_update: option
            .and_then(|option| option.allow_auto_update)
            .unwrap_or(true),
        preferred_transport: transport_kind_from_option(option),
        timeout_seconds: option.and_then(|option| option.timeout_seconds),
        danger_accept_invalid_certs: option
            .and_then(|option| option.danger_accept_invalid_certs)
            .unwrap_or(false),
        is_current,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PrfOption;

    #[test]
    fn maps_remote_profile_to_subscription_source() {
        let item = PrfItem {
            uid: Some("source-a".into()),
            itype: Some("remote".into()),
            name: Some("Source A".into()),
            url: Some("https://example.com/sub.yaml".into()),
            desc: Some("primary subscription".into()),
            updated: Some(123),
            option: Some(PrfOption {
                self_proxy: Some(true),
                update_interval: Some(24),
                allow_auto_update: Some(false),
                timeout_seconds: Some(30),
                danger_accept_invalid_certs: Some(true),
                ..PrfOption::default()
            }),
            ..PrfItem::default()
        };

        let source =
            subscription_source_from_profile_item(&item, Some("source-a")).expect("source");

        assert_eq!(source.source_id, "source-a");
        assert_eq!(source.name.as_deref(), Some("Source A"));
        assert_eq!(source.preferred_transport, TransportKind::LocalProxy);
        assert_eq!(source.update_interval, Some(24));
        assert!(!source.allow_auto_update);
        assert_eq!(source.timeout_seconds, Some(30));
        assert!(source.danger_accept_invalid_certs);
        assert!(source.is_current);
    }

    #[test]
    fn skips_non_remote_or_unidentified_profiles() {
        let local = PrfItem {
            uid: Some("local-a".into()),
            itype: Some("local".into()),
            ..PrfItem::default()
        };
        let missing_uid = PrfItem {
            itype: Some("remote".into()),
            ..PrfItem::default()
        };

        assert!(subscription_source_from_profile_item(&local, None).is_none());
        assert!(subscription_source_from_profile_item(&missing_uid, None).is_none());
    }
}
