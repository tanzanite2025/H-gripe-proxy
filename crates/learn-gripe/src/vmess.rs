//! VMess outbound (AEAD).
//!
//! Implements the VMess request/response framing and the chunked AEAD body
//! stream only; the transport (tcp/ws/grpc/xhttp/httpupgrade/h2) and security
//! (none/tls/reality) layers it runs over are provided by [`crate::transport`]
//! via the shared [`crate::transport::build_layers`], so this module is purely
//! the protocol layer. Because security and transport are orthogonal, VMess
//! works over every supported transport and over REALITY automatically.
//!
//! Only the modern **AEAD** header format (`alterId: 0`) is implemented. The
//! legacy MD5-authenticated header (`alterId > 0`) is rejected by
//! [`VmessOutboundConfig::from_proxy`] rather than silently mis-encoded. Body
//! security is `aes-128-gcm` (default / `auto`) or `chacha20-poly1305`.
//!
//! All cryptographic primitives are delegated to vetted RustCrypto crates
//! (`aes`, `aes-gcm`, `chacha20poly1305`, `sha2`, `md-5`, `crc32fast`); the only
//! thing assembled here is the VMess-specific KDF (a nested-HMAC construction
//! over SHA-256 that no crate provides) and the on-wire framing.
//!
//! Request layout (before AEAD sealing of the command block):
//! ```text
//! ver(1) | iv(16) | key(16) | V(1) | opt(1) | pad<<4|sec(1) | rsv(1) | cmd(1)
//!   | port(2) | atyp(1) | addr | fnv1a32(4)
//! ```
//! `atyp` is 0x01 IPv4 / 0x02 domain / 0x03 IPv6. The sealed header on the wire
//! is `authID(16) | AEAD(len)(2+16) | nonce(8) | AEAD(command)(N+16)`.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};
use std::time::{SystemTime, UNIX_EPOCH};

use aes::Aes128;
use aes::cipher::BlockEncrypt;
use aes::cipher::generic_array::GenericArray;
use aes_gcm::Aes128Gcm;
use aes_gcm::aead::{Aead, KeyInit, Payload};
use anyhow::{Context, Result, anyhow, bail};
use chacha20poly1305::ChaCha20Poly1305;
use md5::Md5;
use sha2::{Digest, Sha256};
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt, ReadBuf};

use crate::address::TargetAddr;
use crate::outbound::BoxedStream;
use crate::proxy::{ProxyEntry, parse_uuid};
use crate::transport::{self, Security, Transport};

const VERSION: u8 = 0x01;
const CMD_TCP: u8 = 0x01;
const ATYP_IPV4: u8 = 0x01;
const ATYP_DOMAIN: u8 = 0x02;
const ATYP_IPV6: u8 = 0x03;
/// Request option: chunked AEAD body stream (no masking / global padding).
const OPT_CHUNK_STREAM: u8 = 0x01;
/// Body security selectors carried in the low nibble of the security byte.
const SEC_AES_128_GCM: u8 = 0x03;
const SEC_CHACHA20_POLY1305: u8 = 0x04;
/// Largest plaintext carried in a single body chunk.
const MAX_CHUNK: usize = 16384;

/// Static suffix mixed into the user id to derive the command key.
const CMD_KEY_SUFFIX: &[u8] = b"c48619fe-8f02-49e0-b9e9-edf763e17e21";

// VMess AEAD KDF salt labels (verbatim from the protocol).
const KDF_SALT_AUTH_ID: &[u8] = b"AES Auth ID Encryption";
const KDF_SALT_REQ_LEN_KEY: &[u8] = b"VMess Header AEAD Key_Length";
const KDF_SALT_REQ_LEN_IV: &[u8] = b"VMess Header AEAD Nonce_Length";
const KDF_SALT_REQ_HDR_KEY: &[u8] = b"VMess Header AEAD Key";
const KDF_SALT_REQ_HDR_IV: &[u8] = b"VMess Header AEAD Nonce";
const KDF_SALT_RESP_LEN_KEY: &[u8] = b"AEAD Resp Header Len Key";
const KDF_SALT_RESP_LEN_IV: &[u8] = b"AEAD Resp Header Len IV";
const KDF_SALT_RESP_HDR_KEY: &[u8] = b"AEAD Resp Header Key";
const KDF_SALT_RESP_HDR_IV: &[u8] = b"AEAD Resp Header IV";

/// VMess body cipher (AEAD `security`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VmessCipher {
    Aes128Gcm,
    Chacha20Poly1305,
}

