//! Data-channel key renegotiation (`reneg-sec` / `P_CONTROL_SOFT_RESET_V1`).
//!
//! OpenVPN rotates the data-channel keys periodically without tearing the
//! tunnel down: a soft-reset exchange opens a new key state (key id 1..=7,
//! wrapping back to 1) on the same session, a fresh TLS handshake is tunnelled
//! over `P_CONTROL_V1` packets carrying the new key id, and a new key-method-2
//! exchange derives the replacement AEAD keys. The old key keeps decrypting
//! in-flight packets while the peer transitions (upstream's `reneg-sec`
//! defaults to 3600s, and its transition window keeps both keys alive).
//!
//! This service owns the control channel after the initial handshake: it
//! drains + acks incidental control traffic, initiates a renegotiation when
//! the `reneg-sec` timer fires, and answers a server-initiated soft reset. A
//! failed or timed-out renegotiation stops the service; the tunnel keeps
//! running on the old keys until the peer drops it.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use rustls::pki_types::ServerName;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio_rustls::TlsConnector;

use super::control::{ControlChannel, ControlTlsIo};
use super::data::DataChannel;
use super::device::read_server_key_method;
use super::keymethod::{ClientKeyMethod2, derive_client_key_material};
use super::packet::{ControlPacket, P_CONTROL_SOFT_RESET_V1, P_CONTROL_V1};
use super::tlswrap::AuthDigest;

/// Retransmission cadence for unacked reliable control packets on UDP.
const RETRANSMIT: Duration = Duration::from_secs(1);

/// How long one full renegotiation (soft reset + TLS + key method) may take
/// before the service gives up (upstream's `hand-window` default is 60s).
const REKEY_TIMEOUT: Duration = Duration::from_secs(60);

/// Everything the renegotiation service needs to run fresh key exchanges.
pub(super) struct RekeyContext {
    pub(super) control: Arc<ControlChannel>,
    pub(super) connector: TlsConnector,
    pub(super) server_name: ServerName<'static>,
    pub(super) options: String,
    pub(super) peer_info: String,
    pub(super) username: String,
    pub(super) password: String,
    pub(super) cipher: String,
    pub(super) auth: AuthDigest,
    pub(super) key_len: usize,
    pub(super) peer_id: u32,
    pub(super) udp: bool,
    /// `reneg-sec` interval; `None` disables client-initiated renegotiation.
    pub(super) interval: Option<Duration>,
    /// Freshly negotiated data channels handed to the device loop.
    pub(super) rekey_tx: mpsc::UnboundedSender<DataChannel>,
}

enum Trigger {
    /// The `reneg-sec` timer fired: we initiate.
    Timer,
    /// The server opened a new key state with this soft reset.
    Server(ControlPacket),
}

pub(super) async fn run(ctx: RekeyContext) {
    let mut key_id: u8 = 0;
    loop {
        let Some(trigger) = wait_trigger(&ctx).await else {
            return;
        };
        key_id = match &trigger {
            Trigger::Timer => next_key_id(key_id),
            Trigger::Server(packet) => packet.key_id,
        };
        let renegotiated = tokio::time::timeout(REKEY_TIMEOUT, async {
            match trigger {
                Trigger::Timer => client_initiate(&ctx, key_id).await?,
                Trigger::Server(packet) => server_initiate(&ctx, &packet).await?,
            }
            negotiate(&ctx, key_id).await
        })
        .await;
        match renegotiated {
            Ok(Ok(data)) => {
                if ctx.rekey_tx.send(data).is_err() {
                    return; // device loop is gone
                }
            }
            _ => return,
        }
    }
}

/// Drain (and ack) control traffic during the data phase until either the
/// renegotiation timer fires or the server opens a new key state.
async fn wait_trigger(ctx: &RekeyContext) -> Option<Trigger> {
    let deadline = ctx.interval.map(|i| tokio::time::Instant::now() + i);
    loop {
        if ctx.rekey_tx.is_closed() {
            return None;
        }
        let packet = match deadline {
            Some(deadline) => match tokio::time::timeout_at(deadline, ctx.control.read()).await {
                Err(_) => return Some(Trigger::Timer),
                Ok(Ok(packet)) => packet,
                Ok(Err(_)) => return None,
            },
            None => match ctx.control.read().await {
                Ok(packet) => packet,
                Err(_) => return None,
            },
        };
        if ctx.control.send_ack().await.is_err() {
            return None;
        }
        if packet.opcode == P_CONTROL_SOFT_RESET_V1 {
            return Some(Trigger::Server(packet));
        }
    }
}

