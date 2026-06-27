//! ShadowsocksR (SSR) outbound (TCP relay).
//!
//! SSR is the legacy fork of Shadowsocks that adds three extra layers on top of
//! the raw stream:
//!
//! 1. **Stream cipher** — legacy (non-AEAD) ciphers: `aes-128-cfb`,
//!    `aes-256-cfb`, `chacha20-ietf`, `rc4-md5`, `none`. Key derivation uses
//!    the same `EVP_BytesToKey` as classic Shadowsocks; a random IV is
//!    prepended to the stream in the clear.
//!
//! 2. **Protocol** — authentication / framing layer that wraps the encrypted
//!    payload: `origin` (pass-through), `auth_aes128_sha1`, `auth_aes128_md5`,
//!    `auth_chain_a`.
//!
//! 3. **Obfuscation** — transport-level disguise: `plain` (pass-through),
//!    `http_simple` (fake HTTP GET), `tls1.2_ticket_auth` (fake TLS handshake).
//!
//! Data flow (client write):
//! ```text
//! app data → protocol.pre_encrypt(socks5_addr + data)
//!          → stream_cipher.encrypt(protocol_output)
//!          → IV ++ encrypted  (IV only for the first write)
//!          → obfs.encode(wire_data)
//!          → TCP send
//! ```
//!
//! These are intentionally weak constructions that were deliberately excluded
//! from the AEAD-only kernel. They are re-introduced here solely to enable SSR
//! interop with existing deployments.

use std::io;
use std::pin::Pin;
use std::task::{Context as TaskContext, Poll, ready};

use aes::Aes128;
use aes::Aes256;
use aes::cipher::{BlockEncrypt, KeyInit as AesKeyInit};
use aes_gcm::aead::generic_array::GenericArray;
use anyhow::{Context, Result, bail};
use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

use crate::address::TargetAddr;
use crate::config::outbound_opts::ProxyEntry;
use crate::inbound::socks5;
use crate::outbound::BoxedStream;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Maximum bytes we buffer before flushing to the inner stream.
const MAX_WRITE_BUF: usize = 0x4000;

// ---------------------------------------------------------------------------
// Stream Cipher layer
// ---------------------------------------------------------------------------

/// SSR stream-cipher method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrCipher {
    Aes128Cfb,
    Aes256Cfb,
    Chacha20Ietf,
    Rc4Md5,
    None,
}

impl SsrCipher {
    /// Key length fed to `EVP_BytesToKey`.
    pub fn key_size(self) -> usize {
        match self {
            SsrCipher::Aes128Cfb => 16,
            SsrCipher::Aes256Cfb | SsrCipher::Chacha20Ietf => 32,
            SsrCipher::Rc4Md5 => 16,
            SsrCipher::None => 0,
        }
    }

    /// Length of the random IV prepended to the stream.
    pub fn iv_size(self) -> usize {
        match self {
            SsrCipher::Aes128Cfb | SsrCipher::Aes256Cfb | SsrCipher::Rc4Md5 => 16,
            SsrCipher::Chacha20Ietf => 12,
            SsrCipher::None => 0,
        }
    }
}

/// A stateful stream encryptor / decryptor. SSR stream ciphers are XOR-based:
/// the same operation encrypts and decrypts.
enum StreamCryptor {
    Aes128Cfb(Box<Aes128CfbState>),
    Aes256Cfb(Box<Aes256CfbState>),
    Chacha20(Box<Chacha20State>),
    Rc4(Box<Rc4State>),
    None,
}

impl StreamCryptor {
    fn new_encrypt(cipher: SsrCipher, key: &[u8], iv: &[u8]) -> Self {
        match cipher {
            SsrCipher::Aes128Cfb => StreamCryptor::Aes128Cfb(Box::new(Aes128CfbState::new(key, iv, true))),
            SsrCipher::Aes256Cfb => StreamCryptor::Aes256Cfb(Box::new(Aes256CfbState::new(key, iv, true))),
            SsrCipher::Chacha20Ietf => StreamCryptor::Chacha20(Box::new(Chacha20State::new(key, iv))),
            SsrCipher::Rc4Md5 => StreamCryptor::Rc4(Box::new(Rc4State::new(key, iv))),
            SsrCipher::None => StreamCryptor::None,
        }
    }

    fn new_decrypt(cipher: SsrCipher, key: &[u8], iv: &[u8]) -> Self {
        match cipher {
            SsrCipher::Aes128Cfb => StreamCryptor::Aes128Cfb(Box::new(Aes128CfbState::new(key, iv, false))),
            SsrCipher::Aes256Cfb => StreamCryptor::Aes256Cfb(Box::new(Aes256CfbState::new(key, iv, false))),
            SsrCipher::Chacha20Ietf => StreamCryptor::Chacha20(Box::new(Chacha20State::new(key, iv))),
            SsrCipher::Rc4Md5 => StreamCryptor::Rc4(Box::new(Rc4State::new(key, iv))),
            SsrCipher::None => StreamCryptor::None,
        }
    }

    fn update(&mut self, data: &mut [u8]) {
        match self {
            StreamCryptor::Aes128Cfb(s) => s.update(data),
            StreamCryptor::Aes256Cfb(s) => s.update(data),
            StreamCryptor::Chacha20(s) => s.update(data),
            StreamCryptor::Rc4(s) => s.update(data),
            StreamCryptor::None => {}
        }
    }
}

// -- AES-128-CFB (manual CFB-128 over the `aes` block cipher) ---------------

struct Aes128CfbState {
    cipher: Aes128,
    /// Feedback register (previous ciphertext block, or IV for the first block).
    feedback: [u8; 16],
    /// Keystream buffer for the current block.
    keystream: [u8; 16],
    /// Position within the current 16-byte keystream block.
    pos: usize,
    /// `true` for encryption, `false` for decryption.
    encrypting: bool,
}

impl Aes128CfbState {
    fn new(key: &[u8], iv: &[u8], encrypting: bool) -> Self {
        let cipher = Aes128::new(GenericArray::from_slice(&key[..16]));
        let mut feedback = [0u8; 16];
        feedback.copy_from_slice(&iv[..16]);
        Self {
            cipher,
            feedback,
            keystream: [0u8; 16],
            pos: 16, // force keystream generation on first byte
            encrypting,
        }
    }

    fn update(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            if self.pos >= 16 {
                let mut block = GenericArray::clone_from_slice(&self.feedback);
                self.cipher.encrypt_block(&mut block);
                self.keystream = block.into();
                self.pos = 0;
            }
            if self.encrypting {
                *byte ^= self.keystream[self.pos];
                self.feedback[self.pos] = *byte; // feedback = ciphertext
            } else {
                let ct = *byte;
                *byte ^= self.keystream[self.pos];
                self.feedback[self.pos] = ct; // feedback = ciphertext (input)
            }
            self.pos += 1;
        }
    }
}

// -- AES-256-CFB ------------------------------------------------------------

struct Aes256CfbState {
    cipher: Aes256,
    feedback: [u8; 16],
    keystream: [u8; 16],
    pos: usize,
    encrypting: bool,
}

impl Aes256CfbState {
    fn new(key: &[u8], iv: &[u8], encrypting: bool) -> Self {
        let cipher = Aes256::new(GenericArray::from_slice(&key[..32]));
        let mut feedback = [0u8; 16];
        feedback.copy_from_slice(&iv[..16]);
        Self {
            cipher,
            feedback,
            keystream: [0u8; 16],
            pos: 16,
            encrypting,
        }
    }

