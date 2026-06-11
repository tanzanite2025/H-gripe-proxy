use crate::{
    config::{Config, PrfOption},
    utils::network::NetworkManager,
};
use clash_verge_logging::{Type, logging};
use serde::{Deserialize, Serialize};
use smartstring::alias::String;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransportKind {
    Direct,
    LocalProxy,
    SystemProxy,
}

impl TransportKind {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Direct => "direct update",
            Self::LocalProxy => "clash proxy update",
            Self::SystemProxy => "system proxy update",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TransportCandidate {
    pub kind: TransportKind,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct TransportPlan {
    pub ordered_candidates: Vec<TransportCandidate>,
}

impl TransportPlan {
    pub async fn for_subscription_update() -> Self {
        let mut ordered_candidates = vec![TransportCandidate {
            kind: TransportKind::Direct,
            reason: "Always attempt direct fetch first".into(),
        }];

        let local_proxy_port = {
            let verge_port = Config::verge().await.data_arc().verge_mixed_port;
            match verge_port {
                Some(port) => Some(port),
                None => Some(Config::clash().await.data_arc().get_mixed_port()),
            }
        };

        if let Some(port) = local_proxy_port {
            ordered_candidates.push(TransportCandidate {
                kind: TransportKind::LocalProxy,
                reason: format!("Local Clash/Mihomo mixed port {port} is configured").into(),
            });
        }

        match NetworkManager::system_proxy_url() {
            Ok(Some(proxy_url)) => {
                ordered_candidates.push(TransportCandidate {
                    kind: TransportKind::SystemProxy,
                    reason: format!("System proxy is enabled at {proxy_url}").into(),
                });
            }
            Ok(None) => {}
            Err(err) => {
                logging!(
                    warn,
                    Type::Config,
                    "Warning: [Subscription Update] failed to inspect system proxy settings, skip system proxy candidate: {}",
                    err
                );
            }
        }

        Self { ordered_candidates }
    }
}

pub fn transport_kind_from_option(option: Option<&PrfOption>) -> TransportKind {
    if option.is_some_and(|current| current.self_proxy.unwrap_or(false)) {
        TransportKind::LocalProxy
    } else if option.is_some_and(|current| current.with_proxy.unwrap_or(false)) {
        TransportKind::SystemProxy
    } else {
        TransportKind::Direct
    }
}

pub fn apply_transport_to_option(base: Option<&PrfOption>, transport: TransportKind) -> PrfOption {
    let mut option = base.cloned().unwrap_or_default();

    match transport {
        TransportKind::Direct => {
            option.self_proxy = Some(false);
            option.with_proxy = Some(false);
        }
        TransportKind::LocalProxy => {
            option.self_proxy = Some(true);
            option.with_proxy = Some(false);
        }
        TransportKind::SystemProxy => {
            option.self_proxy = Some(false);
            option.with_proxy = Some(true);
        }
    }

    option
}
