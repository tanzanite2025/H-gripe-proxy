//! Minimal OpenVPN `PUSH_REPLY` parsing.
//!
//! Only the fields the data plane needs to come up are extracted: the assigned
//! tunnel addresses (`ifconfig` / `ifconfig-ipv6`), the data-channel `peer-id`,
//! and the keepalive timers (`keepalive` / `ping` / `ping-restart`). Routes,
//! DNS, and redirect directives are ignored for this slice.

use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Duration;

use anyhow::{Result, bail};

use super::data::PEER_ID_UNSET;

pub(super) const PUSH_REQUEST: &str = "PUSH_REQUEST";

/// The subset of a parsed `PUSH_REPLY` the client uses.
pub(super) struct PushReply {
    /// Assigned tunnel IPv4 address (from `ifconfig`), when pushed.
    pub(super) local_v4: Option<Ipv4Addr>,
    /// Assigned tunnel IPv6 address (from `ifconfig-ipv6`), when pushed.
    pub(super) local_v6: Option<Ipv6Addr>,
    /// Data-channel peer id (`peer-id`), or [`PEER_ID_UNSET`].
    pub(super) peer_id: u32,
    /// Send a data-channel ping after this much send-side idle time
    /// (`ping n`, or the first argument of `keepalive n m`). 0 disables.
    pub(super) ping_interval: Option<Duration>,
    /// Tear the tunnel down after this much receive-side silence
    /// (`ping-restart n`, or the second argument of `keepalive n m`).
    /// 0 disables.
    pub(super) ping_restart: Option<Duration>,
}

/// Parse a `PUSH_REPLY` control message. Fails when the message is not a push
/// reply or lacks an assigned tunnel address (`ifconfig` / `ifconfig-ipv6`).
pub(super) fn parse_push_reply(message: &str) -> Result<PushReply> {
    let message = message.trim_end_matches('\0');
    if !message.starts_with("PUSH_REPLY") {
        bail!("openvpn: unexpected push message {message:?}");
    }
    let mut local_v4: Option<Ipv4Addr> = None;
    let mut local_v6: Option<Ipv6Addr> = None;
    let mut peer_id = PEER_ID_UNSET;
    let mut ping_interval: Option<Duration> = None;
    let mut ping_restart: Option<Duration> = None;
    // A later option overrides an earlier one, like upstream's option parser
    // (`keepalive n m` is exactly a `ping n` + `ping-restart m` macro).
    let parse_secs = |value: &str, name: &str| -> Result<Option<Duration>> {
        let secs: u64 = value
            .parse()
            .map_err(|_| anyhow::anyhow!("openvpn: invalid pushed {name} {value:?}"))?;
        Ok((secs > 0).then(|| Duration::from_secs(secs)))
    };
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
            Some("ifconfig-ipv6") if fields.len() >= 2 => {
                // `ifconfig-ipv6 <addr>/<prefix> <remote>`; only the address
                // matters (the tunnel is the only egress).
                let addr = fields[1].split('/').next().unwrap_or(fields[1]);
                local_v6 =
                    Some(addr.parse::<Ipv6Addr>().map_err(|_| {
                        anyhow::anyhow!("openvpn: invalid pushed ifconfig-ipv6 address {:?}", fields[1])
                    })?);
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
            Some("keepalive") if fields.len() >= 3 => {
                ping_interval = parse_secs(fields[1], "keepalive interval")?;
                ping_restart = parse_secs(fields[2], "keepalive timeout")?;
            }
            Some("ping") if fields.len() >= 2 => {
                ping_interval = parse_secs(fields[1], "ping")?;
            }
            Some("ping-restart") if fields.len() >= 2 => {
                ping_restart = parse_secs(fields[1], "ping-restart")?;
            }
            _ => {}
        }
    }
    if local_v4.is_none() && local_v6.is_none() {
        bail!("openvpn: push reply missing ifconfig / ifconfig-ipv6 address");
    }
    Ok(PushReply {
        local_v4,
        local_v6,
        peer_id,
        ping_interval,
        ping_restart,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ifconfig_and_peer_id() {
        let reply =
            parse_push_reply("PUSH_REPLY,ifconfig 10.8.0.2 255.255.255.0,peer-id 3,route-gateway 10.8.0.1\0").unwrap();
        assert_eq!(reply.local_v4, Some(Ipv4Addr::new(10, 8, 0, 2)));
        assert_eq!(reply.local_v6, None);
        assert_eq!(reply.peer_id, 3);
        assert_eq!(reply.ping_interval, None);
        assert_eq!(reply.ping_restart, None);
    }

    #[test]
    fn parses_keepalive_timers() {
        let reply = parse_push_reply("PUSH_REPLY,ifconfig 10.8.0.2 255.255.255.0,keepalive 10 60\0").unwrap();
        assert_eq!(reply.ping_interval, Some(Duration::from_secs(10)));
        assert_eq!(reply.ping_restart, Some(Duration::from_secs(60)));
    }

    #[test]
    fn explicit_ping_options_override_keepalive() {
        let reply = parse_push_reply("PUSH_REPLY,ifconfig 10.8.0.2,keepalive 10 60,ping 5,ping-restart 0\0").unwrap();
        assert_eq!(reply.ping_interval, Some(Duration::from_secs(5)));
        assert_eq!(reply.ping_restart, None);
    }

    #[test]
    fn invalid_keepalive_is_rejected() {
        assert!(parse_push_reply("PUSH_REPLY,ifconfig 10.8.0.2,keepalive ten 60").is_err());
    }

    #[test]
    fn parses_ifconfig_ipv6() {
        let reply =
            parse_push_reply("PUSH_REPLY,ifconfig 10.8.0.2 255.255.255.0,ifconfig-ipv6 fd00:8::2/64 fd00:8::1\0")
                .unwrap();
        assert_eq!(reply.local_v4, Some(Ipv4Addr::new(10, 8, 0, 2)));
        assert_eq!(reply.local_v6, Some("fd00:8::2".parse().unwrap()));
    }

    #[test]
    fn accepts_ipv6_only_push() {
        let reply = parse_push_reply("PUSH_REPLY,ifconfig-ipv6 fd00:8::2/64 fd00:8::1,peer-id 2\0").unwrap();
        assert_eq!(reply.local_v4, None);
        assert_eq!(reply.local_v6, Some("fd00:8::2".parse().unwrap()));
        assert_eq!(reply.peer_id, 2);
    }

    #[test]
    fn invalid_ifconfig_ipv6_is_rejected() {
        assert!(parse_push_reply("PUSH_REPLY,ifconfig-ipv6 not-an-addr/64 fd00:8::1").is_err());
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
