//! Snell UDP-over-TCP: [`SnellUdp`] (v3, shadowaead chunk per datagram),
//! [`SnellV4Udp`] (v4/v5, one v4 frame per datagram) and the version-agnostic
//! [`SnellUdpAssoc`] wrapper the UDP egress loop uses.

use std::net::SocketAddr;

use anyhow::{Context, Result, anyhow, bail};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::sync::Mutex;

use crate::address::TargetAddr;
use crate::outbound::BoxedStream;

use super::crypto::{AeadCipher, SnellCipher, increment_nonce, random_bytes, snell_kdf};
use super::v4::{V4_INITIAL_PADDING_MIN, V4_INITIAL_PADDING_SPAN, build_v4_frame, read_v4_frame};
use super::{
    COMMAND_UDP, MAX_CHUNK, RESP_ERROR, RESP_TUNNEL, SALT_LEN, SNELL_PROTO_BYTE, SnellOutboundConfig, TAG_LEN,
    UDP_ADDR_IPV4, UDP_ADDR_IPV6, UDP_FORWARD, connect_transport,
};

/// A Snell UDP-over-TCP association (one per destination, mirroring the other
/// UDP egresses' `connect` / `send` / `recv` shape). It runs over the same
/// shadowaead chunk stream as TCP, but the handshake uses `CommandUDP` and each
/// datagram is a single AEAD chunk so packet boundaries survive. The TCP stream
/// is split so `send` and `recv` can run concurrently in the egress `select!`;
/// each half guards its own cipher + counter nonce behind a mutex.
pub struct SnellUdp {
    /// The fixed destination sealed into every packet sent on this association.
    target: TargetAddr,
    /// The Snell PSK, used to derive the read subkey from the server's salt.
    psk: Vec<u8>,
    /// The AEAD cipher family (v3 => AES-128-GCM).
    cipher: SnellCipher,
    write: Mutex<UdpWriteSide>,
    read: Mutex<UdpReadSide>,
}

struct UdpWriteSide {
    writer: WriteHalf<BoxedStream>,
    cipher: AeadCipher,
    nonce: [u8; 12],
}

struct UdpReadSide {
    reader: ReadHalf<BoxedStream>,
    /// Derived from the server's salt on the first `recv`; `None` until then.
    cipher: Option<AeadCipher>,
    nonce: [u8; 12],
    salt_done: bool,
}

impl SnellUdp {
    /// Open a Snell UDP association to `config.server` for datagrams destined to
    /// `target`. Sends the client salt and the `CommandUDP` handshake header
    /// (one AEAD chunk) before returning. Requires protocol v3.
    pub async fn connect(config: &SnellOutboundConfig, target: &TargetAddr) -> Result<Self> {
        if config.version != 3 {
            bail!("snell udp (shadowaead): requires version 3 (got v{})", config.version);
        }
        let cipher = config.cipher();
        let transport = connect_transport(config).await?;
        let (reader, mut writer) = tokio::io::split(transport);

        let mut salt = [0u8; SALT_LEN];
        random_bytes(&mut salt);
        writer.write_all(&salt).await.context("snell udp: send salt")?;

        let subkey = snell_kdf(&config.psk, &salt, cipher.key_size());
        let write_cipher = AeadCipher::new(cipher, &subkey)?;
        let mut write_nonce = [0u8; 12];

        // UDP handshake header: `proto(1) | CommandUDP | clientID-len(0)`; no
        // host/port (every datagram carries its own address).
        let header = [SNELL_PROTO_BYTE, COMMAND_UDP, 0];
        write_packet_chunk(&mut writer, &write_cipher, &mut write_nonce, &header)
            .await
            .context("snell udp: send handshake header")?;

        Ok(Self {
            target: target.clone(),
            psk: config.psk.clone(),
            cipher,
            write: Mutex::new(UdpWriteSide {
                writer,
                cipher: write_cipher,
                nonce: write_nonce,
            }),
            read: Mutex::new(UdpReadSide {
                reader,
                cipher: None,
                nonce: [0u8; 12],
                salt_done: false,
            }),
        })
    }