    fn update(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            if self.pos >= 16 {
                let mut block = GenericArray::clone_from_slice(&self.feedback);
                self.cipher.encrypt_block(&mut block);
                self.keystream = block.into();
                self.pos = 0;
            }
            if self.encrypting {
                *byte ^= self.keystream[self.pos];
                self.feedback[self.pos] = *byte;
            } else {
                let ct = *byte;
                *byte ^= self.keystream[self.pos];
                self.feedback[self.pos] = ct;
            }
            self.pos += 1;
        }
    }
}

// -- ChaCha20-IETF (raw, no Poly1305) --------------------------------------

struct Chacha20State {
    /// Current byte offset into the keystream (for seek-based streaming).
    byte_offset: u64,
    /// The cipher key (32 bytes).
    key: [u8; 32],
    /// The nonce (12 bytes, IETF).
    nonce: [u8; 12],
}

impl Chacha20State {
    fn new(key: &[u8], iv: &[u8]) -> Self {
        let mut k = [0u8; 32];
        k.copy_from_slice(&key[..32]);
        let mut n = [0u8; 12];
        n.copy_from_slice(&iv[..12]);
        Self {
            byte_offset: 0,
            key: k,
            nonce: n,
        }
    }

    fn update(&mut self, data: &mut [u8]) {
        use chacha20::ChaCha20;
        use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};

        let mut cipher = ChaCha20::new(
            GenericArray::from_slice(&self.key),
            GenericArray::from_slice(&self.nonce),
        );
        cipher.seek(self.byte_offset);
        cipher.apply_keystream(data);
        self.byte_offset += data.len() as u64;
    }
}

// -- RC4-MD5 ----------------------------------------------------------------

/// RC4 stream cipher keyed with `MD5(key || iv)`.
struct Rc4State {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl Rc4State {
    fn new(key: &[u8], iv: &[u8]) -> Self {
        // Derive the actual RC4 key: MD5(key || iv).
        use md5::Digest;
        let mut hasher = Md5::new();
        hasher.update(key);
        hasher.update(iv);
        let derived: [u8; 16] = hasher.finalize().into();

        // RC4 Key-Scheduling Algorithm (KSA).
        let mut s = [0u8; 256];
        for (i, byte) in s.iter_mut().enumerate() {
            *byte = i as u8;
        }
        let mut j: u8 = 0;
        for i in 0..256 {
            j = j
                .wrapping_add(s[i])
                .wrapping_add(derived[i % derived.len()]);
            s.swap(i, j as usize);
        }
        Self { s, i: 0, j: 0 }
    }

    fn update(&mut self, data: &mut [u8]) {
        // RC4 Pseudo-Random Generation Algorithm (PRGA).
        for byte in data.iter_mut() {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let k = self.s[self.s[self.i as usize].wrapping_add(self.s[self.j as usize]) as usize];
            *byte ^= k;
        }
    }
}

// ---------------------------------------------------------------------------
// Protocol layer
// ---------------------------------------------------------------------------

/// SSR protocol (authentication / framing) method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrProtocol {
    Origin,
    AuthAes128Sha1,
    AuthAes128Md5,
    AuthChainA,
}

/// Hash variant used by the auth_aes128 protocol family.
#[derive(Debug, Clone, Copy)]
enum AuthHashKind {
    Sha1,
    Md5,
}

/// Protocol layer state. Wraps application data with authentication / framing
/// before encryption, and strips it after decryption.
enum ProtocolState {
    /// No framing — data passes through unchanged.
    Origin,
    /// auth_aes128_sha1 / auth_aes128_md5.
    AuthAes128(AuthAes128State),
    /// auth_chain_a.
    AuthChainA(AuthChainAState),
}

impl ProtocolState {
    fn new(
        protocol: SsrProtocol,
        key: &[u8],
        client_iv: &[u8],
        _protocol_param: &str,
    ) -> Self {
        match protocol {
            SsrProtocol::Origin => ProtocolState::Origin,
            SsrProtocol::AuthAes128Sha1 => ProtocolState::AuthAes128(AuthAes128State::new(
                AuthHashKind::Sha1,
                key,
                client_iv,
            )),
            SsrProtocol::AuthAes128Md5 => ProtocolState::AuthAes128(AuthAes128State::new(
                AuthHashKind::Md5,
                key,
                client_iv,
            )),
            SsrProtocol::AuthChainA => ProtocolState::AuthChainA(AuthChainAState::new(key, client_iv)),
        }
    }

    /// Wrap `data` with protocol framing before encryption.
    fn client_pre_encrypt(&mut self, data: &[u8]) -> Vec<u8> {
        match self {
            ProtocolState::Origin => data.to_vec(),
            ProtocolState::AuthAes128(s) => s.client_pre_encrypt(data),
            ProtocolState::AuthChainA(s) => s.client_pre_encrypt(data),
        }
    }

    /// Strip protocol framing from `data` after decryption.
    fn client_post_decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            ProtocolState::Origin => Ok(data.to_vec()),
            ProtocolState::AuthAes128(s) => s.client_post_decrypt(data),
            ProtocolState::AuthChainA(s) => s.client_post_decrypt(data),
        }
    }
}

// -- auth_aes128 (sha1 / md5) -----------------------------------------------

/// Per-connection state for the `auth_aes128_sha1` / `auth_aes128_md5` protocol.
///
/// Wire format overview:
///
/// **First client packet** (auth request):
/// ```text
/// rnd_data(1-byte len + random) | HMAC[0:2]
/// | AES-128-ECB( uid(4) | conn_id(4) | data_len(2) | rnd_len(2) | checksum(4) )
/// | HMAC[0:4]
/// | data | random_padding | HMAC[0:4]
/// ```
///
/// **Subsequent client packets**:
/// ```text
/// data_len(2) | HMAC[0:4] | data | random_padding
/// ```
///
/// **Server response packets** (same for all):
/// ```text
/// data_len(2) | HMAC[0:4] | data | random_padding
/// ```
struct AuthAes128State {
    hash_kind: AuthHashKind,
    user_key: Vec<u8>,
    /// 4-byte user identifier.
    uid: [u8; 4],
    /// Connection counter (per-session, incrementing).
    connection_id: u32,
    /// Packet counter (client → server).
    pack_id: u32,
    /// Packet counter (server → client, for post_decrypt).
    recv_id: u32,
    /// Whether the auth header has been sent.
    has_sent_header: bool,
    /// Buffer for incomplete server response parsing.
    recv_buf: Vec<u8>,
    /// Client IV for key derivation.
    client_iv: Vec<u8>,
}

impl AuthAes128State {
    fn new(hash_kind: AuthHashKind, key: &[u8], client_iv: &[u8]) -> Self {
        let mut uid = [0u8; 4];
        random_bytes(&mut uid);

        let mut connection_id_bytes = [0u8; 4];
        random_bytes(&mut connection_id_bytes);
        let connection_id = u32::from_le_bytes(connection_id_bytes) % 0xFF_FFFF;

        Self {
            hash_kind,
            user_key: key.to_vec(),
            uid,
            connection_id,
            pack_id: 1,
            recv_id: 1,
            has_sent_header: false,
            recv_buf: Vec::new(),
            client_iv: client_iv.to_vec(),
        }
    }

    fn hmac_digest(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        match self.hash_kind {
            AuthHashKind::Sha1 => {
                let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(key)
                    .expect("HMAC key length");
                mac.update(data);
                mac.finalize().into_bytes().to_vec()
            }
            AuthHashKind::Md5 => {
                let mut mac = <Hmac<Md5> as Mac>::new_from_slice(key)
                    .expect("HMAC key length");
                mac.update(data);
                mac.finalize().into_bytes().to_vec()
            }
        }
    }

