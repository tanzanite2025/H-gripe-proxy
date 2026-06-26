//! Shadowsocks outbound (AEAD ciphers).
//!
//! Implements the modern Shadowsocks **AEAD** stream (the `aes-128-gcm`,
//! `aes-256-gcm` and `chacha20-ietf-poly1305` methods) over plain TCP. The
//! legacy stream ciphers (`aes-*-cfb`, `rc4-md5`, …) and the newer
//! `2022-blake3-*` methods use different constructions and are rejected by
//! [`ShadowsocksOutboundConfig::from_proxy`] rather than silently mis-encoded,
//! as are SIP003 `plugin`s (obfs / v2ray-plugin), which would need their own
//! transport layer.
//!
//! All cryptographic primitives are delegated to vetted RustCrypto crates
//! (`aes-gcm`, `chacha20poly1305`, `md-5`, `sha1`); the only things assembled
//! here are the Shadowsocks key schedule (OpenSSL `EVP_BytesToKey` for the
//! master key, HKDF-SHA1 for the per-session subkey) and the on-wire framing.
//!
//! Wire format (per direction, RFC-less Shadowsocks AEAD spec):
//! ```text
//! salt (key_len bytes, in clear) | chunk | chunk | ...
//! chunk = AEAD(len)(2+16) | AEAD(payload)(len+16)
//! ```
//! `salt` seeds `subkey = HKDF-SHA1(master_key, salt, "ss-subkey")`. Each AEAD
//! operation uses a 12-byte little-endian counter nonce that starts at 0 and
//! increments after every seal/open. The first plaintext bytes the client sends
//! are the SOCKS5-format target address; application data follows.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use aes_gcm::aead::generic_array::GenericArray;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use anyhow::{Context, Result, anyhow, bail};
use chacha20poly1305::ChaCha20Poly1305;
use md5::Md5;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::net::{TcpStream, UdpSocket, lookup_host};

use crate::address::TargetAddr;
use crate::inbound::socks5;
use crate::outbound::BoxedStream;
use crate::proxy::ProxyEntry;

/// HKDF `info` string that derives the per-session subkey from the master key.
const SS_SUBKEY_INFO: &[u8] = b"ss-subkey";
/// Largest plaintext carried in a single AEAD chunk (Shadowsocks caps the
/// length field at 0x3FFF so it never collides with the high bit).
const MAX_CHUNK: usize = 0x3fff;
/// The AEAD tag length for every supported cipher.
const TAG_LEN: usize = 16;
/// Upper bound on a received Shadowsocks UDP packet (salt + sealed payload).
const MAX_UDP_PACKET: usize = 64 * 1024;

/// Shadowsocks AEAD method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShadowsocksCipher {
    Aes128Gcm,
    Aes256Gcm,
    Chacha20IetfPoly1305,
}

impl ShadowsocksCipher {
    /// Key length in bytes; the salt length equals the key length for every
    /// supported AEAD method.
    fn key_size(self) -> usize {
        match self {
            ShadowsocksCipher::Aes128Gcm => 16,
            ShadowsocksCipher::Aes256Gcm | ShadowsocksCipher::Chacha20IetfPoly1305 => 32,
        }
    }
}

/// Fully-resolved Shadowsocks outbound parameters. The password is pre-expanded
/// into the master `key` via `EVP_BytesToKey`, so the dial path never touches
/// the raw secret again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShadowsocksOutboundConfig {
    pub server: String,
    pub port: u16,
    pub cipher: ShadowsocksCipher,
    pub key: Vec<u8>,
}

impl ShadowsocksOutboundConfig {
    /// Build an outbound config from a parsed `ss` proxy entry, rejecting
    /// methods and features that are not implemented yet so traffic is never
    /// mis-framed.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("shadowsocks: missing server")?;
        let port = opts.port.context("shadowsocks: missing port")?;
        let password = opts
            .password
            .as_deref()
            .filter(|s| !s.is_empty())
            .context("shadowsocks: missing password")?;