    /// Seal `payload` as one datagram (`UDPForward | addr | payload`) and send it
    /// to the server as a single AEAD chunk.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let mut plain = Vec::with_capacity(1 + 1 + 16 + 2 + payload.len());
        plain.push(UDP_FORWARD);
        encode_udp_addr(&mut plain, &self.target)?;
        plain.extend_from_slice(payload);
        if plain.len() > MAX_CHUNK {
            bail!(
                "snell udp: packet too large for one chunk ({} > {MAX_CHUNK})",
                plain.len()
            );
        }
        let mut w = self.write.lock().await;
        let UdpWriteSide { writer, cipher, nonce } = &mut *w;
        write_packet_chunk(writer, cipher, nonce, &plain).await
    }

    /// Receive one reply datagram (one AEAD chunk), strip the server's source
    /// address, and return the application payload.
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut r = self.read.lock().await;
        if !r.salt_done {
            let mut salt = [0u8; SALT_LEN];
            r.reader.read_exact(&mut salt).await.context("snell udp: read salt")?;
            let subkey = snell_kdf(&self.psk, &salt, self.cipher.key_size());
            r.cipher = Some(AeadCipher::new(self.cipher, &subkey)?);
            r.salt_done = true;
        }
        let UdpReadSide {
            reader, cipher, nonce, ..
        } = &mut *r;
        let cipher = cipher.as_ref().ok_or_else(|| anyhow!("snell udp: read cipher unset"))?;
        let plain = read_packet_chunk(reader, cipher, nonce).await?;
        decode_udp_reply(&plain)
    }
}

/// A Snell **v4/v5** UDP-over-TCP association. The datagram framing (handshake
/// `CommandUDP` header, per-packet `UDPForward | addr | payload`, reply `addr |
/// payload`) is identical to v3; only the transport differs: each datagram is
/// carried as one v4 frame (`AEAD(header) | [padding] | AEAD(payload)`) instead
/// of a shadowaead chunk, the salt + initial padding ride the handshake frame,
/// and — unlike v3 — v4 sends a one-byte command response that is consumed
/// before the first reply datagram (mirroring upstream's `ReadReply`).
pub struct SnellV4Udp {
    /// The fixed destination sealed into every packet sent on this association.
    target: TargetAddr,
    write: Mutex<UdpV4WriteSide>,
    read: Mutex<UdpV4ReadSide>,
}

struct UdpV4WriteSide {
    writer: WriteHalf<BoxedStream>,
    cipher: AeadCipher,
    nonce: [u8; 12],
}

struct UdpV4ReadSide {
    reader: ReadHalf<BoxedStream>,
    psk: Vec<u8>,
    /// Derived from the server's salt on the first `recv`; `None` until then.
    cipher: Option<AeadCipher>,
    nonce: [u8; 12],
    salt_done: bool,
    /// Whether the v4 command-response byte has been consumed.
    reply_done: bool,
    /// A datagram that shared the reply frame (if the server coalesced the
    /// command byte with its first response), surfaced by the next `recv`.
    pending: Option<Vec<u8>>,
}

impl SnellV4Udp {
    /// Open a v4/v5 UDP association to `config.server` for datagrams destined to
    /// `target`. Sends the `CommandUDP` handshake header as the first v4 frame
    /// (carrying the client salt + initial padding). Requires v4/v5.
    pub async fn connect(config: &SnellOutboundConfig, target: &TargetAddr) -> Result<Self> {
        if !config.uses_v4_framing() {
            bail!("snell v4 udp: requires version >= 4 (got v{})", config.version);
        }
        let transport = connect_transport(config).await?;
        let (reader, mut writer) = tokio::io::split(transport);

        // v4 is always AES-128-GCM.
        let mut salt = [0u8; SALT_LEN];
        random_bytes(&mut salt);
        let subkey = snell_kdf(&config.psk, &salt, SnellCipher::Aes128Gcm.key_size());
        let write_cipher = AeadCipher::new(SnellCipher::Aes128Gcm, &subkey)?;
        let mut write_nonce = [0u8; 12];

        let mut delta = [0u8; 2];
        random_bytes(&mut delta);
        let initial_padding = V4_INITIAL_PADDING_MIN + (u16::from_le_bytes(delta) as usize) % V4_INITIAL_PADDING_SPAN;

        // UDP handshake header `proto | CommandUDP | clientID-len(0)` rides the
        // first v4 frame, which prepends the salt + initial padding.
        let header = [SNELL_PROTO_BYTE, COMMAND_UDP, 0];
        let frame = build_v4_frame(&write_cipher, &mut write_nonce, &header, initial_padding, Some(&salt))?;
        writer
            .write_all(&frame)
            .await
            .context("snell v4 udp: send handshake header")?;
        writer.flush().await.context("snell v4 udp: flush handshake header")?;

        Ok(Self {
            target: target.clone(),
            write: Mutex::new(UdpV4WriteSide {
                writer,
                cipher: write_cipher,
                nonce: write_nonce,
            }),
            read: Mutex::new(UdpV4ReadSide {
                reader,
                psk: config.psk.clone(),
                cipher: None,
                nonce: [0u8; 12],
                salt_done: false,
                reply_done: false,
                pending: None,
            }),
        })
    }

