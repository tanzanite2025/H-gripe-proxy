//! Per-config OpenVPN tunnel device: TCP/UDP connect, the hard-reset / TLS /
//! key-method-2 / push handshake, and the registry that lets concurrent
//! connections share one live tunnel.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::sync::{Notify, mpsc, oneshot};
use tokio_rustls::TlsConnector;

use super::OpenVpnOutboundConfig;
use super::control::{ControlChannel, PacketWriter, run_mux, run_mux_udp, spawn_tls_bridge};
use super::data::DataChannel;
use super::device_loop::{CHANNEL_DEPTH, DeviceLoop, Keepalive};
use super::keymethod::{
    ClientKeyMethod2, ServerKeyMethod2, derive_client_key_material, options_string, parse_server_key_method2, peer_info,
};
use super::packet::{P_CONTROL_HARD_RESET_SERVER_V2, new_session_id};
use super::push::{PUSH_REQUEST, PushReply, parse_push_reply};
use super::rekey::{self, RekeyContext};
use super::stream::{OvpnTcpStream, OvpnUdpAssoc};
use super::tls;
use super::tlswrap;

/// How long the handshake waits for a control-channel reply on UDP before
/// resending unacked reliable control packets (OpenVPN's default is ~1s).
const CONTROL_RETRANSMIT_DELAY: Duration = Duration::from_secs(1);

/// Registry key identifying a tunnel configuration (server endpoint + auth).
type OvpnKey = String;

/// Per-config registry of live tunnel devices so concurrent connections to the
/// same server share one handshake + netstack. A device whose command channel
/// has closed (its loop exited) is discarded and rebuilt on the next connect.
static DEVICE_REGISTRY: Mutex<Option<HashMap<OvpnKey, Arc<OpenVpnDevice>>>> = Mutex::new(None);

/// Command sent from a `connect` call into the device's poll loop.
pub(super) enum Command {
    OpenTcp {
        dst: SocketAddr,
        reply: oneshot::Sender<OvpnTcpStream>,
    },
    OpenUdp {
        dst: SocketAddr,
        reply: oneshot::Sender<OvpnUdpAssoc>,
    },
}

/// Handle to a running OpenVPN tunnel device: the command channel into its loop.
pub struct OpenVpnDevice {
    commands: mpsc::Sender<Command>,
    /// Whether the server pushed an `ifconfig` (IPv4) tunnel address.
    has_v4: bool,
    /// Whether the server pushed an `ifconfig-ipv6` tunnel address.
    has_v6: bool,
}

impl OpenVpnDevice {
    pub(super) async fn get_or_create(config: &OpenVpnOutboundConfig) -> Result<Arc<Self>> {
        let key = config.registry_key();
        {
            let mut guard = DEVICE_REGISTRY.lock().expect("openvpn device registry");
            let map = guard.get_or_insert_with(HashMap::new);
            if let Some(device) = map.get(&key) {
                if !device.commands.is_closed() {
                    return Ok(device.clone());
                }
                map.remove(&key);
            }
        }

        let device = Arc::new(Self::spawn(config).await?);
        let mut guard = DEVICE_REGISTRY.lock().expect("openvpn device registry");
        let map = guard.get_or_insert_with(HashMap::new);
        if let Some(existing) = map.get(&key) {
            if !existing.commands.is_closed() {
                return Ok(existing.clone());
            }
        }
        map.insert(key, device.clone());
        Ok(device)
    }

