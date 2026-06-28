//! End-to-end proof for the SSH outbound: a SOCKS5 client -> gripe inbound ->
//! SSH `direct-tcpip` outbound -> SSH server -> echo server.
//!
//! The peer is a real SSH server (the `russh` server side, an independent code
//! path from the client outbound) so the test exercises the genuine on-wire SSH
//! transport, authentication, and channel forwarding rather than gripe talking
//! to itself. Password and public-key authentication and host-key pinning are
//! all covered.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

use learn_gripe::{GripeConfig, GripeKernel, OutboundMode, SshAuth, SshOutboundConfig};
use russh::keys::{PublicKey, decode_secret_key};
use russh::server::{Auth, Config, Handler, Msg, Session, run_stream};
use russh::{Channel, ChannelId};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

// Test-only key material (not real credentials).
const SERVER_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\n\
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n\
QyNTUxOQAAACAiZA6RO0eY8DEF7ViRcVuf5jOf75gu63lSDn1wjM0BHwAAAJD3QGQ/90Bk\n\
PwAAAAtzc2gtZWQyNTUxOQAAACAiZA6RO0eY8DEF7ViRcVuf5jOf75gu63lSDn1wjM0BHw\n\
AAAEC/XMXR5aqzA+Hh9pCXYzG9g/Vm0Yn7PxHV1OKeaA4oYiJkDpE7R5jwMQXtWJFxW5/m\n\
M5/vmC7reVIOfXCMzQEfAAAACmdyaXBlLXRlc3QBAgM=\n\
-----END OPENSSH PRIVATE KEY-----\n";
const SERVER_PUB: &str = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAICJkDpE7R5jwMQXtWJFxW5/mM5/vmC7reVIOfXCMzQEf gripe-test";

const CLIENT_KEY: &str = "-----BEGIN OPENSSH PRIVATE KEY-----\n\
b3BlbnNzaC1rZXktdjEAAAAABG5vbmUAAAAEbm9uZQAAAAAAAAABAAAAMwAAAAtzc2gtZW\n\
QyNTUxOQAAACA4MD4wt0Q5NvzM5mp5IfHTcJ9iZ1tJR4/ZC+qzadF+8gAAAJDyL3MH8i9z\n\
BwAAAAtzc2gtZWQyNTUxOQAAACA4MD4wt0Q5NvzM5mp5IfHTcJ9iZ1tJR4/ZC+qzadF+8g\n\
AAAEC4G4oQ5s4gnxzIQ4cm42yXSgkQvVOBzlHusfTW2MoH4zgwPjC3RDk2/Mzmankh8dNw\n\
n2JnW0lHj9kL6rNp0X7yAAAADGdyaXBlLWNsaWVudAE=\n\
-----END OPENSSH PRIVATE KEY-----\n";
const CLIENT_PUB: &str =
    "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIDgwPjC3RDk2/Mzmankh8dNwn2JnW0lHj9kL6rNp0X7y gripe-client";

const SSH_USER: &str = "tester";
const SSH_PASSWORD: &str = "s3cr3t";

async fn spawn_echo_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                loop {
                    match stream.read(&mut buf).await {
                        Ok(0) | Err(_) => return,
                        Ok(n) => {
                            if stream.write_all(&buf[..n]).await.is_err() {
                                return;
                            }
                        }
                    }
                }
            });
        }
    });
    addr
}

/// Accepts password auth (`tester`/`s3cr3t`) and public-key auth for the fixed
/// client key, and bridges each `direct-tcpip` channel to the requested target.
struct ServerHandler;

impl Handler for ServerHandler {
    type Error = russh::Error;

    async fn auth_password(&mut self, user: &str, password: &str) -> Result<Auth, Self::Error> {
        if user == SSH_USER && password == SSH_PASSWORD {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::reject())
        }
    }

    async fn auth_publickey(&mut self, user: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        let authorized = PublicKey::from_openssh(CLIENT_PUB).unwrap();
        if user == SSH_USER && key.key_data() == authorized.key_data() {
            Ok(Auth::Accept)
        } else {
            Ok(Auth::reject())
        }
    }

    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        _session: &mut Session,
    ) -> Result<bool, Self::Error> {
        let addr = format!("{host_to_connect}:{port_to_connect}");
        tokio::spawn(async move {
            if let Ok(mut upstream) = TcpStream::connect(addr).await {
                let mut stream = channel.into_stream();
                let _ = tokio::io::copy_bidirectional(&mut stream, &mut upstream).await;
            }
        });
        Ok(true)
    }

    async fn data(&mut self, _channel: ChannelId, _data: &[u8], _session: &mut Session) -> Result<(), Self::Error> {
        Ok(())
    }
}