impl VmessCipher {
    /// The low-nibble security selector written into the request header.
    fn security_byte(self) -> u8 {
        match self {
            VmessCipher::Aes128Gcm => SEC_AES_128_GCM,
            VmessCipher::Chacha20Poly1305 => SEC_CHACHA20_POLY1305,
        }
    }
}

/// Fully-resolved VMess outbound parameters.
///
/// `security` and `transport` are orthogonal layers (see [`crate::transport`]):
/// e.g. `VMess-WS-TLS` is `Security::Tls` + `Transport::Ws`. `cipher` is the
/// AEAD body security, independent of the transport-level TLS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VmessOutboundConfig {
    pub server: String,
    pub port: u16,
    pub uuid: [u8; 16],
    pub cipher: VmessCipher,
    pub security: Security,
    pub transport: Transport,
}

impl VmessOutboundConfig {
    /// Build an outbound config from a parsed `vmess` proxy entry, rejecting
    /// sub-features that are not implemented yet so traffic is never mis-framed.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("vmess: missing server")?;
        let port = opts.port.context("vmess: missing port")?;
        let uuid =
            parse_uuid(opts.uuid.as_deref().context("vmess: missing uuid")?).map_err(|e| anyhow!("vmess: {e}"))?;

        // Only the AEAD header format (alterId 0) is implemented; the legacy
        // MD5 alterId authentication is intentionally not carried.
        if let Some(alter_id) = opts.alter_id
            && alter_id != 0
        {
            bail!("vmess: alterId {alter_id} (legacy MD5 auth) not supported; use alterId 0 (AEAD)");
        }

        let cipher = match opts.cipher.as_deref() {
            None | Some("") | Some("auto") | Some("aes-128-gcm") => VmessCipher::Aes128Gcm,
            Some("chacha20-poly1305") | Some("chacha20-ietf-poly1305") => VmessCipher::Chacha20Poly1305,
            Some(other) => {
                bail!("vmess: cipher {other:?} not supported (use auto / aes-128-gcm / chacha20-poly1305)");
            }
        };

        // VMess is plaintext unless `tls` / `reality-opts` opt in; security and
        // transport are orthogonal to the framing and built by the shared helper.
        let (security, transport) = transport::build_layers(opts, "vmess", false)?;

        Ok(Self {
            server,
            port,
            uuid,
            cipher,
            security,
            transport,
        })
    }
}

/// Connect a VMess outbound to `target` and return a relay-ready stream with
/// the AEAD request header already sent. The returned stream encrypts writes
/// into VMess body chunks and decrypts the response header + body on reads.
pub async fn connect(config: &VmessOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let mut stream = transport::establish(&config.server, config.port, &config.security, &config.transport).await?;

    let mut body_key = [0u8; 16];
    let mut body_iv = [0u8; 16];
    let mut v = [0u8; 1];
    random_bytes(&mut body_key);
    random_bytes(&mut body_iv);
    random_bytes(&mut v);
    let response_verifier = v[0];

    let cmd_key = command_key(&config.uuid);
    let command = encode_command(&body_iv, &body_key, response_verifier, config.cipher, target);
    let header = seal_request_header(&cmd_key, &command)?;
    stream.write_all(&header).await.context("vmess: send request header")?;

    let session = VmessSession::new(body_key, body_iv, response_verifier, config.cipher)?;
    Ok(Box::new(VmessStream::new(stream, session)))
}

/// Fill `buf` with cryptographically secure random bytes from the OS.
fn random_bytes(buf: &mut [u8]) {
    // getrandom reads directly from the platform CSPRNG; it cannot meaningfully
    // fail on the supported targets, and a failure here is unrecoverable.
    if getrandom::fill(buf).is_err() {
        panic!("vmess: system RNG unavailable");
    }
}

/// `MD5(uuid || CMD_KEY_SUFFIX)` — the per-user command key.
fn command_key(uuid: &[u8; 16]) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(uuid);
    hasher.update(CMD_KEY_SUFFIX);
    hasher.finalize().into()
}