    /// Seal `payload` as one datagram (`UDPForward | addr | payload`) and send it
    /// as a single v4 frame (no padding: the salt already rode the handshake).
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let mut plain = Vec::with_capacity(1 + 1 + 16 + 2 + payload.len());
        plain.push(UDP_FORWARD);
        encode_udp_addr(&mut plain, &self.target)?;
        plain.extend_from_slice(payload);
        if plain.len() > MAX_CHUNK {
            bail!(
                "snell v4 udp: packet too large for one frame ({} > {MAX_CHUNK})",
                plain.len()
            );
        }
        let mut w = self.write.lock().await;
        let UdpV4WriteSide { writer, cipher, nonce } = &mut *w;
        let frame = build_v4_frame(cipher, nonce, &plain, 0, None)?;
        writer.write_all(&frame).await.context("snell v4 udp: write datagram")?;
        writer.flush().await.context("snell v4 udp: flush datagram")?;
        Ok(())
    }

    /// Receive one reply datagram (one v4 frame), stripping the server's source
    /// address. Lazily reads the server salt and the one-byte command response
    /// on the first call.
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut r = self.read.lock().await;
        if !r.salt_done {
            let mut salt = [0u8; SALT_LEN];
            r.reader
                .read_exact(&mut salt)
                .await
                .context("snell v4 udp: read salt")?;
            let subkey = snell_kdf(&r.psk, &salt, SnellCipher::Aes128Gcm.key_size());
            r.cipher = Some(AeadCipher::new(SnellCipher::Aes128Gcm, &subkey)?);
            r.salt_done = true;
        }

        if !r.reply_done {
            let frame = {
                let UdpV4ReadSide {
                    reader, cipher, nonce, ..
                } = &mut *r;
                let cipher = cipher
                    .as_ref()
                    .ok_or_else(|| anyhow!("snell v4 udp: read cipher unset"))?;
                read_v4_frame(reader, cipher, nonce).await?
            };
            match frame.first().copied() {
                Some(RESP_TUNNEL) => {}
                Some(RESP_ERROR) => bail!("snell v4 udp: server reported error"),
                Some(other) => bail!("snell v4 udp: unexpected command response {other}"),
                None => bail!("snell v4 udp: empty reply frame"),
            }
            if frame.len() > 1 {
                r.pending = Some(frame[1..].to_vec());
            }
            r.reply_done = true;
        }

        if let Some(pending) = r.pending.take() {
            return decode_udp_reply(&pending);
        }

        let frame = {
            let UdpV4ReadSide {
                reader, cipher, nonce, ..
            } = &mut *r;
            let cipher = cipher
                .as_ref()
                .ok_or_else(|| anyhow!("snell v4 udp: read cipher unset"))?;
            read_v4_frame(reader, cipher, nonce).await?
        };
        if frame.is_empty() {
            bail!("snell v4 udp: server closed the association");
        }
        decode_udp_reply(&frame)
    }
}

/// A Snell UDP association over either the v3 shadowaead stream or the v4/v5
/// frame stream, so the UDP egress loop can stay version-agnostic.
pub enum SnellUdpAssoc {
    V3(SnellUdp),
    V4(SnellV4Udp),
}

impl SnellUdpAssoc {
    /// Open the association, dispatching on the protocol version: v4/v5 use the
    /// v4 frame stream, v3 uses the shadowaead chunk stream.
    pub async fn connect(config: &SnellOutboundConfig, target: &TargetAddr) -> Result<Self> {
        if config.uses_v4_framing() {
            Ok(Self::V4(SnellV4Udp::connect(config, target).await?))
        } else {
            Ok(Self::V3(SnellUdp::connect(config, target).await?))
        }
    }

    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        match self {
            Self::V3(assoc) => assoc.send(payload).await,
            Self::V4(assoc) => assoc.send(payload).await,
        }
    }

    pub async fn recv(&self) -> Result<Vec<u8>> {
        match self {
            Self::V3(assoc) => assoc.recv().await,
            Self::V4(assoc) => assoc.recv().await,
        }
    }
}

