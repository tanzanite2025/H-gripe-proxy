//! End-to-end test for the TUN inbound, without a real OS TUN device.
//!
//! A second, independent smoltcp stack plays the role of the OS networking
//! stack ("the client"): it performs a real TCP handshake to a destination,
//! sends bytes, and reads the reply — all as raw IP frames exchanged with
//! `learn_gripe::serve_tun` over in-memory channels. The kernel's outbound is
//! `Direct`, so the flow is dialed to a real tokio echo server. Real bytes
//! traverse two real TCP state machines plus the kernel relay.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use learn_gripe::{DEFAULT_MTU, OutboundMode, serve_tun};
use smoltcp::iface::{Config, Interface, SocketSet};
use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::socket::tcp;
use smoltcp::time::Instant as SmolInstant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr, IpEndpoint, Ipv4Address};
use std::collections::VecDeque;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::{Notify, mpsc};

#[tokio::test]
async fn tun_relays_tcp_flow_to_direct_outbound() {
    // Echo server reached via the kernel's Direct outbound.
    let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut sock, _) = echo.accept().await.unwrap();
        let mut buf = [0u8; 1024];
        loop {
            match sock.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if sock.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // In-memory TUN: client <-> kernel frame channels.
    let (to_kernel_tx, to_kernel_rx) = mpsc::channel::<Vec<u8>>(256);
    let (to_client_tx, to_client_rx) = mpsc::channel::<Vec<u8>>(256);

    let shutdown = Arc::new(Notify::new());
    let kernel = tokio::spawn(serve_tun(
        to_kernel_rx,
        to_client_tx,
        OutboundMode::Direct,
        None,
        shutdown.clone(),
        DEFAULT_MTU,
    ));

    let payload = b"hello tun relay".to_vec();
    let echoed = tokio::time::timeout(
        Duration::from_secs(10),
        run_client(echo_addr, payload.clone(), to_kernel_tx, to_client_rx),
    )
    .await
    .expect("client timed out");

    assert_eq!(echoed, payload, "bytes should round-trip through the TUN relay");

    shutdown.notify_waiters();
    kernel.abort();
}

#[tokio::test]
async fn tun_relays_multi_segment_payload() {
    let echo = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let echo_addr = echo.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut sock, _) = echo.accept().await.unwrap();
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            match sock.read(&mut buf).await {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if sock.write_all(&buf[..n]).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    let (to_kernel_tx, to_kernel_rx) = mpsc::channel::<Vec<u8>>(1024);
    let (to_client_tx, to_client_rx) = mpsc::channel::<Vec<u8>>(1024);
    let shutdown = Arc::new(Notify::new());
    let kernel = tokio::spawn(serve_tun(
        to_kernel_rx,
        to_client_tx,
        OutboundMode::Direct,
        None,
        shutdown.clone(),
        DEFAULT_MTU,
    ));

    // Much larger than the MTU and the socket buffers, so it spans many
    // segments and exercises back-pressure in both directions.
    let payload: Vec<u8> = (0..256 * 1024).map(|i| (i % 251) as u8).collect();
    let echoed = tokio::time::timeout(
        Duration::from_secs(20),
        run_client(echo_addr, payload.clone(), to_kernel_tx, to_client_rx),
    )
    .await
    .expect("client timed out");

    assert_eq!(echoed.len(), payload.len(), "all bytes should round-trip");
    assert_eq!(echoed, payload, "multi-segment payload should be intact");

    shutdown.notify_waiters();
    kernel.abort();
}