/// Encode the plaintext VMess command block (pre-AEAD).
fn encode_command(
    iv: &[u8; 16],
    key: &[u8; 16],
    response_verifier: u8,
    cipher: VmessCipher,
    target: &TargetAddr,
) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    buf.push(VERSION);
    buf.extend_from_slice(iv);
    buf.extend_from_slice(key);
    buf.push(response_verifier);
    buf.push(OPT_CHUNK_STREAM);
    // padding length 0 in the high nibble; body security in the low nibble.
    buf.push(cipher.security_byte());
    buf.push(0); // reserved
    buf.push(CMD_TCP);
    buf.extend_from_slice(&target.port().to_be_bytes());
    match target {
        TargetAddr::Ip(SocketAddr::V4(addr)) => {
            buf.push(ATYP_IPV4);
            buf.extend_from_slice(&addr.ip().octets());
        }
        TargetAddr::Ip(SocketAddr::V6(addr)) => {
            buf.push(ATYP_IPV6);
            buf.extend_from_slice(&addr.ip().octets());
        }
        TargetAddr::Domain(host, _) => {
            buf.push(ATYP_DOMAIN);
            buf.push(host.len() as u8);
            buf.extend_from_slice(host.as_bytes());
        }
    }
    // No request padding (padding length 0), then the FNV-1a checksum.
    let checksum = fnv1a32(&buf);
    buf.extend_from_slice(&checksum.to_be_bytes());
    buf
}

/// Seal the command block into the on-wire AEAD request header.
fn seal_request_header(cmd_key: &[u8; 16], command: &[u8]) -> Result<Vec<u8>> {
    let auth_id = auth_id(cmd_key, unix_time());
    let mut nonce = [0u8; 8];
    random_bytes(&mut nonce);

    let len_key = kdf16(cmd_key, &[KDF_SALT_REQ_LEN_KEY, &auth_id, &nonce]);
    let len_iv = kdf12(cmd_key, &[KDF_SALT_REQ_LEN_IV, &auth_id, &nonce]);
    let length = u16::try_from(command.len()).map_err(|_| anyhow!("vmess: command header too large"))?;
    let sealed_len = aes_gcm_seal(&len_key, &len_iv, &auth_id, &length.to_be_bytes())?;

    let hdr_key = kdf16(cmd_key, &[KDF_SALT_REQ_HDR_KEY, &auth_id, &nonce]);
    let hdr_iv = kdf12(cmd_key, &[KDF_SALT_REQ_HDR_IV, &auth_id, &nonce]);
    let sealed_hdr = aes_gcm_seal(&hdr_key, &hdr_iv, &auth_id, command)?;

    let mut out = Vec::with_capacity(16 + sealed_len.len() + 8 + sealed_hdr.len());
    out.extend_from_slice(&auth_id);
    out.extend_from_slice(&sealed_len);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed_hdr);
    Ok(out)
}

/// Build the 16-byte authenticated user id for the current connection:
/// `AES-ECB(KDF16(cmd_key, "AES Auth ID Encryption"), time(8) | rand(4) | crc32(4))`.
fn auth_id(cmd_key: &[u8; 16], time: i64) -> [u8; 16] {
    let mut block = [0u8; 16];
    block[..8].copy_from_slice(&time.to_be_bytes());
    random_bytes(&mut block[8..12]);
    let crc = crc32fast::hash(&block[..12]);
    block[12..16].copy_from_slice(&crc.to_be_bytes());

    let key = kdf16(cmd_key, &[KDF_SALT_AUTH_ID]);
    aes_ecb_encrypt_block(&key, &block)
}

