//! End-to-end proof that UDP rides a Snell outbound:
//! SOCKS5 UDP ASSOCIATE -> gripe inbound -> Snell UDP -> fake Snell UDP server.
//!
//! Snell UDP is UDP-over-TCP on the same shadowaead chunk stream as Snell TCP
//! (16-byte salt + Argon2id session subkey + `AEAD(len)|AEAD(payload)` chunks,
//! 12-byte LE counter nonce). The only departures are the handshake header
//! (`proto(1) | CommandUDP(6) | clientID-len(0)`, no host/port) and the
//! per-datagram framing: each datagram is exactly one AEAD chunk whose plaintext
//! is `UDPForward(0x01) | addr | payload` (client->server) or `addr | payload`
//! (server->client). The address is Snell's own form, not SOCKS5: a domain is
//! `len(1) | host | port`, an IP is `0x00 | family(4|6) | addr | port` going up
//! and `family(4|6) | addr | port` coming back.
//!
//! The fake server below is an *independent* re-implementation of that wire
//! format (TCP listener): it reads the client salt, derives the read subkey,
//! decrypts the `CommandUDP` handshake chunk, sends its own salt, then for each
//! client packet chunk recovers the destination + payload and echoes a reply
//! chunk back with a synthetic source address. Snell UDP is v3 only
//! (AES-128-GCM). We cover IPv4, IPv6 and domain destinations, multiple
//! datagrams on one association, a large datagram, and a `Routed` outbound.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use aes_gcm::Aes128Gcm;
use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use argon2::{Algorithm, Argon2, Params, Version};
use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, Router, SnellOutboundConfig};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream, UdpSocket};

const TEST_PSK: &[u8] = b"snell-udp-test-psk";
const SALT_LEN: usize = 16;
const TAG_LEN: usize = 16;
const KEY_SIZE: usize = 16; // v3 => AES-128-GCM
const COMMAND_UDP: u8 = 6;
const UDP_FORWARD: u8 = 1;

/// Snell's session-subkey KDF (independent of the kernel's copy).
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

/// Read and decrypt one AEAD chunk; returns `None` on clean EOF.
async fn read_chunk<S>(stream: &mut S, cipher: &Aes128Gcm, nonce: &mut [u8; 12]) -> Option<Vec<u8>>
where
    S: AsyncRead + Unpin,
{
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    if stream.read_exact(&mut sealed_len).await.is_err() {
        return None;
    }
    let len_plain = cipher
        .decrypt(
            GenericArray::from_slice(nonce),
            Payload {
                msg: &sealed_len,
                aad: &[],
            },
        )
        .expect("decrypt chunk length");
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_plain[0], len_plain[1]]) as usize;

    let mut sealed = vec![0u8; clen + TAG_LEN];
    stream.read_exact(&mut sealed).await.expect("read chunk body");
    let plain = cipher
        .decrypt(GenericArray::from_slice(nonce), Payload { msg: &sealed, aad: &[] })
        .expect("decrypt chunk body");
    increment_nonce(nonce);
    Some(plain)
}

