use crate::{
    config::PrfOption,
    subscription::transport::{TransportKind, transport_kind_from_option},
    utils::network::{NetworkManager, ProxyType},
};
use anyhow::{Context as _, Result};
use reqwest::header::HeaderMap;
use serde::Serialize;
use smartstring::alias::String;
use std::time::Duration;

#[derive(Debug, Clone, Serialize)]
pub struct ResponseMetadata {
    pub status_code: u16,
    pub content_type: Option<String>,
    pub content_length: usize,
    pub transport: TransportKind,
}

#[derive(Debug, Clone)]
pub struct FetchedSubscriptionPayload {
    pub body: String,
    pub headers: HeaderMap,
    pub metadata: ResponseMetadata,
}

fn proxy_type_from_transport(transport: TransportKind) -> ProxyType {
    match transport {
        TransportKind::Direct => ProxyType::None,
        TransportKind::LocalProxy => ProxyType::Localhost,
        TransportKind::SystemProxy => ProxyType::System,
    }
}

fn header_value(headers: &HeaderMap, key: &str) -> Option<String> {
    headers.get(key).and_then(|value| value.to_str().ok()).map(Into::into)
}

pub async fn fetch_remote_profile(url: &str, option: Option<&PrfOption>) -> Result<FetchedSubscriptionPayload> {
    let transport = transport_kind_from_option(option);
    let accept_invalid_certs = option.is_some_and(|current| current.danger_accept_invalid_certs.unwrap_or(false));
    let timeout_seconds = option.and_then(|current| current.timeout_seconds).unwrap_or(20);
    let user_agent = option.and_then(|current| current.user_agent.clone());

    let response = match NetworkManager::new()
        .get_with_interrupt(
            url,
            proxy_type_from_transport(transport),
            Some(timeout_seconds),
            user_agent,
            accept_invalid_certs,
        )
        .await
    {
        Ok(response) => response,
        Err(err) => {
            tokio::time::sleep(Duration::from_millis(100)).await;
            return Err(err).context("failed to fetch remote profile");
        }
    };

    let headers = response.headers().clone();
    let body: String = response.text_with_charset()?.into();
    let metadata = ResponseMetadata {
        status_code: response.status().as_u16(),
        content_type: header_value(&headers, "content-type"),
        content_length: body.len(),
        transport,
    };

    Ok(FetchedSubscriptionPayload {
        body,
        headers,
        metadata,
    })
}