/// Drive a client smoltcp stack: connect to `remote`, send `payload`, read the
/// echo back, then close. Returns the bytes received.
async fn run_client(
    remote: SocketAddr,
    payload: Vec<u8>,
    to_kernel: mpsc::Sender<Vec<u8>>,
    mut from_kernel: mpsc::Receiver<Vec<u8>>,
) -> Vec<u8> {
    let start = Instant::now();
    let mut phy = ClientPhy::new(DEFAULT_MTU);
    let mut iface = Interface::new(Config::new(HardwareAddress::Ip), &mut phy, now(start));
    iface.update_ip_addrs(|addrs| {
        let _ = addrs.push(IpCidr::new(IpAddress::Ipv4(Ipv4Address::new(10, 0, 0, 1)), 24));
    });
    let _ = iface.routes_mut().add_default_ipv4_route(Ipv4Address::new(10, 0, 0, 1));

    let mut sockets = SocketSet::new(Vec::new());
    let handle = sockets.add(tcp::Socket::new(
        tcp::SocketBuffer::new(vec![0u8; 64 * 1024]),
        tcp::SocketBuffer::new(vec![0u8; 64 * 1024]),
    ));

    let remote_ep = IpEndpoint::new(
        IpAddress::Ipv4(Ipv4Address::from(match remote.ip() {
            std::net::IpAddr::V4(v4) => v4.octets(),
            std::net::IpAddr::V6(_) => panic!("test uses ipv4"),
        })),
        remote.port(),
    );
    {
        let sock = sockets.get_mut::<tcp::Socket>(handle);
        sock.connect(iface.context(), remote_ep, 40000u16).unwrap();
    }

    let mut sent_off = 0usize;
    let mut received: Vec<u8> = Vec::new();

    loop {
        iface.poll(now(start), &mut phy, &mut sockets);
        {
            let sock = sockets.get_mut::<tcp::Socket>(handle);
            while sent_off < payload.len() && sock.can_send() {
                match sock.send_slice(&payload[sent_off..]) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => sent_off += n,
                }
            }
            while sock.can_recv() {
                let chunk = sock.recv(|b| (b.len(), b.to_vec())).unwrap_or_default();
                if chunk.is_empty() {
                    break;
                }
                received.extend_from_slice(&chunk);
            }
            // The payload has round-tripped: initiate close, flush the FIN, and
            // return without blocking on the full TIME-WAIT teardown.
            if sent_off >= payload.len() && received.len() >= payload.len() {
                sock.close();
                drain_tx(&mut phy, &to_kernel).await;
                return received;
            }
        }

        drain_tx(&mut phy, &to_kernel).await;

        let delay = iface
            .poll_delay(now(start), &sockets)
            .map(|d| Duration::from_micros(d.total_micros()))
            .map_or(Duration::from_millis(5), |d| d.min(Duration::from_millis(5)));

        tokio::select! {
            frame = from_kernel.recv() => {
                match frame {
                    Some(frame) => phy.rx.push_back(frame),
                    None => return received,
                }
            }
            _ = tokio::time::sleep(delay) => {}
        }
    }
}

async fn drain_tx(phy: &mut ClientPhy, to_kernel: &mpsc::Sender<Vec<u8>>) {
    while let Some(frame) = phy.tx.pop_front() {
        if to_kernel.send(frame).await.is_err() {
            return;
        }
    }
}

fn now(start: Instant) -> SmolInstant {
    SmolInstant::from_micros(start.elapsed().as_micros() as i64)
}

struct ClientPhy {
    rx: VecDeque<Vec<u8>>,
    tx: VecDeque<Vec<u8>>,
    mtu: usize,
}

impl ClientPhy {
    fn new(mtu: usize) -> Self {
        Self {
            rx: VecDeque::new(),
            tx: VecDeque::new(),
            mtu,
        }
    }
}

struct ClientRxToken {
    buf: Vec<u8>,
}

struct ClientTxToken<'a> {
    tx: &'a mut VecDeque<Vec<u8>>,
}

impl Device for ClientPhy {
    type RxToken<'a> = ClientRxToken;
    type TxToken<'a> = ClientTxToken<'a>;

    fn receive(&mut self, _timestamp: SmolInstant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let buf = self.rx.pop_front()?;
        Some((ClientRxToken { buf }, ClientTxToken { tx: &mut self.tx }))
    }

    fn transmit(&mut self, _timestamp: SmolInstant) -> Option<Self::TxToken<'_>> {
        Some(ClientTxToken { tx: &mut self.tx })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = self.mtu;
        caps
    }
}

impl RxToken for ClientRxToken {
    fn consume<R, F: FnOnce(&[u8]) -> R>(self, f: F) -> R {
        f(&self.buf)
    }
}

impl TxToken for ClientTxToken<'_> {
    fn consume<R, F: FnOnce(&mut [u8]) -> R>(self, len: usize, f: F) -> R {
        let mut buf = vec![0u8; len];
        let result = f(&mut buf);
        self.tx.push_back(buf);
        result
    }
}