    fn client_pre_encrypt(&mut self, data: &[u8]) -> Vec<u8> {
        if !self.has_sent_header {
            self.has_sent_header = true;
            self.pack_auth_data(data)
        } else {
            self.pack_data(data)
        }
    }

    /// Build the auth-header first packet.
    fn pack_auth_data(&mut self, data: &[u8]) -> Vec<u8> {
        let data_len = data.len();
        // Random data: 4-12 bytes for small payloads.
        let rnd_len = if data_len > 400 {
            random_u16() as usize % 128
        } else {
            random_u16() as usize % 1024
        };

        let mut out = Vec::with_capacity(1 + 6 + 16 + 4 + data_len + rnd_len + 4);

        // Phase 1: random head (1-byte length indicator + random bytes).
        let rnd_data_len = 1u8.max((random_u16() % 32) as u8 + 1);
        out.push(rnd_data_len);
        let mut rnd_head = vec![0u8; rnd_data_len as usize];
        random_bytes(&mut rnd_head);
        out.extend_from_slice(&rnd_head);

        // Phase 2: HMAC check of random head (2 bytes).
        let hmac_check = self.hmac_digest(&self.user_key, &out);
        out.extend_from_slice(&hmac_check[..2]);

        // Phase 3: AES-128-ECB encrypted metadata (16 bytes).
        // Derive the AES key from user_key + client_iv.
        let aes_key = {
            use md5::Digest;
            let mut hasher = Md5::new();
            hasher.update(&self.user_key);
            hasher.update(&self.client_iv);
            let result: [u8; 16] = hasher.finalize().into();
            result
        };

        let mut meta = [0u8; 16];
        meta[0..4].copy_from_slice(&self.uid);
        meta[4..8].copy_from_slice(&self.connection_id.to_le_bytes());
        meta[8..10].copy_from_slice(&(data_len as u16).to_le_bytes());
        meta[10..12].copy_from_slice(&(rnd_len as u16).to_le_bytes());
        // Checksum of the first 12 bytes.
        let checksum = crc32fast::hash(&meta[..12]);
        meta[12..16].copy_from_slice(&checksum.to_le_bytes());

        // AES-128-ECB encrypt (single block).
        let aes = Aes128::new(GenericArray::from_slice(&aes_key));
        let mut block = GenericArray::clone_from_slice(&meta);
        aes.encrypt_block(&mut block);
        out.extend_from_slice(&block);

        // Phase 4: HMAC of everything so far (4 bytes).
        let hmac_header = self.hmac_digest(&self.user_key, &out);
        out.extend_from_slice(&hmac_header[..4]);

        // Phase 5: data.
        let data_start = out.len();
        out.extend_from_slice(data);

        // Phase 6: random padding.
        let mut padding = vec![0u8; rnd_len];
        random_bytes(&mut padding);
        out.extend_from_slice(&padding);

        // Phase 7: HMAC of data + padding (4 bytes).
        let hmac_data = self.hmac_digest(&self.user_key, &out[data_start..]);
        out.extend_from_slice(&hmac_data[..4]);

        self.pack_id += 1;
        self.connection_id = self.connection_id.wrapping_add(1);
        out
    }

    /// Pack a subsequent data packet.
    fn pack_data(&mut self, data: &[u8]) -> Vec<u8> {
        let data_len = data.len();
        let rnd_len = if data_len > 400 {
            random_u16() as usize % 128
        } else {
            random_u16() as usize % 512
        };

        let mut out = Vec::with_capacity(2 + 4 + data_len + rnd_len);

        // 2-byte data length (XOR with key material for obfuscation).
        let pack_key = {
            use md5::Digest;
            let mut h = Md5::new();
            h.update(&self.user_key);
            h.update(self.pack_id.to_le_bytes());
            let r: [u8; 16] = h.finalize().into();
            r
        };
        let len_val = (data_len as u16) ^ u16::from_le_bytes([pack_key[0], pack_key[1]]);
        out.extend_from_slice(&len_val.to_le_bytes());

        // HMAC of length (4 bytes).
        let hmac_len = self.hmac_digest(&self.user_key, &out);
        out.extend_from_slice(&hmac_len[..4]);

        // Data.
        out.extend_from_slice(data);

        // Random padding.
        let mut padding = vec![0u8; rnd_len];
        random_bytes(&mut padding);
        out.extend_from_slice(&padding);

        self.pack_id += 1;
        out
    }

    /// Parse a server response packet, stripping framing and returning payload.
    fn client_post_decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.recv_buf.extend_from_slice(data);
        let mut result = Vec::new();

        while self.recv_buf.len() >= 6 {
            // 2-byte data length (XOR-obfuscated) + 4-byte HMAC.
            let recv_key = {
                use md5::Digest;
                let mut h = Md5::new();
                h.update(&self.user_key);
                h.update(self.recv_id.to_le_bytes());
                let r: [u8; 16] = h.finalize().into();
                r
            };

            let raw_len = u16::from_le_bytes([self.recv_buf[0], self.recv_buf[1]]);
            let data_len = (raw_len ^ u16::from_le_bytes([recv_key[0], recv_key[1]])) as usize;

            // Total packet: 2 (len) + 4 (hmac) + data_len + possible padding.
            // The server may or may not add padding; we use data_len to extract.
            let packet_overhead = 6; // 2-byte len + 4-byte HMAC
            if self.recv_buf.len() < packet_overhead + data_len {
                break; // incomplete packet
            }

            // Extract data (skip the 2-byte len + 4-byte HMAC header).
            let payload = &self.recv_buf[packet_overhead..packet_overhead + data_len];
            result.extend_from_slice(payload);

            // Consume the entire packet. For simplicity, consume len + hmac + data_len.
            // The remaining bytes might include padding; since we don't know the
            // exact padding length from the server side, we consume only what we
            // decoded and let the next iteration try again.
            let consumed = packet_overhead + data_len;
            self.recv_buf.drain(..consumed);
            self.recv_id += 1;
        }

        Ok(result)
    }
}

// -- auth_chain_a -----------------------------------------------------------

/// Per-connection state for `auth_chain_a`.
///
/// Similar structure to auth_aes128 but uses a different random-length
/// generator (xorshift128plus) for padding and links packets via a chain.
struct AuthChainAState {
    user_key: Vec<u8>,
    uid: [u8; 4],
    connection_id: u32,
    pack_id: u32,
    recv_id: u32,
    has_sent_header: bool,
    recv_buf: Vec<u8>,
    client_iv: Vec<u8>,
    /// xorshift128plus state for client random length generation.
    rng: Xorshift128Plus,
    /// xorshift128plus state for server random length generation.
    recv_rng: Xorshift128Plus,
    /// Whether the recv rng has been initialized (after first server packet).
    recv_rng_init: bool,
}

/// xorshift128plus PRNG used by auth_chain_a for deterministic padding lengths.
struct Xorshift128Plus {
    s0: u64,
    s1: u64,
}

impl Xorshift128Plus {
    fn new(seed0: u64, seed1: u64) -> Self {
        Self {
            s0: if seed0 == 0 { 1 } else { seed0 },
            s1: if seed1 == 0 { 1 } else { seed1 },
        }
    }