/// Current Unix time in seconds (the server tolerates a small skew window).
fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// FNV-1a 32-bit hash, used as the request command checksum.
fn fnv1a32(data: &[u8]) -> u32 {
    let mut hash = 0x811c_9dc5u32;
    for byte in data {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash
}

/// Encrypt a single 16-byte block with AES-128 in ECB mode (one raw block).
fn aes_ecb_encrypt_block(key: &[u8; 16], block: &[u8; 16]) -> [u8; 16] {
    let cipher = <Aes128 as aes::cipher::KeyInit>::new(GenericArray::from_slice(key));
    let mut buf = *GenericArray::<u8, aes::cipher::consts::U16>::from_slice(block);
    cipher.encrypt_block(&mut buf);
    let mut out = [0u8; 16];
    out.copy_from_slice(buf.as_slice());
    out
}

/// AES-128-GCM seal with an explicit 12-byte nonce (header AEAD).
fn aes_gcm_seal(key: &[u8; 16], nonce: &[u8; 12], aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes128Gcm::new_from_slice(key).map_err(|_| anyhow!("vmess: invalid aes key length"))?;
    cipher
        .encrypt(GenericArray::from_slice(nonce), Payload { msg: plaintext, aad })
        .map_err(|_| anyhow!("vmess: aes-gcm seal failed"))
}

/// AES-128-GCM open with an explicit 12-byte nonce (header AEAD).
fn aes_gcm_open(key: &[u8; 16], nonce: &[u8; 12], aad: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>> {
    let cipher = Aes128Gcm::new_from_slice(key).map_err(|_| anyhow!("vmess: invalid aes key length"))?;
    cipher
        .decrypt(GenericArray::from_slice(nonce), Payload { msg: ciphertext, aad })
        .map_err(|_| anyhow!("vmess: aes-gcm open failed"))
}

/// SHA-256 over the concatenation of `parts`.
fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

/// First 16 bytes of `SHA-256(input)` (response key/iv derivation).
fn sha256_16(input: &[u8]) -> [u8; 16] {
    let digest = sha256(&[input]);
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

/// `MD5`-expanded 32-byte key for ChaCha20-Poly1305 (v2ray key schedule):
/// `MD5(key) || MD5(MD5(key))`.
fn chacha_key(key: &[u8; 16]) -> [u8; 32] {
    let mut first = Md5::new();
    first.update(key);
    let first: [u8; 16] = first.finalize().into();
    let mut second = Md5::new();
    second.update(first);
    let second: [u8; 16] = second.finalize().into();
    let mut out = [0u8; 32];
    out[..16].copy_from_slice(&first);
    out[16..].copy_from_slice(&second);
    out
}

// --- VMess AEAD KDF -------------------------------------------------------
//
// VMess derives every sub-key with a nested-HMAC KDF: the root hash function is
// HMAC-SHA256 keyed with "VMess AEAD KDF", and each path element wraps the
// previous level in another HMAC whose "hash function" is that previous level.
// No crate implements this construction, so it is assembled here directly on
// top of the vetted `sha2` SHA-256 primitive. SHA-256's block size is 64 bytes
// at every level.

const SHA256_BLOCK: usize = 64;
const KDF_ROOT_KEY: &[u8] = b"VMess AEAD KDF";

/// HMAC inner/outer key pads (`key ⊕ ipad`, `key ⊕ opad`) for SHA-256.
fn hmac_pads(key: &[u8]) -> ([u8; SHA256_BLOCK], [u8; SHA256_BLOCK]) {
    let mut block = [0u8; SHA256_BLOCK];
    if key.len() > SHA256_BLOCK {
        block[..32].copy_from_slice(&sha256(&[key]));
    } else {
        block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0u8; SHA256_BLOCK];
    let mut opad = [0u8; SHA256_BLOCK];
    for i in 0..SHA256_BLOCK {
        ipad[i] = block[i] ^ 0x36;
        opad[i] = block[i] ^ 0x5c;
    }
    (ipad, opad)
}

/// Standard HMAC-SHA256 (the innermost KDF hash, keyed with `KDF_ROOT_KEY`).
fn hmac_sha256(key: &[u8], msg: &[u8]) -> [u8; 32] {
    let (ipad, opad) = hmac_pads(key);
    let inner = sha256(&[&ipad, msg]);
    sha256(&[&opad, &inner])
}

/// Evaluate the VMess KDF nested HMAC tree for the given `paths` over `msg`.
fn kdf_rec(level: usize, paths: &[&[u8]], msg: &[u8]) -> [u8; 32] {
    if level == 0 {
        return hmac_sha256(KDF_ROOT_KEY, msg);
    }
    let (ipad, opad) = hmac_pads(paths[level - 1]);
    let mut inner_in = Vec::with_capacity(SHA256_BLOCK + msg.len());
    inner_in.extend_from_slice(&ipad);
    inner_in.extend_from_slice(msg);
    let inner = kdf_rec(level - 1, paths, &inner_in);
    let mut outer_in = Vec::with_capacity(SHA256_BLOCK + inner.len());
    outer_in.extend_from_slice(&opad);
    outer_in.extend_from_slice(&inner);
    kdf_rec(level - 1, paths, &outer_in)
}

/// VMess KDF: `key` folded through the nested HMAC tree defined by `paths`.
fn kdf(key: &[u8], paths: &[&[u8]]) -> [u8; 32] {
    kdf_rec(paths.len(), paths, key)
}

/// First 16 bytes of the KDF output (AEAD keys).
fn kdf16(key: &[u8], paths: &[&[u8]]) -> [u8; 16] {
    let full = kdf(key, paths);
    let mut out = [0u8; 16];
    out.copy_from_slice(&full[..16]);
    out
}

/// First 12 bytes of the KDF output (AEAD nonces).
fn kdf12(key: &[u8], paths: &[&[u8]]) -> [u8; 12] {
    let full = kdf(key, paths);
    let mut out = [0u8; 12];
    out.copy_from_slice(&full[..12]);
    out
}

/// A keyed AEAD body cipher (request or response direction).
enum BodyCipher {
    Aes(Box<Aes128Gcm>),
    Chacha(Box<ChaCha20Poly1305>),
}

impl BodyCipher {
    /// Build the body cipher for `cipher` from the 16-byte base key
    /// (chacha applies the v2ray MD5 key expansion).
    fn new(cipher: VmessCipher, base_key: &[u8; 16]) -> Result<Self> {
        match cipher {
            VmessCipher::Aes128Gcm => {
                let aead = Aes128Gcm::new_from_slice(base_key).map_err(|_| anyhow!("vmess: invalid aes body key"))?;
                Ok(BodyCipher::Aes(Box::new(aead)))
            }
            VmessCipher::Chacha20Poly1305 => {
                let key = chacha_key(base_key);
                let aead =
                    ChaCha20Poly1305::new_from_slice(&key).map_err(|_| anyhow!("vmess: invalid chacha body key"))?;
                Ok(BodyCipher::Chacha(Box::new(aead)))
            }
        }
    }

    fn seal(&self, nonce: &[u8; 12], plaintext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: plaintext,
            aad: &[],
        };
        let result = match self {
            BodyCipher::Aes(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
            BodyCipher::Chacha(c) => c.encrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("vmess: body seal failed"))
    }

    fn open(&self, nonce: &[u8; 12], ciphertext: &[u8]) -> Result<Vec<u8>> {
        let payload = Payload {
            msg: ciphertext,
            aad: &[],
        };
        let result = match self {
            BodyCipher::Aes(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
            BodyCipher::Chacha(c) => c.decrypt(GenericArray::from_slice(nonce), payload),
        };
        result.map_err(|_| anyhow!("vmess: body open failed"))
    }
}

/// The 12-byte body chunk nonce: `count(2) | iv[2..12]`.
fn chunk_nonce(iv: &[u8; 16], count: u16) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..2].copy_from_slice(&count.to_be_bytes());
    nonce[2..].copy_from_slice(&iv[2..12]);
    nonce
}

/// All derived per-connection material needed to drive the body stream.
struct VmessSession {
    request_cipher: BodyCipher,
    request_iv: [u8; 16],
    response_cipher: BodyCipher,
    response_iv: [u8; 16],
    response_verifier: u8,
    response_len_key: [u8; 16],
    response_len_iv: [u8; 12],
    response_hdr_key: [u8; 16],
    response_hdr_iv: [u8; 12],
}

impl VmessSession {
    fn new(body_key: [u8; 16], body_iv: [u8; 16], response_verifier: u8, cipher: VmessCipher) -> Result<Self> {
        let request_cipher = BodyCipher::new(cipher, &body_key)?;

        let response_body_key = sha256_16(&body_key);
        let response_body_iv = sha256_16(&body_iv);
        let response_cipher = BodyCipher::new(cipher, &response_body_key)?;

        Ok(Self {
            request_cipher,
            request_iv: body_iv,
            response_cipher,
            response_iv: response_body_iv,
            response_verifier,
            response_len_key: kdf16(&response_body_key, &[KDF_SALT_RESP_LEN_KEY]),
            response_len_iv: kdf12(&response_body_iv, &[KDF_SALT_RESP_LEN_IV]),
            response_hdr_key: kdf16(&response_body_key, &[KDF_SALT_RESP_HDR_KEY]),
            response_hdr_iv: kdf12(&response_body_iv, &[KDF_SALT_RESP_HDR_IV]),
        })
    }
}

/// Read-side framing state machine.
#[derive(Clone, Copy)]
enum ReadState {
    /// Waiting for the 18-byte AEAD-sealed response header length.
    ResponseLen,
    /// Waiting for the sealed response header (`len + 16` bytes).
    ResponseHeader(usize),
    /// Waiting for a 2-byte body chunk length.
    BodyLen,
    /// Waiting for a `clen`-byte sealed body chunk.
    BodyData(usize),
    /// Stream terminated (empty chunk or clean EOF).
    Eof,
}

/// Wraps a transport stream: writes are encrypted into VMess body chunks; reads
/// strip and verify the AEAD response header, then decrypt body chunks. The
/// AEAD request header is sent at connect time before this wrapper is built.
struct VmessStream<S> {
    inner: S,
    session: VmessSession,
    // Read side.
    read_state: ReadState,
    read_raw: Vec<u8>,
    plain: Vec<u8>,
    plain_pos: usize,
    read_count: u16,
    // Write side.
    write_buf: Vec<u8>,
    write_pos: usize,
    write_count: u16,
    shutdown_sent: bool,
}

impl<S> VmessStream<S> {
    fn new(inner: S, session: VmessSession) -> Self {
        Self {
            inner,
            session,
            read_state: ReadState::ResponseLen,
            read_raw: Vec::new(),
            plain: Vec::new(),
            plain_pos: 0,
            read_count: 0,
            write_buf: Vec::new(),
            write_pos: 0,
            write_count: 0,
            shutdown_sent: false,
        }
    }
}

impl<S: AsyncWrite + Unpin> VmessStream<S> {
    /// Flush any pending sealed bytes to the inner stream.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "vmess: write zero")));
            }
            self.write_pos += n;
        }
        self.write_buf.clear();
        self.write_pos = 0;
        Poll::Ready(Ok(()))
    }

    /// Seal `plaintext` into a length-prefixed body chunk queued for writing.
    fn queue_chunk(&mut self, plaintext: &[u8]) -> io::Result<()> {
        let nonce = chunk_nonce(&self.session.request_iv, self.write_count);
        let chunk = self
            .session
            .request_cipher
            .seal(&nonce, plaintext)
            .map_err(|e| io::Error::other(e.to_string()))?;
        self.write_count = self.write_count.wrapping_add(1);
        let len = u16::try_from(chunk.len())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "vmess: chunk too large"))?;
        self.write_buf.clear();
        self.write_pos = 0;
        self.write_buf.extend_from_slice(&len.to_be_bytes());
        self.write_buf.extend_from_slice(&chunk);
        Ok(())
    }
}

