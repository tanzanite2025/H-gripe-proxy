//! End-to-end proof that UDP rides a Snell **v4/v5** outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> Snell v4 UDP -> fake v4 UDP server.
//!
//! The datagram framing is identical to v3 (handshake `proto | CommandUDP(6) |
//! clientID-len(0)`, per-packet `UDPForward(0x01) | addr | payload` up and
//! `family(4|6) | addr | port | payload` down). Only the transport differs:
//! each datagram is one v4 frame (`AEAD(header) | [padding] | AEAD(payload)`)
//! rather than a shadowaead chunk, the salt + initial padding ride the handshake
//! frame, and — unlike v3 — the server sends a one-byte command response
//! (`Tunnel`) before any reply datagram.
//!
//! The fake server below is an *independent* re-implementation of that v4 wire
//! format (TCP listener): it reads the client salt, decrypts the handshake
//! frame, writes its own salt + a `Tunnel` reply frame, then echoes each client
//! datagram frame back with a synthetic source address.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use argon2::{Algorithm, Argon2, Params, Version};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, Router, SnellOutboundConfig};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};

const TEST_PSK: &[u8] = b"snell-v4-udp-test-psk";
const SALT_LEN: usize = 16;
const TAG_LEN: usize = 16;
const KEY_SIZE: usize = 16; // v4 => AES-128-GCM
const COMMAND_UDP: u8 = 6;
const UDP_FORWARD: u8 = 1;
const HEADER_PLAIN: usize = 7;
const HEADER_CIPHER: usize = HEADER_PLAIN + TAG_LEN;
const FRAME_BYTE: u8 = 4;
const RESP_TUNNEL: u8 = 0;
/// The fake server's own first-frame padding length (the client must tolerate
/// any value); kept in the v4 range to mirror a real peer.
const SERVER_INITIAL_PADDING: usize = 0x180;

fn snell_kdf(psk: &[u8], salt: &[u8]) -> Vec<u8> {
    let params = Params::new(8, 3, 1, Some(32)).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut out = [0u8; 32];
    argon2.hash_password_into(psk, salt, &mut out).unwrap();
    out[..KEY_SIZE].to_vec()
}

fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

fn cipher(subkey: &[u8]) -> Aes128Gcm {
    Aes128Gcm::new_from_slice(subkey).unwrap()
}

fn seal(c: &Aes128Gcm, nonce: &[u8; 12], plaintext: &[u8]) -> Vec<u8> {
    c.encrypt(
        GenericArray::from_slice(nonce),
        Payload {
            msg: plaintext,
            aad: &[],
        },
    )
    .unwrap()
}

fn open(c: &Aes128Gcm, nonce: &[u8; 12], ciphertext: &[u8]) -> Vec<u8> {
    c.decrypt(
        GenericArray::from_slice(nonce),
        Payload {
            msg: ciphertext,
            aad: &[],
        },
    )
    .unwrap()
}

/// Its own inverse: swap every even byte between padding and payload ciphertext.
fn swap_padding(padding: &mut [u8], payload_cipher: &mut [u8]) {
    let limit = padding.len().min(payload_cipher.len());
    let mut i = 0;
    while i < limit {
        std::mem::swap(&mut padding[i], &mut payload_cipher[i]);
        i += 2;
    }
}

/// Read and decrypt one v4 frame; returns `None` on clean EOF.
async fn read_frame(stream: &mut TcpStream, c: &Aes128Gcm, nonce: &mut [u8; 12]) -> Option<Vec<u8>> {
    let mut header_cipher = [0u8; HEADER_CIPHER];
    if stream.read_exact(&mut header_cipher).await.is_err() {
        return None;
    }
    let header = open(c, nonce, &header_cipher);
    increment_nonce(nonce);
    assert_eq!(header[0], FRAME_BYTE, "v4 frame marker");
    let padding = u16::from_be_bytes([header[3], header[4]]) as usize;
    let payload = u16::from_be_bytes([header[5], header[6]]) as usize;
    let mut frame = vec![0u8; padding + payload + TAG_LEN];
    stream.read_exact(&mut frame).await.expect("read frame body");
    if padding > 0 {
        let (pad_part, pay_part) = frame.split_at_mut(padding);
        swap_padding(pad_part, pay_part);
    }
    let plain = open(c, nonce, &frame[padding..]);
    increment_nonce(nonce);
    Some(plain)
}