    fn next(&mut self) -> u64 {
        let mut s1 = self.s0;
        let s0 = self.s1;
        self.s0 = s0;
        s1 ^= s1 << 23;
        s1 ^= s1 >> 17;
        s1 ^= s0;
        s1 ^= s0 >> 26;
        self.s1 = s1;
        self.s0.wrapping_add(self.s1)
    }

    /// Random padding length in the range determined by data_len.
    fn rnd_len(&mut self, data_len: usize) -> usize {
        if data_len >= 1440 {
            return 0;
        }
        let full_len = self.next() % 8589934609; // keep in range
        if data_len > 1300 {
            (full_len % 31) as usize
        } else if data_len > 900 {
            (full_len % 127) as usize
        } else if data_len > 400 {
            (full_len % 521) as usize
        } else {
            (full_len % 1021) as usize
        }
    }
}

impl AuthChainAState {
    fn new(key: &[u8], client_iv: &[u8]) -> Self {
        let mut uid = [0u8; 4];
        random_bytes(&mut uid);

        let mut cid_bytes = [0u8; 4];
        random_bytes(&mut cid_bytes);
        let connection_id = u32::from_le_bytes(cid_bytes) % 0xFF_FFFF;

        // Initialize the client RNG from key material.
        let rng_seed = {
            use md5::Digest;
            let mut h = Md5::new();
            h.update(key);
            h.update(b"auth_chain_a_client");
            let r: [u8; 16] = h.finalize().into();
            let s0 = u64::from_le_bytes(r[0..8].try_into().expect("8 bytes"));
            let s1 = u64::from_le_bytes(r[8..16].try_into().expect("8 bytes"));
            (s0, s1)
        };

        Self {
            user_key: key.to_vec(),
            uid,
            connection_id,
            pack_id: 1,
            recv_id: 1,
            has_sent_header: false,
            recv_buf: Vec::new(),
            client_iv: client_iv.to_vec(),
            rng: Xorshift128Plus::new(rng_seed.0, rng_seed.1),
            recv_rng: Xorshift128Plus::new(0, 0),
            recv_rng_init: false,
        }
    }

    fn hmac_md5(&self, key: &[u8], data: &[u8]) -> Vec<u8> {
        let mut mac = <Hmac<Md5> as Mac>::new_from_slice(key).expect("HMAC key");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    fn client_pre_encrypt(&mut self, data: &[u8]) -> Vec<u8> {
        if !self.has_sent_header {
            self.has_sent_header = true;
            self.pack_auth_data(data)
        } else {
            self.pack_data(data)
        }
    }

    fn pack_auth_data(&mut self, data: &[u8]) -> Vec<u8> {
        let data_len = data.len();
        let rnd_len = self.rng.rnd_len(data_len);

        let mut out = Vec::with_capacity(1 + 6 + 16 + 4 + data_len + rnd_len + 4);

        // Random head.
        let rnd_data_len = 1u8.max((random_u16() % 32) as u8 + 1);
        out.push(rnd_data_len);
        let mut rnd_head = vec![0u8; rnd_data_len as usize];
        random_bytes(&mut rnd_head);
        out.extend_from_slice(&rnd_head);

        // HMAC check of random head (2 bytes).
        let hmac_check = self.hmac_md5(&self.user_key, &out);
        out.extend_from_slice(&hmac_check[..2]);

        // AES-128-ECB encrypted metadata.
        let aes_key = {
            use md5::Digest;
            let mut h = Md5::new();
            h.update(&self.user_key);
            h.update(&self.client_iv);
            let r: [u8; 16] = h.finalize().into();
            r
        };

        let mut meta = [0u8; 16];
        meta[0..4].copy_from_slice(&self.uid);
        meta[4..8].copy_from_slice(&self.connection_id.to_le_bytes());
        meta[8..10].copy_from_slice(&(data_len as u16).to_le_bytes());
        meta[10..12].copy_from_slice(&(rnd_len as u16).to_le_bytes());
        let checksum = crc32fast::hash(&meta[..12]);
        meta[12..16].copy_from_slice(&checksum.to_le_bytes());

        let aes = Aes128::new(GenericArray::from_slice(&aes_key));
        let mut block = GenericArray::clone_from_slice(&meta);
        aes.encrypt_block(&mut block);
        out.extend_from_slice(&block);

        // HMAC of header (4 bytes).
        let hmac_header = self.hmac_md5(&self.user_key, &out);
        out.extend_from_slice(&hmac_header[..4]);

        // Data.
        let data_start = out.len();
        out.extend_from_slice(data);

        // Random padding.
        let mut padding = vec![0u8; rnd_len];
        random_bytes(&mut padding);
        out.extend_from_slice(&padding);

        // HMAC of data + padding (4 bytes).
        let hmac_data = self.hmac_md5(&self.user_key, &out[data_start..]);
        out.extend_from_slice(&hmac_data[..4]);

        self.pack_id += 1;
        self.connection_id = self.connection_id.wrapping_add(1);
        out
    }

    fn pack_data(&mut self, data: &[u8]) -> Vec<u8> {
        let data_len = data.len();
        let rnd_len = self.rng.rnd_len(data_len);

        let mut out = Vec::with_capacity(2 + 4 + data_len + rnd_len);

        // 2-byte data length (XOR-obfuscated).
        let pack_key = {
            use md5::Digest;
            let mut h = Md5::new();
            h.update(&self.user_key);
            h.update(self.pack_id.to_le_bytes());
            let r: [u8; 16] = h.finalize().into();
            r
        };
        let len_val = (data_len as u16) ^ u16::from_le_bytes([pack_key[0], pack_key[1]]);
        out.extend_from_slice(&len_val.to_le_bytes());

        // HMAC of length (4 bytes).
        let hmac_len = self.hmac_md5(&self.user_key, &out);
        out.extend_from_slice(&hmac_len[..4]);

        // Data.
        out.extend_from_slice(data);

        // Random padding (deterministic length from xorshift RNG).
        let mut padding = vec![0u8; rnd_len];
        random_bytes(&mut padding);
        out.extend_from_slice(&padding);

        self.pack_id += 1;
        out
    }

    fn client_post_decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.recv_buf.extend_from_slice(data);

        if !self.recv_rng_init {
            // Initialize server RNG from key material.
            use md5::Digest;
            let mut h = Md5::new();
            h.update(&self.user_key);
            h.update(b"auth_chain_a_server");
            let r: [u8; 16] = h.finalize().into();
            let s0 = u64::from_le_bytes(r[0..8].try_into().expect("8 bytes"));
            let s1 = u64::from_le_bytes(r[8..16].try_into().expect("8 bytes"));
            self.recv_rng = Xorshift128Plus::new(s0, s1);
            self.recv_rng_init = true;
        }

        let mut result = Vec::new();

        while self.recv_buf.len() >= 6 {
            let recv_key = {
                use md5::Digest;
                let mut h = Md5::new();
                h.update(&self.user_key);
                h.update(self.recv_id.to_le_bytes());
                let r: [u8; 16] = h.finalize().into();
                r
            };

            let raw_len = u16::from_le_bytes([self.recv_buf[0], self.recv_buf[1]]);
            let data_len = (raw_len ^ u16::from_le_bytes([recv_key[0], recv_key[1]])) as usize;
            let rnd_len = self.recv_rng.rnd_len(data_len);

            let total = 6 + data_len + rnd_len;
            if self.recv_buf.len() < total {
                break;
            }

            let payload = &self.recv_buf[6..6 + data_len];
            result.extend_from_slice(payload);
            self.recv_buf.drain(..total);
            self.recv_id += 1;
        }

        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Obfuscation layer
// ---------------------------------------------------------------------------

/// SSR obfuscation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrObfs {
    Plain,
    HttpSimple,
    Tls12TicketAuth,
}

/// Obfuscation layer state. Wraps the first packet in a disguise (HTTP GET /
/// TLS Client Hello) and passes subsequent packets through.
enum ObfsState {
    Plain,
    HttpSimple(HttpSimpleState),
    Tls12TicketAuth(Tls12TicketAuthState),
}

impl ObfsState {
    fn new(obfs: SsrObfs, server: &str, port: u16, obfs_param: &str) -> Self {
        match obfs {
            SsrObfs::Plain => ObfsState::Plain,
            SsrObfs::HttpSimple => ObfsState::HttpSimple(HttpSimpleState::new(server, port, obfs_param)),
            SsrObfs::Tls12TicketAuth => {
                ObfsState::Tls12TicketAuth(Tls12TicketAuthState::new(server, obfs_param))
            }
        }
    }