        let cipher = match opts.cipher.as_deref() {
            Some("aes-128-gcm") => ShadowsocksCipher::Aes128Gcm,
            Some("aes-256-gcm") => ShadowsocksCipher::Aes256Gcm,
            Some("chacha20-ietf-poly1305") | Some("chacha20-poly1305") => ShadowsocksCipher::Chacha20IetfPoly1305,
            None | Some("") => bail!("shadowsocks: missing cipher"),
            Some(other) => bail!(
                "shadowsocks: cipher {other:?} not supported \
                 (use aes-128-gcm / aes-256-gcm / chacha20-ietf-poly1305)"
            ),
        };

        // SIP003 plugins (obfs / v2ray-plugin) wrap the stream in another
        // transport that is not implemented; reject rather than dial blindly.
        if let Some(plugin) = opts.plugin.as_deref().filter(|s| !s.is_empty()) {
            bail!("shadowsocks: plugin {plugin:?} not supported");
        }

        let key = evp_bytes_to_key(password.as_bytes(), cipher.key_size());

        Ok(Self {
            server,
            port,
            cipher,
            key,
        })
    }
}

/// Connect a Shadowsocks outbound to `target` and return a relay-ready stream.
/// The salt and the AEAD-sealed target address are sent before the stream is
/// handed back, so reads/writes thereafter carry application data only.
pub async fn connect(config: &ShadowsocksOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let mut tcp = TcpStream::connect((config.server.as_str(), config.port))
        .await
        .with_context(|| format!("shadowsocks: connect {}:{}", config.server, config.port))?;

    let salt_len = config.cipher.key_size();
    let mut salt = vec![0u8; salt_len];
    random_bytes(&mut salt);
    tcp.write_all(&salt).await.context("shadowsocks: send salt")?;

    let subkey = hkdf_sha1(&config.key, &salt, SS_SUBKEY_INFO, salt_len);
    let write_cipher = AeadCipher::new(config.cipher, &subkey)?;

    let mut stream = ShadowsocksStream::new(tcp, config.cipher, config.key.clone(), write_cipher);

    // The first plaintext bytes are the SOCKS5-format destination address.
    let mut addr = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr, target);
    stream
        .write_all(&addr)
        .await
        .context("shadowsocks: send target address")?;

    Ok(Box::new(stream))
}

/// A Shadowsocks UDP association to a single destination.
///
/// Shadowsocks UDP is connectionless and frames each datagram independently —
/// unlike the TCP stream there is no per-session salt, length-prefixing or
/// nonce counter. Every packet is:
/// ```text
/// salt (key_len bytes, in clear) | AEAD(subkey, nonce=0, socks5_addr | payload)
/// ```
/// with a fresh random `salt` per packet (so the all-zero nonce is never reused
/// under one key) and `subkey = HKDF-SHA1(master_key, salt, "ss-subkey")`. The
/// sealed plaintext is the SOCKS5-format destination address followed by the
/// application payload; replies carry the source address in the same shape,
/// which is stripped before the payload is returned.
pub struct ShadowsocksUdp {
    socket: UdpSocket,
    cipher: ShadowsocksCipher,
    key: Vec<u8>,
    target: TargetAddr,
}

impl ShadowsocksUdp {
    /// Bind a UDP socket and connect it to the Shadowsocks server. `target` is
    /// the eventual destination sealed into every datagram sent on this socket.
    pub async fn connect(config: &ShadowsocksOutboundConfig, target: &TargetAddr) -> Result<Self> {
        let server = lookup_host((config.server.as_str(), config.port))
            .await
            .with_context(|| format!("shadowsocks udp: resolve {}:{}", config.server, config.port))?
            .next()
            .ok_or_else(|| anyhow!("shadowsocks udp: no address for {}:{}", config.server, config.port))?;
        let socket = crate::udp::bind_egress(server).await?;
        socket
            .connect(server)
            .await
            .with_context(|| format!("shadowsocks udp: connect {server}"))?;
        Ok(Self {
            socket,
            cipher: config.cipher,
            key: config.key.clone(),
            target: target.clone(),
        })
    }

