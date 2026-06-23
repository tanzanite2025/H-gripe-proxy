use super::super::RustSocksUdpFragmentsPacketEvidence;
use super::{
    constants::{
        SOCKS_UDP_ECHO_PREFIX, SOCKS_UDP_FRAGMENT_FINAL_TWO, SOCKS_UDP_FRAGMENT_ONE, TEST_FRAGMENT_ONE,
        TEST_FRAGMENT_TWO,
    },
    protocol::{encode_socks_udp_fragment, parse_socks_udp_fragment},
    reassembly::{ensure_loopback_target, reassemble_ordered_fragments},
};
use anyhow::{Context as _, Result, anyhow};
use std::{
    net::{Ipv4Addr, UdpSocket},
    thread,
    time::Duration,
};

pub(super) fn run_bounded_socks_udp_fragment_reassembly() -> Result<RustSocksUdpFragmentsPacketEvidence> {
    let echo = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind UDP target")?;
    echo.set_read_timeout(Some(Duration::from_secs(2)))?;
    echo.set_write_timeout(Some(Duration::from_secs(2)))?;
    let target = echo.local_addr()?;
    let echo_thread = thread::spawn(move || -> Result<usize> {
        let mut buffer = [0_u8; 512];
        let (received, peer) = echo.recv_from(&mut buffer)?;
        let mut response = SOCKS_UDP_ECHO_PREFIX.to_vec();
        response.extend_from_slice(&buffer[..received]);
        echo.send_to(&response, peer)?;
        Ok(received)
    });

    let fragments = [
        parse_socks_udp_fragment(&encode_socks_udp_fragment(
            target,
            SOCKS_UDP_FRAGMENT_ONE,
            TEST_FRAGMENT_ONE,
        ))?,
        parse_socks_udp_fragment(&encode_socks_udp_fragment(
            target,
            SOCKS_UDP_FRAGMENT_FINAL_TWO,
            TEST_FRAGMENT_TWO,
        ))?,
    ];
    let reassembled = reassemble_ordered_fragments(&fragments)?;
    ensure_loopback_target(target)?;

    let relay = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).context("failed to bind UDP relay")?;
    relay.set_read_timeout(Some(Duration::from_secs(2)))?;
    relay.set_write_timeout(Some(Duration::from_secs(2)))?;
    relay.send_to(&reassembled, target)?;
    let mut response = [0_u8; 512];
    let (response_len, response_peer) = relay.recv_from(&mut response)?;
    let target_received_bytes = echo_thread
        .join()
        .map_err(|_| anyhow!("UDP target thread panicked"))??;
    ensure_loopback_target(response_peer)?;

    let response_payload = &response[..response_len];
    let response_payload_prefix =
        std::str::from_utf8(&response_payload[..SOCKS_UDP_ECHO_PREFIX.len().min(response_len)])
            .unwrap_or_default()
            .to_string();
    let expected_payload_bytes = TEST_FRAGMENT_ONE.len() + TEST_FRAGMENT_TWO.len();

    Ok(RustSocksUdpFragmentsPacketEvidence {
        target_addr: target.ip().to_string().into(),
        target_port: target.port(),
        fragment_count: fragments.len(),
        first_fragment: format!("0x{SOCKS_UDP_FRAGMENT_ONE:02x}").into(),
        final_fragment: format!("0x{SOCKS_UDP_FRAGMENT_FINAL_TWO:02x}").into(),
        request_payload_bytes: expected_payload_bytes,
        reassembled_payload_bytes: reassembled.len(),
        target_received_bytes,
        response_payload_bytes: response_len,
        response_payload_prefix: response_payload_prefix.into(),
        fragments_reassembled: reassembled.len() == expected_payload_bytes,
        datagram_round_trip: response_payload.starts_with(SOCKS_UDP_ECHO_PREFIX),
        loopback_only: target.ip().is_loopback() && response_peer.ip().is_loopback(),
    })
}