/// Seal `plaintext` into one AEAD chunk and write it.
async fn write_chunk<S>(stream: &mut S, cipher: &Aes128Gcm, nonce: &mut [u8; 12], plaintext: &[u8])
where
    S: AsyncWrite + Unpin,
{
    let len = (plaintext.len() as u16).to_be_bytes();
    let sealed_len = cipher
        .encrypt(GenericArray::from_slice(nonce), Payload { msg: &len, aad: &[] })
        .unwrap();
    increment_nonce(nonce);
    let sealed = cipher
        .encrypt(
            GenericArray::from_slice(nonce),
            Payload {
                msg: plaintext,
                aad: &[],
            },
        )
        .unwrap();
    increment_nonce(nonce);
    stream.write_all(&sealed_len).await.unwrap();
    stream.write_all(&sealed).await.unwrap();
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

/// Build a server->client reply chunk: `family(4) | ipv4 | port | payload`.
fn reply_packet(payload: &[u8]) -> Vec<u8> {
    let mut out = vec![4u8];
    out.extend_from_slice(&Ipv4Addr::new(1, 2, 3, 4).octets());
    out.extend_from_slice(&443u16.to_be_bytes());
    out.extend_from_slice(payload);
    out
}

/// Validate the Snell UDP handshake, then echo each datagram chunk by chunk.
async fn serve_snell_udp(mut stream: TcpStream) {
    // Read client salt, derive the read cipher.
    let mut salt = [0u8; SALT_LEN];
    stream.read_exact(&mut salt).await.unwrap();
    let read_cipher = cipher(&snell_kdf(TEST_PSK, &salt));
    let mut read_nonce = [0u8; 12];

    // First chunk is the Snell UDP handshake header.
    let header = read_chunk(&mut stream, &read_cipher, &mut read_nonce)
        .await
        .expect("udp handshake header");
    assert_eq!(header, [1, COMMAND_UDP, 0], "snell udp handshake header");

    // Reply with our salt, then echo each client datagram as a reply chunk.
    let mut salt_w = [0u8; SALT_LEN];
    for (i, b) in salt_w.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(11).wrapping_add(3);
    }
    stream.write_all(&salt_w).await.unwrap();
    let write_cipher = cipher(&snell_kdf(TEST_PSK, &salt_w));
    let mut write_nonce = [0u8; 12];

    while let Some(packet) = read_chunk(&mut stream, &read_cipher, &mut read_nonce).await {
        let payload = parse_client_packet(&packet);
        write_chunk(&mut stream, &write_cipher, &mut write_nonce, &reply_packet(&payload)).await;
    }
}

async fn spawn_fake_snell_udp_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(serve_snell_udp(stream));
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

/// Drive `datagram` through the kernel on a fresh association and assert the
/// echoed payload round-trips verbatim.
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
async fn udp_relays_ipv4_destination() {
    let server = spawn_fake_snell_udp_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let payload = b"snell udp ipv4";
    assert_udp_relays(
        OutboundMode::Snell(snell(server, 3)),
        udp_datagram_ipv4(dst, payload),
        payload,
    )
    .await;
}

#[tokio::test]
async fn udp_relays_ipv6_destination() {
    let server = spawn_fake_snell_udp_server().await;
    let dst = SocketAddr::from((Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1), 53));
    let payload = b"snell udp ipv6";
    assert_udp_relays(
        OutboundMode::Snell(snell(server, 3)),
        udp_datagram_ipv6(dst, payload),
        payload,
    )
    .await;
}

#[tokio::test]
async fn udp_relays_domain_destination() {
    let server = spawn_fake_snell_udp_server().await;
    let payload = b"snell udp domain";
    assert_udp_relays(
        OutboundMode::Snell(snell(server, 3)),
        udp_datagram_domain("example.com", 443, payload),
        payload,
    )
    .await;
}

#[tokio::test]
async fn udp_relays_large_datagram() {
    let server = spawn_fake_snell_udp_server().await;
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let payload = vec![0x5au8; 1400];
    assert_udp_relays(
        OutboundMode::Snell(snell(server, 3)),
        udp_datagram_ipv4(dst, &payload),
        &payload,
    )
    .await;
}

#[tokio::test]
async fn udp_relays_multiple_datagrams_on_one_association() {
    let server = spawn_fake_snell_udp_server().await;
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: OutboundMode::Snell(snell(server, 3)),
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
async fn udp_relays_through_routed_snell() {
    let server = spawn_fake_snell_udp_server().await;
    let mut outbounds = HashMap::new();
    outbounds.insert("proxy".to_string(), OutboundMode::Snell(snell(server, 3)));
    let router = Router::new(outbounds, vec![], "proxy").unwrap();
    let dst = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let payload = b"routed snell udp";
    assert_udp_relays(
        OutboundMode::Routed(Box::new(router)),
        udp_datagram_ipv4(dst, payload),
        payload,
    )
    .await;
}