/// Seal `payload` into one v4 frame (padding on the first frame only).
async fn write_frame(
    stream: &mut TcpStream,
    c: &Aes128Gcm,
    nonce: &mut [u8; 12],
    salt_sent: &mut bool,
    payload: &[u8],
) {
    let first = !*salt_sent;
    let padding_len = if first && !payload.is_empty() {
        SERVER_INITIAL_PADDING
    } else {
        0
    };

    let mut header = [0u8; HEADER_PLAIN];
    header[0] = FRAME_BYTE;
    header[3..5].copy_from_slice(&(padding_len as u16).to_be_bytes());
    header[5..7].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    let sealed_header = seal(c, nonce, &header);
    increment_nonce(nonce);
    let mut payload_cipher = seal(c, nonce, payload);
    increment_nonce(nonce);

    let mut out = sealed_header;
    if padding_len > 0 {
        let mut padding = vec![0u8; padding_len];
        for (i, b) in padding.iter_mut().enumerate() {
            *b = (i as u8).wrapping_mul(31).wrapping_add(7);
        }
        swap_padding(&mut padding, &mut payload_cipher);
        out.extend_from_slice(&padding);
    }
    out.extend_from_slice(&payload_cipher);
    *salt_sent = true;
    stream.write_all(&out).await.unwrap();
    stream.flush().await.unwrap();
}

/// Strip the client's `UDPForward | addr | payload` framing, returning the
/// payload (the destination address is parsed and discarded).
fn parse_client_packet(plain: &[u8]) -> Vec<u8> {
    assert_eq!(plain[0], UDP_FORWARD, "client UDP forward command");
    let rest = &plain[1..];
    let addr_len = match rest[0] {
        0x00 => match rest[1] {
            4 => 2 + 4 + 2,
            6 => 2 + 16 + 2,
            other => panic!("unknown snell IP family {other}"),
        },
        host_len => 1 + host_len as usize + 2,
    };
    rest[addr_len..].to_vec()
}

/// Build a server->client reply: `family(4) | ipv4 | port | payload`.
fn reply_packet(payload: &[u8]) -> Vec<u8> {
    let mut out = vec![4u8];
    out.extend_from_slice(&Ipv4Addr::new(1, 2, 3, 4).octets());
    out.extend_from_slice(&443u16.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Validate the v4 UDP handshake, send the `Tunnel` reply, then echo datagrams.
async fn serve_v4_udp(mut stream: TcpStream) {
    let mut salt = [0u8; SALT_LEN];
    stream.read_exact(&mut salt).await.unwrap();
    let read_cipher = cipher(&snell_kdf(TEST_PSK, &salt));
    let mut read_nonce = [0u8; 12];

    // First frame is the v4 UDP handshake header (carries the client's salt +
    // initial padding; the salt was already consumed above).
    let header = read_frame(&mut stream, &read_cipher, &mut read_nonce)
        .await
        .expect("udp handshake header");
    assert_eq!(header, [1, COMMAND_UDP, 0], "snell v4 udp handshake header");

    let mut salt_w = [0u8; SALT_LEN];
    for (i, b) in salt_w.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(11).wrapping_add(3);
    }
    stream.write_all(&salt_w).await.unwrap();
    let write_cipher = cipher(&snell_kdf(TEST_PSK, &salt_w));
    let mut write_nonce = [0u8; 12];
    let mut write_salt_sent = false;

    // v4 sends a one-byte command response before any reply datagram.
    write_frame(
        &mut stream,
        &write_cipher,
        &mut write_nonce,
        &mut write_salt_sent,
        &[RESP_TUNNEL],
    )
    .await;

    while let Some(packet) = read_frame(&mut stream, &read_cipher, &mut read_nonce).await {
        let payload = parse_client_packet(&packet);
        write_frame(
            &mut stream,
            &write_cipher,
            &mut write_nonce,
            &mut write_salt_sent,
            &reply_packet(&payload),
        )
        .await;
    }
}

async fn spawn_fake_v4_udp_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_v4_udp(stream));
        }
    });
    addr
}

fn snell(server: SocketAddr, version: u8) -> Box<SnellOutboundConfig> {
    Box::new(SnellOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        psk: TEST_PSK.to_vec(),
        version,
        obfs: None,
    })
}

// ---------------------------------------------------------------------------
// SOCKS5 UDP ASSOCIATE harness
// ---------------------------------------------------------------------------

async fn socks5_udp_associate(proxy: SocketAddr) -> (TcpStream, SocketAddr) {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);
    stream
        .write_all(&[0x05, 0x03, 0x00, 0x01, 0, 0, 0, 0, 0, 0])
        .await
        .unwrap();
    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[1], 0x00, "expected ASSOCIATE success reply");
    let ip = Ipv4Addr::new(reply[4], reply[5], reply[6], reply[7]);
    let port = u16::from_be_bytes([reply[8], reply[9]]);
    (stream, SocketAddr::from((ip, port)))
}

