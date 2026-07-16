use std::net::IpAddr;

use anyhow::{Context, Result};
use hickory_proto::op::{Message, MessageType, OpCode, Query};
use hickory_proto::rr::rdata::{A, AAAA};
use hickory_proto::rr::{Name, RData, RecordType};

/// A fresh DNS transaction id. Per-query randomness mostly matters for spoofing
/// resistance; here each query rides a dedicated tunnel UDP socket, so this just
/// lets us reject a stale datagram from a prior retransmit.
pub(super) fn dns_query_id() -> u16 {
    let mut bytes = [0u8; 2];
    let _ = getrandom::fill(&mut bytes);
    u16::from_ne_bytes(bytes)
}

/// Encode a recursive DNS query for `host` / `rtype` with transaction id `id`.
pub(super) fn build_dns_query(host: &str, rtype: RecordType, id: u16) -> Result<Vec<u8>> {
    let fqdn = if host.ends_with('.') {
        host.to_string()
    } else {
        format!("{host}.")
    };
    let name = Name::from_utf8(&fqdn).with_context(|| format!("invalid DNS name {host:?}"))?;
    let mut msg = Message::new();
    msg.set_id(id);
    msg.set_message_type(MessageType::Query);
    msg.set_op_code(OpCode::Query);
    msg.set_recursion_desired(true);
    msg.add_query(Query::query(name, rtype));
    msg.to_vec().context("encode DNS query")
}

/// Extract the first address of the requested family from a DNS response, after
/// checking the transaction id matches. Returns `None` for a mismatched id or a
/// response with no usable answer (e.g. NODATA / NXDOMAIN).
pub(super) fn parse_dns_answer(resp: &[u8], id: u16, rtype: RecordType) -> Option<IpAddr> {
    let msg = Message::from_vec(resp).ok()?;
    if msg.id() != id {
        return None;
    }
    for answer in msg.answers() {
        match (rtype, answer.data()) {
            (RecordType::A, Some(RData::A(A(ip)))) => return Some(IpAddr::V4(*ip)),
            (RecordType::AAAA, Some(RData::AAAA(AAAA(ip)))) => return Some(IpAddr::V6(*ip)),
            _ => {}
        }
    }
    None
}