    /// Seal `payload` for the destination and send it to the server.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let packet = self.seal(payload)?;
        self.socket.send(&packet).await.context("shadowsocks udp: send")?;
        Ok(())
    }

    /// Receive one reply datagram, open it, and return the application payload
    /// (the source-address prefix is discarded).
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; MAX_UDP_PACKET];
        let n = self.socket.recv(&mut buf).await.context("shadowsocks udp: recv")?;
        self.open(&buf[..n])
    }

    fn seal(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let salt_len = self.cipher.key_size();
        let mut salt = vec![0u8; salt_len];
        random_bytes(&mut salt);
        let subkey = hkdf_sha1(&self.key, &salt, SS_SUBKEY_INFO, salt_len);
        let cipher = AeadCipher::new(self.cipher, &subkey)?;
        let mut plain = Vec::with_capacity(1 + 256 + 2 + payload.len());
        socks5::encode_address(&mut plain, &self.target);
        plain.extend_from_slice(payload);
        let sealed = cipher.seal(&[0u8; 12], &plain)?;
        let mut packet = salt;
        packet.extend_from_slice(&sealed);
        Ok(packet)
    }

    fn open(&self, datagram: &[u8]) -> Result<Vec<u8>> {
        let salt_len = self.cipher.key_size();
        if datagram.len() < salt_len + TAG_LEN {
            bail!("shadowsocks udp: datagram too short");
        }
        let (salt, sealed) = datagram.split_at(salt_len);
        let subkey = hkdf_sha1(&self.key, salt, SS_SUBKEY_INFO, salt_len);
        let cipher = AeadCipher::new(self.cipher, &subkey)?;
        let plain = cipher.open(&[0u8; 12], sealed)?;
        let (_source, offset) = socks5::decode_address(&plain)?;
        Ok(plain[offset..].to_vec())
    }
}

/// Fill `buf` with cryptographically secure random bytes from the OS.
fn random_bytes(buf: &mut [u8]) {
    if getrandom::fill(buf).is_err() {
        panic!("shadowsocks: system RNG unavailable");
    }
}

/// OpenSSL `EVP_BytesToKey` with MD5, count 1, no salt — the Shadowsocks master
/// key derivation from a password.
fn evp_bytes_to_key(password: &[u8], key_len: usize) -> Vec<u8> {
    let mut key = Vec::with_capacity(key_len);
    let mut prev: Vec<u8> = Vec::new();
    while key.len() < key_len {
        let mut hasher = Md5::new();
        hasher.update(&prev);
        hasher.update(password);
        let digest: [u8; 16] = hasher.finalize().into();
        key.extend_from_slice(&digest);
        prev = digest.to_vec();
    }
    key.truncate(key_len);
    key
}

// --- HKDF-SHA1 ------------------------------------------------------------
//
// Shadowsocks derives the per-session subkey with HKDF-SHA1. The construction
// is standard (RFC 5869) but no HKDF crate is in the dependency tree, so it is
// assembled here on top of the vetted `sha1` primitive, mirroring how the VMess
// module hand-rolls its KDF over `sha2`.

const SHA1_BLOCK: usize = 64;
const SHA1_OUT: usize = 20;