/// Seal `plaintext` as one length-prefixed AEAD chunk and write it (with a
/// flush) to `writer`, advancing the counter nonce twice.
async fn write_packet_chunk<W: AsyncWrite + Unpin>(
    writer: &mut W,
    cipher: &AeadCipher,
    nonce: &mut [u8; 12],
    plaintext: &[u8],
) -> Result<()> {
    let len = u16::try_from(plaintext.len()).map_err(|_| anyhow!("snell udp: chunk too large"))?;
    let sealed_len = cipher.seal(nonce, &len.to_be_bytes())?;
    increment_nonce(nonce);
    let sealed_payload = cipher.seal(nonce, plaintext)?;
    increment_nonce(nonce);

    let mut out = Vec::with_capacity(sealed_len.len() + sealed_payload.len());
    out.extend_from_slice(&sealed_len);
    out.extend_from_slice(&sealed_payload);
    writer.write_all(&out).await.context("snell udp: write chunk")?;
    writer.flush().await.context("snell udp: flush chunk")?;
    Ok(())
}

/// Read exactly one length-prefixed AEAD chunk and return its plaintext,
/// advancing the counter nonce twice.
async fn read_packet_chunk<R: AsyncRead + Unpin>(
    reader: &mut R,
    cipher: &AeadCipher,
    nonce: &mut [u8; 12],
) -> Result<Vec<u8>> {
    let mut sealed_len = [0u8; 2 + TAG_LEN];
    reader
        .read_exact(&mut sealed_len)
        .await
        .context("snell udp: read chunk length")?;
    let len_plain = cipher.open(nonce, &sealed_len)?;
    increment_nonce(nonce);
    let clen = u16::from_be_bytes([len_plain[0], len_plain[1]]) as usize;
    if clen == 0 || clen > MAX_CHUNK {
        bail!("snell udp: invalid chunk length {clen}");
    }
    let mut sealed = vec![0u8; clen + TAG_LEN];
    reader
        .read_exact(&mut sealed)
        .await
        .context("snell udp: read chunk payload")?;
    let plain = cipher.open(nonce, &sealed)?;
    increment_nonce(nonce);
    Ok(plain)
}

/// Encode a Snell UDP destination address into `buf`. The wire form differs from
/// SOCKS5: a domain is `len(1) | host | port(2 BE)`; an IP is `0x00 | family |
/// addr | port(2 BE)` where `family` is `4` (IPv4) or `6` (IPv6).
pub(super) fn encode_udp_addr(buf: &mut Vec<u8>, target: &TargetAddr) -> Result<()> {
    match target {
        TargetAddr::Domain(host, port) => {
            let host_len = u8::try_from(host.len()).map_err(|_| anyhow!("snell udp: host longer than 255 bytes"))?;
            buf.push(host_len);
            buf.extend_from_slice(host.as_bytes());
            buf.extend_from_slice(&port.to_be_bytes());
        }
        TargetAddr::Ip(SocketAddr::V4(addr)) => {
            buf.push(0);
            buf.push(UDP_ADDR_IPV4);
            buf.extend_from_slice(&addr.ip().octets());
            buf.extend_from_slice(&addr.port().to_be_bytes());
        }
        TargetAddr::Ip(SocketAddr::V6(addr)) => {
            buf.push(0);
            buf.push(UDP_ADDR_IPV6);
            buf.extend_from_slice(&addr.ip().octets());
            buf.extend_from_slice(&addr.port().to_be_bytes());
        }
    }
    Ok(())
}

/// Strip the server's source address from a reply chunk and return the payload.
/// Replies are `type | addr | payload` with `type` = `4` (IPv4) or `6` (IPv6).
pub(super) fn decode_udp_reply(plain: &[u8]) -> Result<Vec<u8>> {
    let kind = *plain.first().ok_or_else(|| anyhow!("snell udp: empty reply"))?;
    let payload_off = match kind {
        UDP_ADDR_IPV4 => 1 + 4 + 2,
        UDP_ADDR_IPV6 => 1 + 16 + 2,
        other => bail!("snell udp: unexpected reply address type {other}"),
    };
    if plain.len() < payload_off {
        bail!("snell udp: reply truncated");
    }
    Ok(plain[payload_off..].to_vec())
}