fn udp_datagram_ipv4(dst: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let ip = match dst.ip() {
        IpAddr::V4(v4) => v4.octets(),
        IpAddr::V6(_) => panic!("ipv4 helper"),
    };
    let mut datagram = vec![0x00, 0x00, 0x00, 0x01];
    datagram.extend_from_slice(&ip);
    datagram.extend_from_slice(&dst.port().to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

fn udp_datagram_ipv6(dst: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let ip = match dst.ip() {
        IpAddr::V6(v6) => v6.octets(),
        IpAddr::V4(_) => panic!("ipv6 helper"),
    };
    let mut datagram = vec![0x00, 0x00, 0x00, 0x04];
    datagram.extend_from_slice(&ip);
    datagram.extend_from_slice(&dst.port().to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

fn udp_datagram_domain(host: &str, port: u16, payload: &[u8]) -> Vec<u8> {
    let mut datagram = vec![0x00, 0x00, 0x00, 0x03, host.len() as u8];
    datagram.extend_from_slice(host.as_bytes());
    datagram.extend_from_slice(&port.to_be_bytes());
    datagram.extend_from_slice(payload);
    datagram
}

fn payload_offset(buf: &[u8]) -> usize {
    match buf[3] {
        0x01 => 3 + 1 + 4 + 2,
        0x04 => 3 + 1 + 16 + 2,
        0x03 => 3 + 1 + 1 + buf[4] as usize + 2,
        other => panic!("unexpected reply atyp {other}"),
    }
}

async fn assert_udp_relays(outbound: OutboundMode, datagram: Vec<u8>, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    client.send_to(&datagram, relay).await.unwrap();

    let mut buf = vec![0u8; payload.len() + 64];
    let (n, from) = client.recv_from(&mut buf).await.unwrap();
    assert_eq!(from, relay, "reply must come from the relay socket");
    let offset = payload_offset(&buf[..n]);
    assert_eq!(&buf[offset..n], payload, "payload must be echoed verbatim");

    handle.shutdown().await;
}

#[tokio::test]
async fn v4_udp_relays_ipv4_destination() {
    let server = spawn_fake_v4_udp_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let payload = b"snell v4 udp ipv4";
    assert_udp_relays(
        OutboundMode::Snell(snell(server, 4)),
        udp_datagram_ipv4(dst, payload),
        payload,
    )
    .await;
}

#[tokio::test]
async fn v4_udp_relays_ipv6_destination() {
    let server = spawn_fake_v4_udp_server().await;
    let dst = SocketAddr::from((Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1), 53));
    let payload = b"snell v4 udp ipv6";
    assert_udp_relays(
        OutboundMode::Snell(snell(server, 4)),
        udp_datagram_ipv6(dst, payload),
        payload,
    )
    .await;
}

#[tokio::test]
async fn v4_udp_relays_domain_destination() {
    let server = spawn_fake_v4_udp_server().await;
    let payload = b"snell v4 udp domain";
    assert_udp_relays(
        OutboundMode::Snell(snell(server, 4)),
        udp_datagram_domain("example.com", 443, payload),
        payload,
    )
    .await;
}

#[tokio::test]
async fn v4_udp_relays_large_datagram() {
    let server = spawn_fake_v4_udp_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let payload = vec![0x5au8; 1400];
    assert_udp_relays(
        OutboundMode::Snell(snell(server, 4)),
        udp_datagram_ipv4(dst, &payload),
        &payload,
    )
    .await;
}

#[tokio::test]
async fn v5_udp_dials_as_v4() {
    let server = spawn_fake_v4_udp_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let payload = b"snell v5 udp";
    // version 5 takes the v4 frame path (version >= 4).
    assert_udp_relays(
        OutboundMode::Snell(snell(server, 5)),
        udp_datagram_ipv4(dst, payload),
        payload,
    )
    .await;
}

#[tokio::test]
async fn v4_udp_relays_multiple_datagrams_on_one_association() {
    let server = spawn_fake_v4_udp_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell(server, 4)),
    })
    .await
    .unwrap();

    let (_control, relay) = socks5_udp_associate(handle.local_addr()).await;
    let client = UdpSocket::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));

    for i in 0..5u8 {
        let payload = vec![i; 100 + i as usize];
        client.send_to(&udp_datagram_ipv4(dst, &payload), relay).await.unwrap();
        let mut buf = vec![0u8; payload.len() + 64];
        let (n, _) = client.recv_from(&mut buf).await.unwrap();
        let offset = payload_offset(&buf[..n]);
        assert_eq!(&buf[offset..n], &payload[..], "datagram {i} must echo verbatim");
    }

    handle.shutdown().await;
}

#[tokio::test]
async fn v4_udp_relays_through_routed_snell() {
    let server = spawn_fake_v4_udp_server().await;
    let mut outbounds = HashMap::new();
    outbounds.insert("proxy".to_string(), OutboundMode::Snell(snell(server, 4)));
    let router = Router::new(outbounds, vec![], "proxy").unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let payload = b"routed snell v4 udp";
    assert_udp_relays(
        OutboundMode::Routed(Box::new(router)),
        udp_datagram_ipv4(dst, payload),
        payload,
    )
    .await;
}