async fn spawn_ssh_server() -> SocketAddr {
    let server_key = decode_secret_key(SERVER_KEY, None).unwrap();
    let config = Arc::new(Config {
        keys: vec![server_key],
        ..Config::default()
    });
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let config = config.clone();
            tokio::spawn(async move {
                if let Ok(session) = run_stream(config, stream, ServerHandler).await {
                    let _ = session.await;
                }
            });
        }
    });
    addr
}

async fn socks5_connect(proxy: SocketAddr, target: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);

    let ip = match target.ip() {
        std::net::IpAddr::V4(v4) => v4.octets(),
        std::net::IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&ip);
    request.extend_from_slice(&target.port().to_be_bytes());
    stream.write_all(&request).await.unwrap();

    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], 0x05);
    assert_eq!(reply[1], 0x00, "SOCKS5 reply should be success");
    stream
}

fn ssh_mode(server: SocketAddr, auth: SshAuth, host_keys: Vec<String>) -> OutboundMode {
    OutboundMode::Ssh(Box::new(SshOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        username: SSH_USER.to_string(),
        auth,
        host_keys,
        host_key_algorithms: Vec::new(),
    }))
}

async fn relay_roundtrip(outbound: OutboundMode, echo: SocketAddr, payload: &[u8]) {
    let edge = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let mut conn = socks5_connect(edge.local_addr(), echo).await;
    conn.write_all(payload).await.unwrap();
    let mut buf = vec![0u8; payload.len()];
    conn.read_exact(&mut buf).await.unwrap();
    assert_eq!(buf, payload);

    edge.shutdown().await;
}

#[tokio::test]
async fn relays_through_ssh_with_password() {
    let echo = spawn_echo_server().await;
    let server = spawn_ssh_server().await;
    relay_roundtrip(
        ssh_mode(
            server,
            SshAuth::Password(SSH_PASSWORD.to_string()),
            vec![SERVER_PUB.to_string()],
        ),
        echo,
        b"hello ssh tunnel",
    )
    .await;
}

#[tokio::test]
async fn relays_through_ssh_with_private_key() {
    let echo = spawn_echo_server().await;
    let server = spawn_ssh_server().await;
    relay_roundtrip(
        ssh_mode(
            server,
            SshAuth::PrivateKey {
                pem: CLIENT_KEY.to_string(),
                passphrase: None,
            },
            // No host-key pinning here: accept whatever the server presents.
            Vec::new(),
        ),
        echo,
        b"public key auth",
    )
    .await;
}

#[tokio::test]
async fn rejects_wrong_host_key() {
    let echo = spawn_echo_server().await;
    let server = spawn_ssh_server().await;

    // Pin a key the server does not present, so host-key verification fails and
    // the outbound never opens; the inbound answers with a SOCKS5 failure.
    let edge = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound: ssh_mode(
            server,
            SshAuth::Password(SSH_PASSWORD.to_string()),
            vec![CLIENT_PUB.to_string()],
        ),
    })
    .await
    .unwrap();

    let mut stream = TcpStream::connect(edge.local_addr()).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);

    let ip = match echo.ip() {
        std::net::IpAddr::V4(v4) => v4.octets(),
        std::net::IpAddr::V6(_) => panic!("test uses IPv4"),
    };
    let mut request = vec![0x05, 0x01, 0x00, 0x01];
    request.extend_from_slice(&ip);
    request.extend_from_slice(&echo.port().to_be_bytes());
    stream.write_all(&request).await.unwrap();

    let mut reply = [0u8; 10];
    stream.read_exact(&mut reply).await.unwrap();
    assert_eq!(reply[0], 0x05);
    assert_ne!(reply[1], 0x00, "pinned host-key mismatch must not report success");

    edge.shutdown().await;
}