/// Open a new key state: send our soft reset and wait for the server to open
/// its side with a matching soft reset before starting TLS.
async fn client_initiate(ctx: &RekeyContext, key_id: u8) -> Result<()> {
    ctx.control.begin_rekey(key_id);
    ctx.control
        .send_soft_reset()
        .await
        .context("openvpn: send soft reset")?;
    let mut tick = tokio::time::interval(RETRANSMIT);
    tick.tick().await; // the first tick fires immediately
    loop {
        tokio::select! {
            packet = ctx.control.read() => {
                let packet = packet?;
                ctx.control.send_ack().await?;
                if packet.opcode == P_CONTROL_SOFT_RESET_V1 {
                    return Ok(());
                }
            }
            _ = tick.tick(), if ctx.udp => {
                ctx.control.retransmit_pending().await?;
            }
        }
    }
}

/// Answer a server-initiated soft reset by opening our side of its key state.
async fn server_initiate(ctx: &RekeyContext, packet: &ControlPacket) -> Result<()> {
    ctx.control.begin_rekey(packet.key_id);
    ctx.control.note_remote_message(packet.message_id);
    ctx.control
        .send_soft_reset()
        .await
        .context("openvpn: answer soft reset")
}

/// Run the TLS + key-method-2 exchange for the new key state, pumping control
/// packets (and UDP retransmissions) alongside it, and build the replacement
/// data channel.
async fn negotiate(ctx: &RekeyContext, key_id: u8) -> Result<DataChannel> {
    let (in_tx, in_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let io = ControlTlsIo::new(in_rx, out_tx);

    let outbound = ctx.control.clone();
    tokio::spawn(async move {
        while let Some(bytes) = out_rx.recv().await {
            if outbound.send(P_CONTROL_V1, &bytes).await.is_err() {
                break;
            }
        }
    });

    let handshake = exchange_keys(ctx, io, key_id);
    tokio::pin!(handshake);
    let mut tick = tokio::time::interval(RETRANSMIT);
    tick.tick().await;
    loop {
        tokio::select! {
            done = &mut handshake => return done,
            packet = ctx.control.read() => {
                let packet = packet?;
                ctx.control.send_ack().await?;
                if packet.opcode == P_CONTROL_V1 && !packet.payload.is_empty() {
                    let _ = in_tx.send(packet.payload);
                }
            }
            _ = tick.tick(), if ctx.udp => {
                ctx.control.retransmit_pending().await?;
            }
        }
    }
}

async fn exchange_keys(ctx: &RekeyContext, io: ControlTlsIo, key_id: u8) -> Result<DataChannel> {
    let mut stream = ctx
        .connector
        .connect(ctx.server_name.clone(), io)
        .await
        .map_err(|e| anyhow!("openvpn: rekey TLS handshake: {e}"))?;
    let client_record = ClientKeyMethod2::new(
        ctx.options.clone(),
        ctx.peer_info.clone(),
        ctx.username.clone(),
        ctx.password.clone(),
    )?;
    stream
        .write_all(&client_record.marshal())
        .await
        .context("openvpn: write rekey key method")?;
    stream.flush().await.ok();
    let (server_record, _leftover) = read_server_key_method(&mut stream).await?;
    let keys = derive_client_key_material(
        &client_record.source,
        &server_record.source,
        ctx.control.local_session(),
        ctx.control.remote_session(),
        ctx.key_len,
    )?;
    DataChannel::new(&keys, &ctx.cipher, ctx.auth, ctx.peer_id, key_id)
}

/// Next data-channel key id: 1..=7, wrapping past 0 (0 is only ever the
/// initial key state).
fn next_key_id(key_id: u8) -> u8 {
    if key_id >= 7 { 1 } else { key_id + 1 }
}

#[cfg(test)]
mod tests {
    use super::next_key_id;

    #[test]
    fn key_id_rotation_skips_zero() {
        assert_eq!(next_key_id(0), 1);
        assert_eq!(next_key_id(1), 2);
        assert_eq!(next_key_id(7), 1);
    }
}