    /// Encode outgoing data (may wrap the first packet in HTTP/TLS headers).
    fn client_encode(&mut self, data: &[u8]) -> Vec<u8> {
        match self {
            ObfsState::Plain => data.to_vec(),
            ObfsState::HttpSimple(s) => s.client_encode(data),
            ObfsState::Tls12TicketAuth(s) => s.client_encode(data),
        }
    }

    /// Decode incoming data (strip HTTP/TLS framing from the first response).
    /// Returns the decoded data.
    fn client_decode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            ObfsState::Plain => Ok(data.to_vec()),
            ObfsState::HttpSimple(s) => s.client_decode(data),
            ObfsState::Tls12TicketAuth(s) => s.client_decode(data),
        }
    }
}

// -- http_simple ------------------------------------------------------------

/// Disguises the first packet as an HTTP GET request.
struct HttpSimpleState {
    host: String,
    port: u16,
    has_sent_header: bool,
    has_recv_header: bool,
    recv_buf: Vec<u8>,
}

impl HttpSimpleState {
    fn new(server: &str, port: u16, obfs_param: &str) -> Self {
        let host = if obfs_param.is_empty() {
            server.to_string()
        } else {
            obfs_param.to_string()
        };
        Self {
            host,
            port,
            has_sent_header: false,
            has_recv_header: false,
            recv_buf: Vec::new(),
        }
    }

    fn client_encode(&mut self, data: &[u8]) -> Vec<u8> {
        if self.has_sent_header {
            return data.to_vec();
        }
        self.has_sent_header = true;

        let port_str = if self.port == 80 {
            String::new()
        } else {
            format!(":{}", self.port)
        };

        // Encode first ≤64 bytes of data as hex in the URI path.
        let head_size = data.len().min(64);
        let hex_path: String = data[..head_size]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();

        let http_header = format!(
            "GET /{hex_path} HTTP/1.1\r\n\
             Host: {host}{port}\r\n\
             User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36\r\n\
             Accept: text/html,application/xhtml+xml,*/*;q=0.8\r\n\
             Accept-Language: en-US,en;q=0.8\r\n\
             Accept-Encoding: gzip, deflate\r\n\
             DNT: 1\r\n\
             Connection: keep-alive\r\n\
             \r\n",
            hex_path = hex_path,
            host = self.host,
            port = port_str,
        );

        let mut out = Vec::with_capacity(http_header.len() + data.len() - head_size);
        out.extend_from_slice(http_header.as_bytes());
        out.extend_from_slice(&data[head_size..]);
        out
    }

    fn client_decode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if self.has_recv_header {
            return Ok(data.to_vec());
        }
        self.recv_buf.extend_from_slice(data);

        // Look for the end of the HTTP response header (\r\n\r\n).
        if let Some(pos) = find_header_end(&self.recv_buf) {
            self.has_recv_header = true;
            let body = self.recv_buf[pos + 4..].to_vec();
            self.recv_buf.clear();
            Ok(body)
        } else {
            Ok(Vec::new()) // need more data
        }
    }
}

/// Find `\r\n\r\n` in the buffer.
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

// -- tls1.2_ticket_auth -----------------------------------------------------

/// Disguises the first packet as a TLS 1.2 Client Hello with a session ticket.
struct Tls12TicketAuthState {
    host: String,
    has_sent_header: bool,
    has_recv_header: bool,
    recv_buf: Vec<u8>,
}

impl Tls12TicketAuthState {
    fn new(server: &str, obfs_param: &str) -> Self {
        let host = if obfs_param.is_empty() {
            server.to_string()
        } else {
            obfs_param.to_string()
        };
        Self {
            host,
            has_sent_header: false,
            has_recv_header: false,
            recv_buf: Vec::new(),
        }
    }

    fn client_encode(&mut self, data: &[u8]) -> Vec<u8> {
        if self.has_sent_header {
            // Subsequent packets: wrap as TLS Application Data.
            return self.pack_tls_app_data(data);
        }
        self.has_sent_header = true;
        self.build_client_hello(data)
    }

    fn client_decode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if self.has_recv_header {
            // Subsequent packets: unwrap TLS Application Data.
            return self.unpack_tls_records(data);
        }
        self.recv_buf.extend_from_slice(data);