/// SHA-1 over the concatenation of `parts`.
fn sha1(parts: &[&[u8]]) -> [u8; SHA1_OUT] {
    let mut hasher = Sha1::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// Standard HMAC-SHA1.
fn hmac_sha1(key: &[u8], msg: &[u8]) -> [u8; SHA1_OUT] {
    let mut block = [0u8; SHA1_BLOCK];
    if key.len() > SHA1_BLOCK {
        block[..SHA1_OUT].copy_from_slice(&sha1(&[key]));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0u8; SHA1_BLOCK];
    let mut opad = [0u8; SHA1_BLOCK];
    for i in 0..SHA1_BLOCK {
        ipad[i] = block[i] ^ 0x36;
        opad[i] = block[i] ^ 0x5c;
    }
    let inner = sha1(&[&ipad, msg]);
    sha1(&[&opad, &inner])
}

/// HKDF-SHA1 (extract + expand) producing `length` bytes of output.
fn hkdf_sha1(ikm: &[u8], salt: &[u8], info: &[u8], length: usize) -> Vec<u8> {
    let prk = hmac_sha1(salt, ikm);
    let mut okm = Vec::with_capacity(length);
    let mut prev: Vec<u8> = Vec::new();
    let mut counter: u8 = 1;
    while okm.len() < length {
        let mut input = Vec::with_capacity(prev.len() + info.len() + 1);
        input.extend_from_slice(&prev);
        input.extend_from_slice(info);
        input.push(counter);
        let block = hmac_sha1(&prk, &input);
        okm.extend_from_slice(&block);
        prev = block.to_vec();
        counter = counter.wrapping_add(1);
    }
    okm.truncate(length);
    okm
}

/// Increment a 12-byte little-endian counter nonce in place.
fn increment_nonce(nonce: &mut [u8; 12]) {
    for byte in nonce.iter_mut() {
        *byte = byte.wrapping_add(1);
        if *byte != 0 {
            break;
        }
    }
}

/// A keyed Shadowsocks AEAD cipher (one direction).
enum AeadCipher {
    Aes128(Box<Aes128Gcm>),
    Aes256(Box<Aes256Gcm>),
    Chacha(Box<ChaCha20Poly1305>),
}

impl AeadCipher {
    fn new(cipher: ShadowsocksCipher, subkey: &[u8]) -> Result<Self> {
        match cipher {
            ShadowsocksCipher::Aes128Gcm => Ok(AeadCipher::Aes128(Box::new(
                Aes128Gcm::new_from_slice(subkey).map_err(|_| anyhow!("shadowsocks: invalid aes-128 key"))?,
            ))),
            ShadowsocksCipher::Aes256Gcm => Ok(AeadCipher::Aes256(Box::new(
                Aes256Gcm::new_from_slice(subkey).map_err(|_| anyhow!("shadowsocks: invalid aes-256 key"))?,
            ))),
            ShadowsocksCipher::Chacha20IetfPoly1305 => Ok(AeadCipher::Chacha(Box::new(
                ChaCha20Poly1305::new_from_slice(subkey).map_err(|_| anyhow!("shadowsocks: invalid chacha key"))?,
            ))),
        }
    }

    fn seal(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: plaintext,
            aad: &[],
        };
        let result = match self {
            AeadCipher::Aes128(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Aes256(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("shadowsocks: AEAD seal failed"))
    }

    fn open(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: ciphertext,
            aad: &[],
        };
        let result = match self {
            AeadCipher::Aes128(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Aes256(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            AeadCipher::Chacha(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("shadowsocks: AEAD open failed"))
    }
}

/// Read-side framing state machine.
enum ReadState {
    /// Waiting for the peer's `salt_len`-byte salt (derives the read cipher).
    Salt,
    /// Waiting for the 18-byte AEAD-sealed payload length.
    Len,
    /// Waiting for a `clen + 16`-byte sealed payload chunk.
    Data(usize),
    /// Clean EOF (the peer closed the connection).
    Eof,
}

/// Wraps a TCP stream: writes seal application data into Shadowsocks AEAD
/// chunks; reads strip the peer salt, then decrypt length-prefixed chunks. The
/// client salt and sealed target address are sent at connect time.
struct ShadowsocksStream {
    inner: TcpStream,
    cipher: ShadowsocksCipher,
    master_key: Vec<u8>,
    // Write side.
    write_cipher: AeadCipher,
    write_nonce: [u8; 12],
    write_buf: Vec<u8>,
    write_pos: usize,
    // Read side.
    read_cipher: Option<AeadCipher>,
    read_nonce: [u8; 12],
    read_state: ReadState,
    read_raw: Vec<u8>,
    plain: Vec<u8>,
    plain_pos: usize,
}

impl ShadowsocksStream {
    fn new(inner: TcpStream, cipher: ShadowsocksCipher, master_key: Vec<u8>, write_cipher: AeadCipher) -> Self {
        Self {
            inner,
            cipher,
            master_key,
            write_cipher,
            write_nonce: [0u8; 12],
            write_buf: Vec::new(),
            write_pos: 0,
            read_cipher: None,
            read_nonce: [0u8; 12],
            read_state: ReadState::Salt,
            read_raw: Vec::new(),
            plain: Vec::new(),
            plain_pos: 0,
        }
    }

    /// Flush any pending sealed bytes to the inner stream.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "shadowsocks: write zero")));
            }
            self.write_pos += n;
        }
        self.write_buf.clear();
        self.write_pos = 0;
        Poll::Ready(Ok(()))
    }

    /// Seal `plaintext` (at most [`MAX_CHUNK`] bytes) into a length-prefixed AEAD
    /// chunk queued for writing.
    fn queue_chunk(&mut self, plaintext: &[u8]) -> io::Result<()> {
        let len = u16::try_from(plaintext.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "shadowsocks: chunk too large"))?;
        let sealed_len = self
            .write_cipher
            .seal(&self.write_nonce, &len.to_be_bytes())
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);
        let sealed_payload = self
            .write_cipher
            .seal(&self.write_nonce, plaintext)
            .map_err(|e| io::Error::other(e.to_string()))?;
        increment_nonce(&mut self.write_nonce);

        self.write_buf.clear();
        self.write_pos = 0;
        self.write_buf.extend_from_slice(&sealed_len);
        self.write_buf.extend_from_slice(&sealed_payload);
        Ok(())
    }
}

