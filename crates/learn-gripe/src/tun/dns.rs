//! In-stack DNS: answer UDP :53 queries via the kernel DNS logic (fake-IP
//! allocation or upstream forwarding) so a global default route can capture all
//! traffic without black-holing name resolution.

use std::sync::Arc;

use tokio::sync::mpsc;

use crate::dns::{DnsMode, answer_query};

use super::udp::UdpDatagram;
use super::wire::build_udp_reply_frame;

/// Answer a DNS datagram in the background via the kernel DNS logic and emit the
/// reply frame back to the device.
pub(super) fn answer_dns(datagram: UdpDatagram, dns: &Arc<DnsMode>, frames_out: &mpsc::Sender<Vec<u8>>) {
    let dns = dns.clone();
    let frames_out = frames_out.clone();
    tokio::spawn(async move {
        match answer_query(&datagram.payload, &dns).await {
            Ok(response) => {
                if let Some(frame) = build_udp_reply_frame(&datagram.flow, &response) {
                    let _ = frames_out.send(frame).await;
                }
            }
            Err(err) => log::debug!("learn-gripe tun dns: dropped query: {err:#}"),
        }
    });
}