    /// Connect the tunnel transport (TCP or UDP) and run the full OpenVPN client
    /// handshake, then spawn the netstack poll loop bound to the pushed tunnel
    /// address.
    async fn spawn(config: &OpenVpnOutboundConfig) -> Result<Self> {
        let endpoint = tokio::net::lookup_host((config.server.as_str(), config.port))
            .await
            .with_context(|| format!("openvpn: resolve {}:{}", config.server, config.port))?
            .next()
            .ok_or_else(|| anyhow!("openvpn: no addresses for {}:{}", config.server, config.port))?;

        let (control_tx, control_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (data_tx, data_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let writer = if config.udp {
            let bind: SocketAddr = if endpoint.is_ipv4() {
                (std::net::Ipv4Addr::UNSPECIFIED, 0).into()
            } else {
                (std::net::Ipv6Addr::UNSPECIFIED, 0).into()
            };
            let socket = UdpSocket::bind(bind)
                .await
                .with_context(|| format!("openvpn: bind UDP for {endpoint}"))?;
            socket
                .connect(endpoint)
                .await
                .with_context(|| format!("openvpn: connect UDP to {endpoint}"))?;
            let socket = Arc::new(socket);
            tokio::spawn(run_mux_udp(socket.clone(), control_tx, data_tx));
            Arc::new(PacketWriter::udp(socket))
        } else {
            let tcp = TcpStream::connect(endpoint)
                .await
                .with_context(|| format!("openvpn: connect TCP to {endpoint}"))?;
            let _ = tcp.set_nodelay(true);
            let (read_half, write_half) = tcp.into_split();
            tokio::spawn(run_mux(read_half, control_tx, data_tx));
            Arc::new(PacketWriter::tcp(write_half))
        };

        let local_session = new_session_id()?;
        let control = Arc::new(ControlChannel::new(
            writer.clone(),
            local_session,
            control_rx,
            config.control_wrap()?,
        ));

        // On UDP the transport is lossy, so drive control retransmission on a
        // timer until the handshake completes (a no-op on the lossless TCP path).
        let handshake_done = Arc::new(AtomicBool::new(false));
        if config.udp {
            spawn_retransmit(control.clone(), handshake_done.clone());
        }

        // 1. Reliable hard-reset exchange (before the TLS bridge starts reading).
        control.send_reset().await.context("openvpn: send hard reset")?;
        wait_server_reset(&control).await?;

        // 2. TLS handshake tunnelled over P_CONTROL_V1 messages.
        let bridge_shutdown = Arc::new(Notify::new());
        let io = spawn_tls_bridge(control.clone(), handshake_done.clone(), bridge_shutdown.clone());
        let tls_config = tls::build_client_config(
            &config.ca_pem,
            config.client_cert_pem.as_deref(),
            config.client_key_pem.as_deref(),
        )?;
        let connector = TlsConnector::from(tls_config);
        let mut stream = connector
            .connect(tls::server_name(&config.server), io)
            .await
            .map_err(|e| anyhow!("openvpn: control-channel TLS handshake: {e}"))?;

        // 3. key-method-2 exchange + directional key derivation.
        let options = options_string(!config.udp, &config.cipher, &config.auth_digest);
        let info = peer_info(&config.cipher);
        let client_record = ClientKeyMethod2::new(
            options,
            info,
            config.username.clone().unwrap_or_default(),
            config.password.clone().unwrap_or_default(),
        )?;
        stream
            .write_all(&client_record.marshal())
            .await
            .context("openvpn: write client key method")?;
        stream.flush().await.ok();

        let (server_record, leftover) = read_server_key_method(&mut stream).await?;
        let keys = derive_client_key_material(
            &client_record.source,
            &server_record.source,
            control.local_session(),
            control.remote_session(),
            cipher_key_len(&config.cipher),
        )?;

        // 4. Push negotiation for the assigned address + data-channel peer id.
        stream
            .write_all(format!("{PUSH_REQUEST}\0").as_bytes())
            .await
            .context("openvpn: write push request")?;
        stream.flush().await.ok();
        let push = read_push_reply(&mut stream, leftover).await?;

        handshake_done.store(true, Ordering::Relaxed);
        drop(stream);
        // Stop the handshake bridge; the renegotiation service takes over
        // reading (and acking) the control channel.
        bridge_shutdown.notify_one();

        // 5. Bring up the data channel + netstack poll loop.
        let auth = tlswrap::AuthDigest::parse(&config.auth_digest)?;
        let data = DataChannel::new(&keys, &config.cipher, auth, push.peer_id, 0)?;
        let (commands_tx, commands_rx) = mpsc::channel::<Command>(CHANNEL_DEPTH);
        let keepalive = Keepalive {
            ping_interval: push.ping_interval,
            ping_restart: push.ping_restart,
        };
        let (rekey_tx, rekey_rx) = mpsc::unbounded_channel::<DataChannel>();
        let device_loop = DeviceLoop::new(
            data,
            writer,
            data_rx,
            commands_rx,
            config.mtu as usize,
            push.local_v4,
            push.local_v6,
            keepalive,
            rekey_rx,
        );
        tokio::spawn(device_loop.run());

        // 6. The renegotiation service: drains the control channel and rotates
        //    the data keys on the `reneg-sec` timer or a server soft reset.
        tokio::spawn(rekey::run(RekeyContext {
            control,
            connector,
            server_name: tls::server_name(&config.server),
            options: options_string(!config.udp, &config.cipher, &config.auth_digest),
            peer_info: peer_info(&config.cipher),
            username: config.username.clone().unwrap_or_default(),
            password: config.password.clone().unwrap_or_default(),
            cipher: config.cipher.clone(),
            auth,
            key_len: cipher_key_len(&config.cipher),
            peer_id: push.peer_id,
            udp: config.udp,
            interval: (config.reneg_sec > 0).then(|| Duration::from_secs(config.reneg_sec)),
            rekey_tx,
        }));

        Ok(Self {
            commands: commands_tx,
            has_v4: push.local_v4.is_some(),
            has_v6: push.local_v6.is_some(),
        })
    }

    /// Whether the tunnel can carry inner packets of `dst`'s address family
    /// (the server pushed a tunnel address for it).
    pub(super) fn supports(&self, dst: &SocketAddr) -> bool {
        if dst.is_ipv4() { self.has_v4 } else { self.has_v6 }
    }

    pub(super) async fn open_tcp(&self, dst: SocketAddr) -> Result<OvpnTcpStream> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::OpenTcp { dst, reply: reply_tx })
            .await
            .map_err(|_| anyhow!("openvpn: device loop is gone"))?;
        reply_rx
            .await
            .map_err(|_| anyhow!("openvpn: connection to {dst} failed (connect timeout)"))
    }

