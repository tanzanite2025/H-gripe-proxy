use crate::{
    config::{Config, PrfItem},
    subscription::{
        model::{SubscriptionSourceConfig, SubscriptionSourceState},
        persist::read_subscription_state_document,
        transport::{TransportKind, transport_kind_from_option},
    },
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
    let state = read_subscription_state_document().await?;
    let profiles = Config::profiles().await;
    let profiles = profiles.latest_arc();
    let current_source_id = profiles.get_current().map(String::as_str);
    let sources = state
        .sources
        .iter()
        .filter_map(|source_state| {
            let source_config = source_state.source_config.as_ref()?;
            let profile_item = profiles.get_item(source_state.source_id.as_str()).ok();
            Some(subscription_source_from_state(
                source_state,
                source_config,
                profile_item,
                current_source_id,
            ))
        })
        .collect();

    Ok(sources)
}

pub async fn get_subscription_source(source_id: &str) -> Result<Option<SubscriptionSource>> {
    Ok(list_subscription_sources()
        .await?
        .into_iter()
        .find(|source| source.source_id.as_str() == source_id))
}

pub fn subscription_source_from_state(
    source_state: &SubscriptionSourceState,
    source_config: &SubscriptionSourceConfig,
    profile_item: Option<&PrfItem>,
    current_source_id: Option<&str>,
) -> SubscriptionSource {
    let source_id = source_state.source_id.clone();
    let option = source_config.option.as_ref();
    let is_current = current_source_id == Some(source_id.as_str());

    SubscriptionSource {
        source_id,
        name: profile_item.and_then(|item| item.name.clone()),
        url: Some(source_config.url.clone()),
        home: profile_item.and_then(|item| item.home.clone()),
        description: profile_item.and_then(|item| item.desc.clone()),
        updated: source_state
            .latest_success
            .as_ref()
            .map(|attempt| (attempt.finished_at / 1000) as usize)
            .or_else(|| Some((source_config.updated_at / 1000) as usize)),
        update_interval: option.and_then(|option| option.update_interval),
        allow_auto_update: option.and_then(|option| option.allow_auto_update).unwrap_or(true),
        preferred_transport: transport_kind_from_option(option),
        timeout_seconds: option.and_then(|option| option.timeout_seconds),
        danger_accept_invalid_certs: option
            .and_then(|option| option.danger_accept_invalid_certs)
            .unwrap_or(false),
        is_current,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PrfOption;

    #[test]
    fn maps_state_source_config_to_subscription_source() {
        let source_state = SubscriptionSourceState {
            source_id: "source-a".into(),
            source_config: None,
            active_artifact_version: None,
            latest_artifact: None,
            latest_attempt: None,
            latest_success: None,
        };
        let source_config = SubscriptionSourceConfig {
            url: "https://example.com/sub.yaml".into(),
            updated_at: 123000,
            option: Some(PrfOption {
                self_proxy: Some(true),
                update_interval: Some(24),
                allow_auto_update: Some(false),
                timeout_seconds: Some(30),
                danger_accept_invalid_certs: Some(true),
                ..PrfOption::default()
            }),
        };
        let item = PrfItem {
            name: Some("Source A".into()),
            desc: Some("primary subscription".into()),
            ..PrfItem::default()
        };

        let source = subscription_source_from_state(&source_state, &source_config, Some(&item), Some("source-a"));

        assert_eq!(source.source_id, "source-a");
        assert_eq!(source.name.as_deref(), Some("Source A"));
        assert_eq!(source.url.as_deref(), Some("https://example.com/sub.yaml"));
        assert_eq!(source.updated, Some(123));
        assert_eq!(source.preferred_transport, TransportKind::LocalProxy);
        assert_eq!(source.update_interval, Some(24));
        assert!(!source.allow_auto_update);
        assert_eq!(source.timeout_seconds, Some(30));
        assert!(source.danger_accept_invalid_certs);
        assert!(source.is_current);
    }

    #[test]
    fn maps_state_source_without_profile_metadata() {
        let source_state = SubscriptionSourceState {
            source_id: "source-a".into(),
            source_config: None,
            active_artifact_version: None,
            latest_artifact: None,
            latest_attempt: None,
            latest_success: None,
        };
        let source_config = SubscriptionSourceConfig {
            url: "https://example.com/sub.yaml".into(),
            updated_at: 123000,
            option: None,
        };

        let source = subscription_source_from_state(&source_state, &source_config, None, None);

        assert_eq!(source.source_id, "source-a");
        assert!(source.name.is_none());
        assert_eq!(source.url.as_deref(), Some("https://example.com/sub.yaml"));
        assert!(source.allow_auto_update);
        assert_eq!(source.preferred_transport, TransportKind::Direct);
        assert!(!source.is_current);
    }
}