fn decrypt_err(e: anyhow::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

/// The read cipher is created from the peer salt before any chunk is decrypted,
/// so reaching a chunk state without it is a logic error rather than a
/// recoverable condition; surface it as an error instead of panicking.
fn read_cipher_unset() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "shadowsocks: read cipher unset")
}

impl AsyncRead for ShadowsocksStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let salt_len = this.cipher.key_size();
        loop {
            if this.plain_pos < this.plain.len() {
                let n = buf.remaining().min(this.plain.len() - this.plain_pos);
                buf.put_slice(&this.plain[this.plain_pos..this.plain_pos + n]);
                this.plain_pos += n;
                return Poll::Ready(Ok(()));
            }
            if matches!(this.read_state, ReadState::Eof) {
                return Poll::Ready(Ok(()));
            }

            let need = match this.read_state {
                ReadState::Salt => salt_len,
                ReadState::Len => 2 + TAG_LEN,
                ReadState::Data(clen) => clen + TAG_LEN,
                ReadState::Eof => unreachable!(),
            };

            if this.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                ready!(Pin::new(&mut this.inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // TCP FIN: Shadowsocks marks end-of-stream by closing, with
                    // no terminating chunk, so a clean EOF here is expected.
                    this.read_state = ReadState::Eof;
                    return Poll::Ready(Ok(()));
                }
                this.read_raw.extend_from_slice(filled);
                continue;
            }

            match this.read_state {
                ReadState::Salt => {
                    let salt: Vec<u8> = this.read_raw.drain(..salt_len).collect();
                    let subkey = hkdf_sha1(&this.master_key, &salt, SS_SUBKEY_INFO, salt_len);
                    let cipher = AeadCipher::new(this.cipher, &subkey).map_err(decrypt_err)?;
                    this.read_cipher = Some(cipher);
                    this.read_state = ReadState::Len;
                }
                ReadState::Len => {
                    let sealed: Vec<u8> = this.read_raw.drain(..2 + TAG_LEN).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    let clen = u16::from_be_bytes([plain[0], plain[1]]) as usize;
                    if clen == 0 || clen > MAX_CHUNK {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "shadowsocks: invalid chunk length",
                        )));
                    }
                    this.read_state = ReadState::Data(clen);
                }
                ReadState::Data(clen) => {
                    let sealed: Vec<u8> = this.read_raw.drain(..clen + TAG_LEN).collect();
                    let Some(cipher) = this.read_cipher.as_ref() else {
                        return Poll::Ready(Err(read_cipher_unset()));
                    };
                    let plain = cipher.open(&this.read_nonce, &sealed).map_err(decrypt_err)?;
                    increment_nonce(&mut this.read_nonce);
                    this.plain = plain;
                    this.plain_pos = 0;
                    this.read_state = ReadState::Len;
                }
                ReadState::Eof => unreachable!(),
            }
        }
    }
}