    pub(super) async fn open_udp(&self, dst: SocketAddr) -> Result<OvpnUdpAssoc> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.commands
            .send(Command::OpenUdp { dst, reply: reply_tx })
            .await
            .map_err(|_| anyhow!("openvpn: device loop is gone"))?;
        reply_rx
            .await
            .map_err(|_| anyhow!("openvpn: UDP association to {dst} failed"))
    }
}

/// Periodically resend unacked reliable control packets until `handshake_done`
/// is set. Used on UDP transports, where control packets can be lost.
fn spawn_retransmit(control: Arc<ControlChannel>, handshake_done: Arc<AtomicBool>) {
    tokio::spawn(async move {
        while !handshake_done.load(Ordering::Relaxed) {
            tokio::time::sleep(CONTROL_RETRANSMIT_DELAY).await;
            if handshake_done.load(Ordering::Relaxed) {
                break;
            }
            if control.retransmit_pending().await.is_err() {
                break;
            }
        }
    });
}

/// Read reliable control packets until the server's hard reset arrives, acking
/// each. Runs before the TLS bridge so the reset is delivered here, not dropped.
async fn wait_server_reset(control: &ControlChannel) -> Result<()> {
    loop {
        let packet = control.read().await?;
        control.send_ack().await?;
        if packet.opcode == P_CONTROL_HARD_RESET_SERVER_V2 {
            return Ok(());
        }
    }
}

/// Read the full server key-method-2 record from the TLS control stream,
/// returning it plus any bytes read past the record (which belong to the next
/// message).
pub(super) async fn read_server_key_method<S>(stream: &mut S) -> Result<(ServerKeyMethod2, Vec<u8>)>
where
    S: AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if let Some((record, consumed)) = parse_server_key_method2(&buf)? {
            let leftover = buf.split_off(consumed);
            return Ok((record, leftover));
        }
        let n = stream
            .read(&mut chunk)
            .await
            .context("openvpn: read server key method")?;
        if n == 0 {
            bail!("openvpn: control channel closed during key method exchange");
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > 8192 {
            bail!("openvpn: server key method record too large");
        }
    }
}

/// Read a NUL-terminated push message from the TLS control stream and parse it.
async fn read_push_reply<S>(stream: &mut S, mut buf: Vec<u8>) -> Result<PushReply>
where
    S: AsyncRead + Unpin,
{
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(pos) = buf.iter().position(|&b| b == 0) {
            let message = String::from_utf8_lossy(&buf[..pos]).into_owned();
            return parse_push_reply(&message);
        }
        let n = stream.read(&mut chunk).await.context("openvpn: read push reply")?;
        if n == 0 {
            bail!("openvpn: control channel closed before push reply");
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > 65536 {
            bail!("openvpn: push reply too large");
        }
    }
}

/// Data-cipher key length in bytes for a normalized cipher name.
fn cipher_key_len(cipher: &str) -> usize {
    match cipher {
        "AES-128-GCM" | "AES-128-CBC" => 16,
        "AES-192-CBC" => 24,
        _ => 32,
    }
}
