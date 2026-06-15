use crate::{
    config::{Config, PrfOption},
    subscription::control_plane::subscription_update_uses_dedicated_control_plane,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl TransportPlan {
    pub async fn for_subscription_update(preferred_transport: Option<TransportKind>) -> Self {
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

        let (ordered_candidates, note) = collapse_equivalent_local_core_candidates(
            ordered_candidates,
            subscription_update_uses_dedicated_control_plane().await,
        );
        let preferred_transport = if note.is_some() {
            Some(TransportKind::LocalProxy)
        } else {
            preferred_transport
        };

        Self {
            ordered_candidates: prioritize_transport_candidates(ordered_candidates, preferred_transport),
            note,
        }
    }
}

fn collapse_equivalent_local_core_candidates(
    mut ordered_candidates: Vec<TransportCandidate>,
    subscription_update_uses_dedicated_control_plane: bool,
) -> (Vec<TransportCandidate>, Option<String>) {
    if !subscription_update_uses_dedicated_control_plane {
        return (ordered_candidates, None);
    }

    let has_direct = ordered_candidates
        .iter()
        .any(|candidate| candidate.kind == TransportKind::Direct);
    let has_local_proxy = ordered_candidates
        .iter()
        .any(|candidate| candidate.kind == TransportKind::LocalProxy);

    if !(has_direct && has_local_proxy) {
        return (ordered_candidates, None);
    }

    ordered_candidates.retain(|candidate| {
        if candidate.kind == TransportKind::LocalProxy {
            return true;
        }

        !matches!(candidate.kind, TransportKind::Direct | TransportKind::LocalProxy)
    });

    for candidate in &mut ordered_candidates {
        if candidate.kind == TransportKind::LocalProxy {
            candidate.reason = format!(
                "{}; current TUN + global mode routes subscription updates through a dedicated Mihomo control-plane group instead of the user-selected GLOBAL egress",
                candidate.reason
            )
            .into();
        }
    }

    (
        ordered_candidates,
        Some(
            "Current TUN + global mode makes OS direct sockets and Clash mixed-port requests share the same Mihomo egress, so subscription update will use a dedicated Mihomo control-plane group instead of retrying equivalent direct/local transports."
                .into(),
        ),
    )
}

fn prioritize_transport_candidates(
    mut ordered_candidates: Vec<TransportCandidate>,
    preferred_transport: Option<TransportKind>,
) -> Vec<TransportCandidate> {
    if let Some(preferred_transport) = preferred_transport
        && let Some(index) = ordered_candidates
            .iter()
            .position(|candidate| candidate.kind == preferred_transport)
    {
        let preferred_candidate = ordered_candidates.remove(index);
        ordered_candidates.insert(0, preferred_candidate);
    }

    ordered_candidates
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prioritize_transport_candidates_moves_preferred_transport_to_front() {
        let ordered_candidates = vec![
            TransportCandidate {
                kind: TransportKind::Direct,
                reason: "direct".into(),
            },
            TransportCandidate {
                kind: TransportKind::LocalProxy,
                reason: "local".into(),
            },
            TransportCandidate {
                kind: TransportKind::SystemProxy,
                reason: "system".into(),
            },
        ];

        let reordered = prioritize_transport_candidates(ordered_candidates, Some(TransportKind::LocalProxy));

        assert_eq!(
            reordered.iter().map(|candidate| candidate.kind).collect::<Vec<_>>(),
            vec![
                TransportKind::LocalProxy,
                TransportKind::Direct,
                TransportKind::SystemProxy
            ]
        );
    }

    #[test]
    fn prioritize_transport_candidates_keeps_order_when_preferred_transport_is_unavailable() {
        let ordered_candidates = vec![
            TransportCandidate {
                kind: TransportKind::Direct,
                reason: "direct".into(),
            },
            TransportCandidate {
                kind: TransportKind::LocalProxy,
                reason: "local".into(),
            },
        ];

        let reordered = prioritize_transport_candidates(ordered_candidates, Some(TransportKind::SystemProxy));

        assert_eq!(
            reordered.iter().map(|candidate| candidate.kind).collect::<Vec<_>>(),
            vec![TransportKind::Direct, TransportKind::LocalProxy]
        );
    }

    #[test]
    fn collapse_equivalent_local_core_candidates_keeps_only_preferred_representative() {
        let ordered_candidates = vec![
            TransportCandidate {
                kind: TransportKind::Direct,
                reason: "direct".into(),
            },
            TransportCandidate {
                kind: TransportKind::LocalProxy,
                reason: "local".into(),
            },
            TransportCandidate {
                kind: TransportKind::SystemProxy,
                reason: "system".into(),
            },
        ];

        let (collapsed, note) = collapse_equivalent_local_core_candidates(ordered_candidates, true);

        assert!(note.is_some());
        assert_eq!(
            collapsed.iter().map(|candidate| candidate.kind).collect::<Vec<_>>(),
            vec![TransportKind::LocalProxy, TransportKind::SystemProxy]
        );
        assert!(collapsed[0].reason.contains("dedicated Mihomo control-plane group"));
    }

    #[test]
    fn collapse_equivalent_local_core_candidates_leaves_candidates_unchanged_when_not_equivalent() {
        let ordered_candidates = vec![
            TransportCandidate {
                kind: TransportKind::Direct,
                reason: "direct".into(),
            },
            TransportCandidate {
                kind: TransportKind::LocalProxy,
                reason: "local".into(),
            },
        ];

        let (collapsed, note) = collapse_equivalent_local_core_candidates(ordered_candidates.clone(), false);

        assert!(note.is_none());
        assert_eq!(
            collapsed.iter().map(|candidate| candidate.kind).collect::<Vec<_>>(),
            ordered_candidates
                .iter()
                .map(|candidate| candidate.kind)
                .collect::<Vec<_>>()
        );
    }
}