impl AsyncWrite for ShadowsocksStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_CHUNK);
        this.queue_chunk(&buf[..take])?;
        // Best-effort flush; remaining bytes drain on the next poll.
        if let Poll::Ready(Err(e)) = this.poll_drain(cx) {
            return Poll::Ready(Err(e));
        }
        Poll::Ready(Ok(take))
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        Pin::new(&mut this.inner).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;
        // Shadowsocks has no terminating chunk: closing the TCP stream (FIN) is
        // the end-of-stream signal.
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::ProxyEntry;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn evp_bytes_to_key_single_block_is_md5() {
        // For a 16-byte key the derivation is exactly MD5(password).
        // MD5("foobar") = 3858f62230ac3c915f300c664312c63f.
        let key = evp_bytes_to_key(b"foobar", 16);
        let expected = [
            0x38, 0x58, 0xf6, 0x22, 0x30, 0xac, 0x3c, 0x91, 0x5f, 0x30, 0x0c, 0x66, 0x43, 0x12, 0xc6, 0x3f,
        ];
        assert_eq!(key, expected);
    }

    #[test]
    fn evp_bytes_to_key_extends_to_requested_length() {
        // A 32-byte key chains a second MD5 block; the first 16 bytes still
        // equal MD5(password).
        let key = evp_bytes_to_key(b"foobar", 32);
        assert_eq!(key.len(), 32);
        assert_eq!(&key[..16], &evp_bytes_to_key(b"foobar", 16)[..]);
    }

    #[test]
    fn hmac_sha1_matches_rfc2202_case2() {
        // RFC 2202 test case 2: key="Jefe", data="what do ya want for nothing?".
        let mac = hmac_sha1(b"Jefe", b"what do ya want for nothing?");
        let expected = [
            0xef, 0xfc, 0xdf, 0x6a, 0xe5, 0xeb, 0x2f, 0xa2, 0xd2, 0x74, 0x16, 0xd5, 0xf1, 0x84, 0xdf, 0x9c, 0x25, 0x9a,
            0x7c, 0x79,
        ];
        assert_eq!(mac, expected);
    }

    #[test]
    fn hkdf_sha1_matches_rfc5869_case4() {
        // RFC 5869 Appendix A.4 (SHA-1).
        let ikm = [0x0bu8; 11];
        let salt: Vec<u8> = (0u8..=0x0c).collect();
        let info: Vec<u8> = (0xf0u8..=0xf9).collect();
        let okm = hkdf_sha1(&ikm, &salt, &info, 42);
        let expected = [
            0x08, 0x5a, 0x01, 0xea, 0x1b, 0x10, 0xf3, 0x69, 0x33, 0x06, 0x8b, 0x56, 0xef, 0xa5, 0xad, 0x81, 0xa4, 0xf1,
            0x4b, 0x82, 0x2f, 0x5b, 0x09, 0x15, 0x68, 0xa9, 0xcd, 0xd4, 0xf1, 0x55, 0xfd, 0xa2, 0xc2, 0x2e, 0x42, 0x24,
            0x78, 0xd3, 0x05, 0xf3, 0xf8, 0x96,
        ];
        assert_eq!(okm, expected);
    }

    #[test]
    fn nonce_increments_little_endian_with_carry() {
        let mut nonce = [0u8; 12];
        increment_nonce(&mut nonce);
        assert_eq!(nonce[0], 1);
        nonce[0] = 0xff;
        increment_nonce(&mut nonce);
        assert_eq!(nonce[0], 0);
        assert_eq!(nonce[1], 1);
    }

    #[test]
    fn parses_aes_256_gcm_entry() {
        let yaml = "name: s\ntype: ss\nserver: example.com\nport: 8388\ncipher: aes-256-gcm\npassword: secret\n";
        let cfg = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.server, "example.com");
        assert_eq!(cfg.port, 8388);
        assert_eq!(cfg.cipher, ShadowsocksCipher::Aes256Gcm);
        assert_eq!(cfg.key, evp_bytes_to_key(b"secret", 32));
    }

    #[test]
    fn chacha_alias_is_accepted() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: chacha20-poly1305\npassword: p\n";
        let cfg = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.cipher, ShadowsocksCipher::Chacha20IetfPoly1305);
    }

    #[test]
    fn legacy_stream_cipher_is_rejected() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: aes-256-cfb\npassword: p\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[test]
    fn ss2022_method_is_rejected() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: 2022-blake3-aes-256-gcm\npassword: p\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("not supported"), "got: {err}");
    }

    #[test]
    fn plugin_is_rejected() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: aes-128-gcm\npassword: p\nplugin: obfs\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("plugin"), "got: {err}");
    }

    #[test]
    fn missing_cipher_is_rejected() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\npassword: p\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("cipher"), "got: {err}");
    }

    #[test]
    fn missing_password_is_rejected() {
        let yaml = "name: s\ntype: ss\nserver: h\nport: 1\ncipher: aes-128-gcm\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("password"), "got: {err}");
    }

    #[test]
    fn missing_server_is_rejected() {
        let yaml = "name: s\ntype: ss\nport: 1\ncipher: aes-128-gcm\npassword: p\n";
        let err = ShadowsocksOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("server"), "got: {err}");
    }
}