        // Look for TLS records. The first response is a Server Hello
        // (type 0x16) followed by Change Cipher Spec (0x14). We skip
        // all TLS handshake records and return Application Data (0x17).
        self.try_parse_server_response()
    }

    /// Build a fake TLS 1.2 Client Hello with the data as a session ticket.
    fn build_client_hello(&self, data: &[u8]) -> Vec<u8> {
        // SNI extension.
        let sni = self.host.as_bytes();
        let sni_ext_len = 5 + sni.len(); // type(1) + name_len(2) + name_list_len(2)

        // Session ticket extension: the actual encrypted data.
        let ticket_data = data;
        let ticket_ext_len = ticket_data.len();

        // Extensions total length.
        let extensions_len = 4 + sni_ext_len + 4 + ticket_ext_len;

        // Client Hello body.
        let mut hello = Vec::with_capacity(128 + extensions_len);
        // Protocol version: TLS 1.2.
        hello.extend_from_slice(&[0x03, 0x03]);
        // Random (32 bytes).
        let mut random = [0u8; 32];
        random_bytes(&mut random);
        hello.extend_from_slice(&random);
        // Session ID length + session ID (32 bytes).
        hello.push(32);
        let mut session_id = [0u8; 32];
        random_bytes(&mut session_id);
        hello.extend_from_slice(&session_id);
        // Cipher suites (2 suites).
        hello.extend_from_slice(&[0x00, 0x04]); // length
        hello.extend_from_slice(&[0xc0, 0x2b]); // TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
        hello.extend_from_slice(&[0xc0, 0x2f]); // TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
        // Compression methods.
        hello.push(0x01);
        hello.push(0x00); // null compression
        // Extensions length.
        hello.extend_from_slice(&(extensions_len as u16).to_be_bytes());
        // SNI extension (type 0x0000).
        hello.extend_from_slice(&[0x00, 0x00]); // ext type
        hello.extend_from_slice(&((sni_ext_len) as u16).to_be_bytes());
        hello.extend_from_slice(&((sni_ext_len - 2) as u16).to_be_bytes()); // list len
        hello.push(0x00); // host name type
        hello.extend_from_slice(&(sni.len() as u16).to_be_bytes());
        hello.extend_from_slice(sni);
        // Session ticket extension (type 0x0023).
        hello.extend_from_slice(&[0x00, 0x23]); // ext type
        hello.extend_from_slice(&(ticket_ext_len as u16).to_be_bytes());
        hello.extend_from_slice(ticket_data);

        // Wrap in TLS handshake (Client Hello = 0x01).
        let mut handshake = Vec::with_capacity(4 + hello.len());
        handshake.push(0x01); // Client Hello
        // 3-byte length.
        let hl = hello.len();
        handshake.push((hl >> 16) as u8);
        handshake.push((hl >> 8) as u8);
        handshake.push(hl as u8);
        handshake.extend_from_slice(&hello);

        // Wrap in TLS record (Handshake = 0x16).
        let mut record = Vec::with_capacity(5 + handshake.len());
        record.push(0x16); // content type: Handshake
        record.extend_from_slice(&[0x03, 0x01]); // version: TLS 1.0 (for compat)
        record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
        record.extend_from_slice(&handshake);

        record
    }

    /// Wrap data as a TLS Application Data record.
    fn pack_tls_app_data(&self, data: &[u8]) -> Vec<u8> {
        let mut record = Vec::with_capacity(5 + data.len());
        record.push(0x17); // content type: Application Data
        record.extend_from_slice(&[0x03, 0x03]); // version: TLS 1.2
        record.extend_from_slice(&(data.len() as u16).to_be_bytes());
        record.extend_from_slice(data);
        record
    }

    /// Try to parse the TLS server response. Skip handshake records, return
    /// Application Data payload.
    fn try_parse_server_response(&mut self) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut offset = 0;

        while offset + 5 <= self.recv_buf.len() {
            let content_type = self.recv_buf[offset];
            let record_len = u16::from_be_bytes([self.recv_buf[offset + 3], self.recv_buf[offset + 4]]) as usize;

            if offset + 5 + record_len > self.recv_buf.len() {
                break; // incomplete record
            }

            if content_type == 0x17 {
                // Application Data — this is our payload.
                result.extend_from_slice(&self.recv_buf[offset + 5..offset + 5 + record_len]);
                self.has_recv_header = true;
            }
            // Skip handshake (0x16) and change cipher spec (0x14) records.
            offset += 5 + record_len;
        }

        // Consume processed bytes.
        if offset > 0 {
            self.recv_buf.drain(..offset);
        }
        Ok(result)
    }

    /// Unwrap TLS Application Data records.
    fn unpack_tls_records(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.recv_buf.extend_from_slice(data);
        let mut result = Vec::new();
        let mut offset = 0;

        while offset + 5 <= self.recv_buf.len() {
            let content_type = self.recv_buf[offset];
            let record_len = u16::from_be_bytes([self.recv_buf[offset + 3], self.recv_buf[offset + 4]]) as usize;

            if offset + 5 + record_len > self.recv_buf.len() {
                break;
            }

            if content_type == 0x17 {
                result.extend_from_slice(&self.recv_buf[offset + 5..offset + 5 + record_len]);
            }
            offset += 5 + record_len;
        }

        if offset > 0 {
            self.recv_buf.drain(..offset);
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Config + connect
// ---------------------------------------------------------------------------

/// Fully-resolved ShadowsocksR outbound parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SsrOutboundConfig {
    pub server: String,
    pub port: u16,
    pub cipher: SsrCipher,
    pub key: Vec<u8>,
    pub protocol: SsrProtocol,
    pub protocol_param: String,
    pub obfs: SsrObfs,
    pub obfs_param: String,
}

impl SsrOutboundConfig {
    /// Build from a parsed `ssr` proxy entry.
    pub fn from_proxy(entry: &ProxyEntry) -> Result<Self> {
        let opts = &entry.options;
        let server = opts
            .server
            .clone()
            .filter(|s| !s.is_empty())
            .context("ssr: missing server")?;
        let port = opts.port.context("ssr: missing port")?;
        let password = opts
            .password
            .as_deref()
            .filter(|s| !s.is_empty())
            .context("ssr: missing password")?;

        let cipher = match opts.cipher.as_deref() {
            Some("aes-128-cfb") => SsrCipher::Aes128Cfb,
            Some("aes-256-cfb") => SsrCipher::Aes256Cfb,
            Some("chacha20-ietf") => SsrCipher::Chacha20Ietf,
            Some("rc4-md5") => SsrCipher::Rc4Md5,
            Some("none") => SsrCipher::None,
            None | Some("") => bail!("ssr: missing cipher"),
            Some(other) => bail!(
                "ssr: cipher {other:?} not supported \
                 (use aes-128-cfb / aes-256-cfb / chacha20-ietf / rc4-md5 / none)"
            ),
        };

        let protocol = match opts.protocol.as_deref() {
            Some("origin") | None | Some("") => SsrProtocol::Origin,
            Some("auth_aes128_sha1") => SsrProtocol::AuthAes128Sha1,
            Some("auth_aes128_md5") => SsrProtocol::AuthAes128Md5,
            Some("auth_chain_a") => SsrProtocol::AuthChainA,
            Some(other) => bail!(
                "ssr: protocol {other:?} not supported \
                 (use origin / auth_aes128_sha1 / auth_aes128_md5 / auth_chain_a)"
            ),
        };

        let obfs = match opts.obfs.as_deref() {
            Some("plain") | None | Some("") => SsrObfs::Plain,
            Some("http_simple") => SsrObfs::HttpSimple,
            Some("tls1.2_ticket_auth") => SsrObfs::Tls12TicketAuth,
            Some(other) => bail!(
                "ssr: obfs {other:?} not supported \
                 (use plain / http_simple / tls1.2_ticket_auth)"
            ),
        };

        let key = evp_bytes_to_key(password.as_bytes(), cipher.key_size());

        let protocol_param = opts.protocol_param.clone().unwrap_or_default();
        let obfs_param = opts.obfs_param.clone().unwrap_or_default();

        Ok(Self {
            server,
            port,
            cipher,
            key,
            protocol,
            protocol_param,
            obfs,
            obfs_param,
        })
    }
}

/// Connect a ShadowsocksR outbound to `target` and return a relay-ready stream.
pub async fn connect(config: &SsrOutboundConfig, target: &TargetAddr) -> Result<BoxedStream> {
    let transport: BoxedStream = Box::new(
        TcpStream::connect((config.server.as_str(), config.port))
            .await
            .with_context(|| format!("ssr: connect {}:{}", config.server, config.port))?,
    );

    // Generate random client IV.
    let iv_len = config.cipher.iv_size();
    let mut client_iv = vec![0u8; iv_len];
    random_bytes(&mut client_iv);

    // Create the write-side stream cipher (encrypt).
    let write_cipher = StreamCryptor::new_encrypt(config.cipher, &config.key, &client_iv);

    // Create the protocol layer.
    let protocol = ProtocolState::new(
        config.protocol,
        &config.key,
        &client_iv,
        &config.protocol_param,
    );

    // Create the obfuscation layer.
    let obfs = ObfsState::new(config.obfs, &config.server, config.port, &config.obfs_param);

    // Prepare the first write: IV + encrypted(protocol(socks5_addr)).
    let mut addr_buf = Vec::with_capacity(1 + 256 + 2);
    socks5::encode_address(&mut addr_buf, target);

    let stream = SsrStream::new(
        transport,
        config.cipher,
        config.key.clone(),
        write_cipher,
        client_iv,
        protocol,
        obfs,
        addr_buf,
    );

    Ok(Box::new(stream))
}

// ---------------------------------------------------------------------------
// SsrStream — AsyncRead + AsyncWrite
// ---------------------------------------------------------------------------

/// Wraps the raw TCP transport in the SSR three-layer stack.
struct SsrStream {
    inner: BoxedStream,
    cipher_kind: SsrCipher,
    key: Vec<u8>,
    // Write side.
    write_cipher: StreamCryptor,
    write_buf: Vec<u8>,
    write_pos: usize,
    /// Whether the client IV has been sent.
    iv_sent: bool,
    /// Client IV (prepended to the first write).
    client_iv: Vec<u8>,
    // Read side.
    read_cipher: Option<StreamCryptor>,
    read_raw: Vec<u8>,
    /// Whether the server IV has been read.
    iv_read: bool,
    plain: Vec<u8>,
    plain_pos: usize,
    // Protocol and obfs layers.
    protocol: ProtocolState,
    obfs: ObfsState,
    /// Initial payload (socks5 addr) queued for the first write.
    pending_addr: Option<Vec<u8>>,
}

impl SsrStream {
    #[allow(clippy::too_many_arguments)]
    fn new(
        inner: BoxedStream,
        cipher_kind: SsrCipher,
        key: Vec<u8>,
        write_cipher: StreamCryptor,
        client_iv: Vec<u8>,
        protocol: ProtocolState,
        obfs: ObfsState,
        addr_payload: Vec<u8>,
    ) -> Self {
        Self {
            inner,
            cipher_kind,
            key,
            write_cipher,
            write_buf: Vec::new(),
            write_pos: 0,
            iv_sent: false,
            client_iv,
            read_cipher: None,
            read_raw: Vec::new(),
            iv_read: false,
            plain: Vec::new(),
            plain_pos: 0,
            protocol,
            obfs,
            pending_addr: Some(addr_payload),
        }
    }

    /// Flush pending sealed bytes to the inner stream.
    fn poll_drain(&mut self, cx: &mut TaskContext<'_>) -> Poll<io::Result<()>> {
        while self.write_pos < self.write_buf.len() {
            let n = ready!(Pin::new(&mut self.inner).poll_write(cx, &self.write_buf[self.write_pos..]))?;
            if n == 0 {
                return Poll::Ready(Err(io::Error::new(io::ErrorKind::WriteZero, "ssr: write zero")));
            }
            self.write_pos += n;
        }
        self.write_buf.clear();
        self.write_pos = 0;
        Poll::Ready(Ok(()))
    }

    /// Encrypt and queue data for writing through the SSR stack.
    fn queue_write(&mut self, data: &[u8]) -> io::Result<()> {
        // Protocol layer: wrap data with auth framing.
        let protocol_data = self.protocol.client_pre_encrypt(data);

        // Stream cipher: encrypt.
        let mut encrypted = protocol_data;
        self.write_cipher.update(&mut encrypted);

        // Prepend IV if this is the first write.
        let wire_data = if !self.iv_sent {
            self.iv_sent = true;
            let mut out = Vec::with_capacity(self.client_iv.len() + encrypted.len());
            out.extend_from_slice(&self.client_iv);
            out.extend_from_slice(&encrypted);
            out
        } else {
            encrypted
        };

        // Obfs layer: wrap.
        let obfs_data = self.obfs.client_encode(&wire_data);

        self.write_buf = obfs_data;
        self.write_pos = 0;
        Ok(())
    }
}

impl AsyncRead for SsrStream {
    fn poll_read(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &mut ReadBuf<'_>) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        loop {
            // Return buffered plaintext.
            if this.plain_pos < this.plain.len() {
                let n = buf.remaining().min(this.plain.len() - this.plain_pos);
                buf.put_slice(&this.plain[this.plain_pos..this.plain_pos + n]);
                this.plain_pos += n;
                return Poll::Ready(Ok(()));
            }

            // Read more raw data from the transport.
            let mut scratch = [0u8; 8192];
            let mut read_buf = ReadBuf::new(&mut scratch);
            ready!(Pin::new(&mut this.inner).poll_read(cx, &mut read_buf))?;
            let filled = read_buf.filled();
            if filled.is_empty() {
                return Poll::Ready(Ok(())); // EOF
            }

            // Obfs decode.
            let decoded = this
                .obfs
                .client_decode(filled)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

            if decoded.is_empty() {
                continue; // obfs needs more data (e.g., incomplete HTTP header)
            }

            this.read_raw.extend_from_slice(&decoded);

            // Extract server IV if not yet read.
            if !this.iv_read {
                let iv_len = this.cipher_kind.iv_size();
                if this.read_raw.len() < iv_len {
                    continue; // need more data for IV
                }
                let server_iv: Vec<u8> = this.read_raw.drain(..iv_len).collect();
                this.read_cipher = Some(StreamCryptor::new_decrypt(
                    this.cipher_kind,
                    &this.key,
                    &server_iv,
                ));
                this.iv_read = true;
                if this.read_raw.is_empty() {
                    continue;
                }
            }

            // Decrypt.
            let Some(cipher) = this.read_cipher.as_mut() else {
                return Poll::Ready(Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "ssr: read cipher not initialized",
                )));
            };
            let mut decrypted = std::mem::take(&mut this.read_raw);
            cipher.update(&mut decrypted);

            // Protocol post-decrypt: strip framing.
            let payload = this
                .protocol
                .client_post_decrypt(&decrypted)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;

            if payload.is_empty() {
                continue; // protocol layer buffering (need more data)
            }

            this.plain = payload;
            this.plain_pos = 0;
        }
    }
}

