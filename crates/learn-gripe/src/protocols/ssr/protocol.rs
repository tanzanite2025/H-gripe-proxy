//! SSR protocol (authentication / framing) layer.

use aes::Aes128;
use aes::cipher::{BlockEncrypt, KeyInit as AesKeyInit};
use aes_gcm::aead::generic_array::GenericArray;
use anyhow::Result;
use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;

use super::crypto::{random_bytes, random_u16};

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
pub(super) enum AuthHashKind {
    Sha1,
    Md5,
}

/// Protocol layer state. Wraps application data with authentication / framing
/// before encryption, and strips it after decryption.
pub(super) enum ProtocolState {
    /// No framing — data passes through unchanged.
    Origin,
    /// auth_aes128_sha1 / auth_aes128_md5.
    AuthAes128(AuthAes128State),
    /// auth_chain_a.
    AuthChainA(AuthChainAState),
}

impl ProtocolState {
    pub(super) fn new(protocol: SsrProtocol, key: &[u8], client_iv: &[u8], _protocol_param: &str) -> Self {
        match protocol {
            SsrProtocol::Origin => ProtocolState::Origin,
            SsrProtocol::AuthAes128Sha1 => {
                ProtocolState::AuthAes128(AuthAes128State::new(AuthHashKind::Sha1, key, client_iv))
            }
            SsrProtocol::AuthAes128Md5 => {
                ProtocolState::AuthAes128(AuthAes128State::new(AuthHashKind::Md5, key, client_iv))
            }
            SsrProtocol::AuthChainA => ProtocolState::AuthChainA(AuthChainAState::new(key, client_iv)),
        }
    }

    /// Wrap `data` with protocol framing before encryption.
    pub(super) fn client_pre_encrypt(&mut self, data: &[u8]) -> Vec<u8> {
        match self {
            ProtocolState::Origin => data.to_vec(),
            ProtocolState::AuthAes128(s) => s.client_pre_encrypt(data),
            ProtocolState::AuthChainA(s) => s.client_pre_encrypt(data),
        }
    }

    /// Strip protocol framing from `data` after decryption.
    pub(super) fn client_post_decrypt(&mut self, data: &[u8]) -> Result<Vec<u8>> {
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
pub(super) struct AuthAes128State {
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
                let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(key).expect("HMAC key length");
                mac.update(data);
                mac.finalize().into_bytes().to_vec()
            }
            AuthHashKind::Md5 => {
                let mut mac = <Hmac<Md5> as Mac>::new_from_slice(key).expect("HMAC key length");
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
pub(super) struct AuthChainAState {
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
pub(super) struct Xorshift128Plus {
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
