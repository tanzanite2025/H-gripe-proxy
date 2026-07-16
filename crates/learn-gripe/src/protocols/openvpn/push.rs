//! Minimal OpenVPN `PUSH_REPLY` parsing.
//!
//! Only the fields the data plane needs to come up are extracted: the assigned
//! tunnel IPv4 address (`ifconfig`) and the data-channel `peer-id`. Routes, DNS,
//! and redirect directives are ignored for this slice.

use std::net::Ipv4Addr;

use anyhow::{Result, bail};

use super::data::PEER_ID_UNSET;

pub(super) const PUSH_REQUEST: &str = "PUSH_REQUEST";

/// The subset of a parsed `PUSH_REPLY` the client uses.
pub(super) struct PushReply {
    /// Assigned tunnel IPv4 address (from `ifconfig`).
    pub(super) local_v4: Ipv4Addr,
    /// Data-channel peer id (`peer-id`), or [`PEER_ID_UNSET`].
    pub(super) peer_id: u32,
}

/// Parse a `PUSH_REPLY` control message. Fails when the message is not a push
/// reply or lacks the assigned `ifconfig` address.
pub(super) fn parse_push_reply(message: &str) -> Result<PushReply> {
    let message = message.trim_end_matches('\0');
    if !message.starts_with("PUSH_REPLY") {
        bail!("openvpn: unexpected push message {message:?}");
    }
    let mut local_v4: Option<Ipv4Addr> = None;
    let mut peer_id = PEER_ID_UNSET;
    for option in message.split(',').skip(1) {
        let fields: Vec<&str> = option.split_whitespace().collect();
        match fields.first().copied() {
            Some("ifconfig") if fields.len() >= 2 => {
                local_v4 = Some(
                    fields[1]
                        .parse::<Ipv4Addr>()
                        .map_err(|_| anyhow::anyhow!("openvpn: invalid pushed ifconfig address {:?}", fields[1]))?,
                );
            }
            Some("peer-id") if fields.len() >= 2 => {
                let id: u32 = fields[1]
                    .parse()
                    .map_err(|_| anyhow::anyhow!("openvpn: invalid pushed peer-id {:?}", fields[1]))?;
                if id > PEER_ID_UNSET {
                    bail!("openvpn: pushed peer-id {id} out of range");
                }
                peer_id = id;
            }
            _ => {}
        }
    }
    let Some(local_v4) = local_v4 else {
        bail!("openvpn: push reply missing ifconfig address");
    };
    Ok(PushReply { local_v4, peer_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ifconfig_and_peer_id() {
        let reply =
            parse_push_reply("PUSH_REPLY,ifconfig 10.8.0.2 255.255.255.0,peer-id 3,route-gateway 10.8.0.1\0").unwrap();
        assert_eq!(reply.local_v4, Ipv4Addr::new(10, 8, 0, 2));
        assert_eq!(reply.peer_id, 3);
    }

    #[test]
    fn missing_ifconfig_is_rejected() {
        assert!(parse_push_reply("PUSH_REPLY,peer-id 5").is_err());
    }

    #[test]
    fn non_push_reply_is_rejected() {
        assert!(parse_push_reply("AUTH_FAILED").is_err());
    }
}