impl AsyncWrite for SsrStream {
    fn poll_write(self: Pin<&mut Self>, cx: &mut TaskContext<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
        let this = self.get_mut();
        ready!(this.poll_drain(cx))?;

        // On the first write, send the queued target address alongside user data.
        if let Some(addr) = this.pending_addr.take() {
            let mut combined = Vec::with_capacity(addr.len() + buf.len());
            combined.extend_from_slice(&addr);
            combined.extend_from_slice(buf);
            this.queue_write(&combined)?;
            // Eagerly start draining.
            if let Poll::Ready(Err(e)) = this.poll_drain(cx) {
                return Poll::Ready(Err(e));
            }
            return Poll::Ready(Ok(buf.len()));
        }

        if buf.is_empty() {
            return Poll::Ready(Ok(0));
        }
        let take = buf.len().min(MAX_WRITE_BUF);
        this.queue_write(&buf[..take])?;
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
        Pin::new(&mut this.inner).poll_shutdown(cx)
    }
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

/// `EVP_BytesToKey` key derivation (MD5-based, shared with classic Shadowsocks).
fn evp_bytes_to_key(password: &[u8], key_len: usize) -> Vec<u8> {
    use md5::Digest;
    let mut key = Vec::with_capacity(key_len);
    let mut prev = Vec::new();
    while key.len() < key_len {
        let mut hasher = Md5::new();
        hasher.update(&prev);
        hasher.update(password);
        let hash: [u8; 16] = hasher.finalize().into();
        key.extend_from_slice(&hash);
        prev = hash.to_vec();
    }
    key.truncate(key_len);
    key
}

/// Fill `buf` with cryptographically secure random bytes.
fn random_bytes(buf: &mut [u8]) {
    if buf.is_empty() {
        return;
    }
    if getrandom::fill(buf).is_err() {
        panic!("ssr: system RNG unavailable");
    }
}

/// Return a random u16.
fn random_u16() -> u16 {
    let mut buf = [0u8; 2];
    random_bytes(&mut buf);
    u16::from_le_bytes(buf)
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::outbound_opts::ProxyEntry;

    fn parse_entry(yaml: &str) -> ProxyEntry {
        serde_yaml_ng::from_str(yaml).expect("parse proxy entry")
    }

    #[test]
    fn parses_ssr_entry_with_all_fields() {
        let entry = parse_entry(
            "name: s\ntype: ssr\nserver: example.com\nport: 443\n\
             cipher: aes-128-cfb\npassword: secret\n\
             protocol: auth_aes128_sha1\nprotocol-param: param1\n\
             obfs: http_simple\nobfs-param: www.example.com\n",
        );
        let config = SsrOutboundConfig::from_proxy(&entry).expect("valid ssr config");
        assert_eq!(config.server, "example.com");
        assert_eq!(config.port, 443);
        assert_eq!(config.cipher, SsrCipher::Aes128Cfb);
        assert_eq!(config.protocol, SsrProtocol::AuthAes128Sha1);
        assert_eq!(config.protocol_param, "param1");
        assert_eq!(config.obfs, SsrObfs::HttpSimple);
        assert_eq!(config.obfs_param, "www.example.com");
    }

    #[test]
    fn parses_ssr_entry_defaults() {
        let entry = parse_entry(
            "name: s\ntype: ssr\nserver: s\nport: 1\ncipher: none\npassword: p\n",
        );
        let config = SsrOutboundConfig::from_proxy(&entry).expect("valid");
        assert_eq!(config.cipher, SsrCipher::None);
        assert_eq!(config.protocol, SsrProtocol::Origin);
        assert_eq!(config.obfs, SsrObfs::Plain);
    }

    #[test]
    fn rejects_unsupported_cipher() {
        let entry = parse_entry(
            "name: s\ntype: ssr\nserver: s\nport: 1\ncipher: aes-256-gcm\npassword: p\n",
        );
        let err = SsrOutboundConfig::from_proxy(&entry).unwrap_err();
        assert!(err.to_string().contains("not supported"), "{err}");
    }

    #[test]
    fn rejects_unsupported_protocol() {
        let entry = parse_entry(
            "name: s\ntype: ssr\nserver: s\nport: 1\ncipher: none\npassword: p\n\
             protocol: auth_sha1_v4\n",
        );
        let err = SsrOutboundConfig::from_proxy(&entry).unwrap_err();
        assert!(err.to_string().contains("not supported"), "{err}");
    }

    #[test]
    fn evp_bytes_to_key_known_vector() {
        // "password" with 16-byte key = MD5("password").
        let key = evp_bytes_to_key(b"password", 16);
        assert_eq!(key.len(), 16);
        // MD5("password") = 5f4dcc3b5aa765d61d8327deb882cf99
        assert_eq!(
            key,
            [0x5f, 0x4d, 0xcc, 0x3b, 0x5a, 0xa7, 0x65, 0xd6, 0x1d, 0x83, 0x27, 0xde, 0xb8, 0x82, 0xcf, 0x99]
        );
    }

    #[test]
    fn stream_cipher_aes128cfb_roundtrip() {
        let key = evp_bytes_to_key(b"test", 16);
        let iv = [0u8; 16];
        let original = b"Hello, SSR!".to_vec();

        let mut data = original.clone();
        let mut enc = StreamCryptor::new_encrypt(SsrCipher::Aes128Cfb, &key, &iv);
        enc.update(&mut data);

        // Data should be different from original.
        assert_ne!(data, original);

        // Decrypt should recover original.
        let mut dec = StreamCryptor::new_decrypt(SsrCipher::Aes128Cfb, &key, &iv);
        dec.update(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn stream_cipher_aes256cfb_roundtrip() {
        let key = evp_bytes_to_key(b"test", 32);
        let iv = [0u8; 16];
        let original = b"AES-256-CFB test data".to_vec();

        let mut data = original.clone();
        let mut enc = StreamCryptor::new_encrypt(SsrCipher::Aes256Cfb, &key, &iv);
        enc.update(&mut data);
        assert_ne!(data, original);

        let mut dec = StreamCryptor::new_decrypt(SsrCipher::Aes256Cfb, &key, &iv);
        dec.update(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn stream_cipher_chacha20_roundtrip() {
        let key = evp_bytes_to_key(b"test", 32);
        let iv = [0u8; 12];
        let original = b"ChaCha20 test".to_vec();

        let mut data = original.clone();
        let mut enc = StreamCryptor::new_encrypt(SsrCipher::Chacha20Ietf, &key, &iv);
        enc.update(&mut data);
        assert_ne!(data, original);

        let mut dec = StreamCryptor::new_decrypt(SsrCipher::Chacha20Ietf, &key, &iv);
        dec.update(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn stream_cipher_rc4md5_roundtrip() {
        let key = evp_bytes_to_key(b"test", 16);
        let iv = [1u8; 16];
        let original = b"RC4-MD5 test".to_vec();

        let mut data = original.clone();
        let mut enc = StreamCryptor::new_encrypt(SsrCipher::Rc4Md5, &key, &iv);
        enc.update(&mut data);
        assert_ne!(data, original);

        let mut dec = StreamCryptor::new_decrypt(SsrCipher::Rc4Md5, &key, &iv);
        dec.update(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn stream_cipher_none_passthrough() {
        let original = b"plaintext".to_vec();
        let mut data = original.clone();
        let mut enc = StreamCryptor::new_encrypt(SsrCipher::None, &[], &[]);
        enc.update(&mut data);
        assert_eq!(data, original);
    }

    #[test]
    fn stream_cipher_streaming_consistency() {
        // Encrypting in one call vs two calls should produce the same output.
        let key = evp_bytes_to_key(b"stream", 16);
        let iv = [2u8; 16];
        let data = b"ABCDEFGHIJKLMNOP1234567890abcdef";

        // One-shot.
        let mut one_shot = data.to_vec();
        let mut enc1 = StreamCryptor::new_encrypt(SsrCipher::Aes128Cfb, &key, &iv);
        enc1.update(&mut one_shot);

        // Split.
        let mut part1 = data[..16].to_vec();
        let mut part2 = data[16..].to_vec();
        let mut enc2 = StreamCryptor::new_encrypt(SsrCipher::Aes128Cfb, &key, &iv);
        enc2.update(&mut part1);
        enc2.update(&mut part2);
        let mut split_result = part1;
        split_result.extend_from_slice(&part2);

        assert_eq!(one_shot, split_result);
    }

    #[test]
    fn http_simple_obfs_encode_decode() {
        let mut obfs = HttpSimpleState::new("example.com", 80, "");
        let data = b"hello world";
        let encoded = obfs.client_encode(data);
        assert!(encoded.starts_with(b"GET /"));
        assert!(encoded.windows(4).any(|w| w == b"\r\n\r\n"));

        // Second call should pass through.
        let data2 = b"more data";
        let encoded2 = obfs.client_encode(data2);
        assert_eq!(encoded2, data2);
    }

    #[test]
    fn tls12_ticket_auth_obfs_encode() {
        let mut obfs = Tls12TicketAuthState::new("example.com", "");
        let data = b"test payload";
        let encoded = obfs.client_encode(data);
        // Should start with TLS record header: 0x16 (Handshake), 0x03 0x01 (TLS 1.0).
        assert_eq!(encoded[0], 0x16);
        assert_eq!(encoded[1], 0x03);
        assert_eq!(encoded[2], 0x01);

        // Second call: TLS Application Data (0x17).
        let data2 = b"more";
        let encoded2 = obfs.client_encode(data2);
        assert_eq!(encoded2[0], 0x17);
    }
}