fn decrypt_err(e: anyhow::Error) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, e.to_string())
}

impl<S: AsyncRead + Unpin> AsyncRead for VmessStream<S> {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
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
                ReadState::ResponseLen => 18,
                ReadState::ResponseHeader(len) => len + 16,
                ReadState::BodyLen => 2,
                ReadState::BodyData(clen) => clen,
                ReadState::Eof => unreachable!(),
            };

            if this.read_raw.len() < need {
                let mut scratch = [0u8; 4096];
                let mut read_buf = ReadBuf::new(&mut scratch);
                ready!(Pin::new(&mut this.inner).poll_read(cx, &mut read_buf))?;
                let filled = read_buf.filled();
                if filled.is_empty() {
                    // Clean EOF: surface it (a well-behaved peer ends with an
                    // empty terminating chunk, which is handled below).
                    this.read_state = ReadState::Eof;
                    return Poll::Ready(Ok(()));
                }
                this.read_raw.extend_from_slice(filled);
                continue;
            }

            match this.read_state {
                ReadState::ResponseLen => {
                    let sealed: Vec<u8> = this.read_raw.drain(..18).collect();
                    let plain = aes_gcm_open(
                        &this.session.response_len_key,
                        &this.session.response_len_iv,
                        &[],
                        &sealed,
                    )
                    .map_err(decrypt_err)?;
                    let len = u16::from_be_bytes([plain[0], plain[1]]) as usize;
                    this.read_state = ReadState::ResponseHeader(len);
                }
                ReadState::ResponseHeader(len) => {
                    let sealed: Vec<u8> = this.read_raw.drain(..len + 16).collect();
                    let header = aes_gcm_open(
                        &this.session.response_hdr_key,
                        &this.session.response_hdr_iv,
                        &[],
                        &sealed,
                    )
                    .map_err(decrypt_err)?;
                    if header.is_empty() || header[0] != this.session.response_verifier {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "vmess: response header verification failed",
                        )));
                    }
                    this.read_state = ReadState::BodyLen;
                }
                ReadState::BodyLen => {
                    let len_bytes: Vec<u8> = this.read_raw.drain(..2).collect();
                    let clen = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
                    if clen < 16 {
                        return Poll::Ready(Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "vmess: short body chunk",
                        )));
                    }
                    this.read_state = ReadState::BodyData(clen);
                }
                ReadState::BodyData(clen) => {
                    let sealed: Vec<u8> = this.read_raw.drain(..clen).collect();
                    let nonce = chunk_nonce(&this.session.response_iv, this.read_count);
                    let plain = this
                        .session
                        .response_cipher
                        .open(&nonce, &sealed)
                        .map_err(decrypt_err)?;
                    this.read_count = this.read_count.wrapping_add(1);
                    if plain.is_empty() {
                        this.read_state = ReadState::Eof;
                        return Poll::Ready(Ok(()));
                    }
                    this.plain = plain;
                    this.plain_pos = 0;
                    this.read_state = ReadState::BodyLen;
                }
                ReadState::Eof => unreachable!(),
            }
        }
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for VmessStream<S> {
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
        if !this.shutdown_sent {
            this.queue_chunk(&[])?;
            this.shutdown_sent = true;
        }
        ready!(this.poll_drain(cx))?;
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddr};

    use super::*;
    use crate::tls::ClientFingerprint;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    fn zero_public_key_b64() -> String {
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string()
    }

    #[test]
    fn kdf_matches_known_vector() {
        // Cross-checked against an independent implementation of v2ray's nested
        // KDF (Python stdlib `hmac` with a recursive digestmod), for
        // KDF(key="Demo Key for KDF Value", path=["Demo Path for KDF Value",
        // "Demo Path for KDF Value2", "Demo Path for KDF Value3"]).
        let out = kdf(
            b"Demo Key for KDF Value",
            &[
                b"Demo Path for KDF Value",
                b"Demo Path for KDF Value2",
                b"Demo Path for KDF Value3",
            ],
        );
        let expected = [
            0xcb, 0xdd, 0x3c, 0x72, 0x07, 0xe7, 0x2f, 0x87, 0x0a, 0xb2, 0xac, 0x86, 0x5d, 0x03, 0xbc, 0x16, 0x1b, 0x90,
            0x08, 0x01, 0x6a, 0x95, 0x1e, 0x52, 0xed, 0x77, 0xfe, 0x1a, 0xfe, 0x5f, 0x68, 0xd9,
        ];
        assert_eq!(out, expected);
    }

    #[test]
    fn hmac_sha256_matches_rfc4231_case2() {
        // RFC 4231 test case 2: key="Jefe", data="what do ya want for nothing?".
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        let expected = [
            0x5b, 0xdc, 0xc1, 0x46, 0xbf, 0x60, 0x75, 0x4e, 0x6a, 0x04, 0x24, 0x26, 0x08, 0x95, 0x75, 0xc7, 0x5a, 0x00,
            0x3f, 0x08, 0x9d, 0x27, 0x39, 0x83, 0x9d, 0xec, 0x58, 0xb9, 0x64, 0xec, 0x38, 0x43,
        ];
        assert_eq!(mac, expected);
    }

    #[test]
    fn fnv1a32_matches_known_vector() {
        // FNV-1a 32-bit of "" is the offset basis; of "a" is 0xe40c292c.
        assert_eq!(fnv1a32(b""), 0x811c_9dc5);
        assert_eq!(fnv1a32(b"a"), 0xe40c_292c);
    }

    #[test]
    fn chacha_key_expands_via_md5() {
        // key = MD5(b) || MD5(MD5(b)); check against MD5 of 16 zero bytes.
        let key = chacha_key(&[0u8; 16]);
        let first: [u8; 16] = {
            let mut h = Md5::new();
            h.update([0u8; 16]);
            h.finalize().into()
        };
        assert_eq!(&key[..16], &first);
    }

    #[test]
    fn auth_id_crc_is_self_consistent() {
        // Decrypt the auth id with the same key and verify the embedded CRC32.
        let cmd_key = command_key(&[0x11u8; 16]);
        let id = auth_id(&cmd_key, 1_700_000_000);
        let key = kdf16(&cmd_key, &[KDF_SALT_AUTH_ID]);
        // AES-ECB decrypt one block.
        use aes::cipher::BlockDecrypt;
        let cipher = <Aes128 as aes::cipher::KeyInit>::new(GenericArray::from_slice(&key));
        let mut block = *GenericArray::<u8, aes::cipher::consts::U16>::from_slice(&id);
        cipher.decrypt_block(&mut block);
        let crc = crc32fast::hash(&block[..12]);
        assert_eq!(&block[12..16], &crc.to_be_bytes());
    }

    #[test]
    fn encodes_domain_command_header() {
        let iv = [1u8; 16];
        let key = [2u8; 16];
        let target = TargetAddr::Domain("example.com".to_string(), 443);
        let cmd = encode_command(&iv, &key, 0x5a, VmessCipher::Aes128Gcm, &target);

        assert_eq!(cmd[0], VERSION);
        assert_eq!(&cmd[1..17], &iv);
        assert_eq!(&cmd[17..33], &key);
        assert_eq!(cmd[33], 0x5a); // response verifier V
        assert_eq!(cmd[34], OPT_CHUNK_STREAM);
        assert_eq!(cmd[35], SEC_AES_128_GCM); // padding 0 | aes-128-gcm
        assert_eq!(cmd[36], 0); // reserved
        assert_eq!(cmd[37], CMD_TCP);
        assert_eq!(&cmd[38..40], &443u16.to_be_bytes());
        assert_eq!(cmd[40], ATYP_DOMAIN);
        assert_eq!(cmd[41], "example.com".len() as u8);
        assert_eq!(&cmd[42..53], b"example.com");
        // trailing 4-byte fnv1a checksum over everything before it.
        let body = &cmd[..cmd.len() - 4];
        assert_eq!(&cmd[cmd.len() - 4..], &fnv1a32(body).to_be_bytes());
    }

    #[test]
    fn encodes_ipv4_command_header() {
        let target = TargetAddr::Ip(SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 8443)));
        let cmd = encode_command(&[0u8; 16], &[0u8; 16], 0, VmessCipher::Chacha20Poly1305, &target);
        assert_eq!(cmd[35], SEC_CHACHA20_POLY1305);
        assert_eq!(&cmd[38..40], &8443u16.to_be_bytes());
        assert_eq!(cmd[40], ATYP_IPV4);
        assert_eq!(&cmd[41..45], &[1, 2, 3, 4]);
    }

    #[test]
    fn defaults_to_aes_gcm_plaintext() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\nuuid: b831381d-6324-4d53-ad4f-8cda48b30811\n";
        let cfg = VmessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.cipher, VmessCipher::Aes128Gcm);
        assert!(matches!(cfg.security, Security::None));
        assert!(matches!(cfg.transport, Transport::Tcp));
    }

    #[test]
    fn auto_cipher_maps_to_aes_gcm() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ncipher: auto\n";
        let cfg = VmessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.cipher, VmessCipher::Aes128Gcm);
    }

    #[test]
    fn chacha_cipher_is_parsed() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ncipher: chacha20-poly1305\n";
        let cfg = VmessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert_eq!(cfg.cipher, VmessCipher::Chacha20Poly1305);
    }

    #[test]
    fn unknown_cipher_is_rejected() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ncipher: rc4\n";
        let err = VmessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("cipher"), "got: {err}");
    }

    #[test]
    fn nonzero_alter_id_is_rejected() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\nalterId: 64\n";
        let err = VmessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("alterId"), "got: {err}");
    }

    #[test]
    fn zero_alter_id_is_accepted() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\nalterId: 0\n";
        assert!(VmessOutboundConfig::from_proxy(&parse_entry(yaml)).is_ok());
    }

    #[test]
    fn missing_uuid_is_rejected() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\n";
        let err = VmessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("uuid"), "got: {err}");
    }

    #[test]
    fn reality_opts_map_to_reality_security() {
        let yaml = format!(
            "name: v\ntype: vmess\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ntls: true\n\
             servername: www.cloudflare.com\nclient-fingerprint: chrome\n\
             reality-opts:\n  public-key: {}\n  short-id: 0123abcd\n",
            zero_public_key_b64()
        );
        let cfg = VmessOutboundConfig::from_proxy(&parse_entry(&yaml)).unwrap();
        match cfg.security {
            Security::Reality(r) => {
                assert_eq!(r.server_name, "www.cloudflare.com");
                assert_eq!(r.public_key, [0u8; 32]);
                assert_eq!(r.short_id, vec![0x01, 0x23, 0xab, 0xcd]);
                assert_eq!(r.client_fingerprint, Some(ClientFingerprint::Chrome));
            }
            other => panic!("expected REALITY security, got {other:?}"),
        }
    }

    #[test]
    fn grpc_forces_h2_alpn() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\ntls: true\n\
             network: grpc\ngrpc-opts:\n  grpc-service-name: TunService\n";
        let cfg = VmessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap();
        assert!(matches!(cfg.transport, Transport::Grpc(_)));
        match cfg.security {
            Security::Tls(tls) => assert_eq!(tls.alpn, vec!["h2".to_string()]),
            other => panic!("expected TLS security, got {other:?}"),
        }
    }

    #[test]
    fn h2_without_tls_is_rejected() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\nnetwork: h2\n";
        let err = VmessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("h2 transport requires TLS"), "got: {err}");
    }

    #[test]
    fn flow_is_rejected() {
        let yaml = "name: v\ntype: vmess\nserver: example.com\nport: 443\n\
             uuid: b831381d-6324-4d53-ad4f-8cda48b30811\nflow: xtls-rprx-vision\n";
        let err = VmessOutboundConfig::from_proxy(&parse_entry(yaml)).unwrap_err();
        assert!(err.to_string().contains("flow"), "got: {err}");
    }
}
