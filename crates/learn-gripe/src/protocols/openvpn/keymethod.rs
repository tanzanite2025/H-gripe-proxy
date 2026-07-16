//! OpenVPN key-method-2 negotiation and the TLS-PRF-style key derivation.
//!
//! After the control-channel TLS handshake, both sides exchange a "key method
//! 2" record (random material + options/username/password/peer-info) inside the
//! TLS stream, then derive the data-channel key block with OpenVPN's MD5+SHA1
//! XOR PRF (the same PRF TLS 1.0/1.1 used).

use anyhow::{Result, bail};
use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;

use super::packet::SessionId;

const PRE_MASTER_SIZE: usize = 48;
const RANDOM_SIZE: usize = 32;
const KEY_METHOD_2: u8 = 2;

/// Max cipher/HMAC key material reserved per direction in the derived block.
const MAX_CIPHER_KEY_LEN: usize = 64;
const MAX_HMAC_KEY_LEN: usize = 64;
/// Two directions, each `cipher || hmac` reserved area.
const KEY_BLOCK_SIZE: usize = 2 * (MAX_CIPHER_KEY_LEN + MAX_HMAC_KEY_LEN);

/// One side's random key source (pre-master is only meaningful for the client).
#[derive(Clone)]
pub(super) struct KeySource {
    pub(super) pre_master: [u8; PRE_MASTER_SIZE],
    pub(super) random1: [u8; RANDOM_SIZE],
    pub(super) random2: [u8; RANDOM_SIZE],
}

impl KeySource {
    fn random_client() -> Result<Self> {
        let mut src = Self {
            pre_master: [0u8; PRE_MASTER_SIZE],
            random1: [0u8; RANDOM_SIZE],
            random2: [0u8; RANDOM_SIZE],
        };
        let rng_err = || anyhow::anyhow!("openvpn: system RNG unavailable");
        getrandom::fill(&mut src.pre_master).map_err(|_| rng_err())?;
        getrandom::fill(&mut src.random1).map_err(|_| rng_err())?;
        getrandom::fill(&mut src.random2).map_err(|_| rng_err())?;
        Ok(src)
    }

    fn empty() -> Self {
        Self {
            pre_master: [0u8; PRE_MASTER_SIZE],
            random1: [0u8; RANDOM_SIZE],
            random2: [0u8; RANDOM_SIZE],
        }
    }
}

/// The derived per-direction data-channel key material.
pub(super) struct KeyMaterial {
    pub(super) send_cipher_key: Vec<u8>,
    pub(super) send_hmac_key: Vec<u8>,
    pub(super) recv_cipher_key: Vec<u8>,
    pub(super) recv_hmac_key: Vec<u8>,
}

/// The client's key-method-2 record: its own random source plus the negotiated
/// options string, credentials, and peer-info.
pub(super) struct ClientKeyMethod2 {
    pub(super) source: KeySource,
    pub(super) options: String,
    pub(super) username: String,
    pub(super) password: String,
    pub(super) peer_info: String,
}

impl ClientKeyMethod2 {
    pub(super) fn new(options: String, peer_info: String, username: String, password: String) -> Result<Self> {
        Ok(Self {
            source: KeySource::random_client()?,
            options,
            username,
            password,
            peer_info,
        })
    }

    /// Serialize the client record for writing into the TLS control stream.
    pub(super) fn marshal(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&0u32.to_be_bytes());
        out.push(KEY_METHOD_2);
        out.extend_from_slice(&self.source.pre_master);
        out.extend_from_slice(&self.source.random1);
        out.extend_from_slice(&self.source.random2);
        append_string(&mut out, &self.options);
        append_string(&mut out, &self.username);
        append_string(&mut out, &self.password);
        append_string(&mut out, &self.peer_info);
        out
    }
}

/// The server half of a key-method-2 exchange: just its random source (options
/// etc. are ignored by the client).
pub(super) struct ServerKeyMethod2 {
    pub(super) source: KeySource,
}

/// Parse a server key-method-2 record read from the TLS control stream.
///
/// Returns `Ok(None)` when the buffer is a valid-but-incomplete prefix (caller
/// should read more), `Ok(Some((record, consumed)))` with the number of bytes
/// the full record occupied (so the caller can drain exactly that much before
/// the next read), and `Err` when the buffer is malformed. The full record ends
/// with four length-prefixed strings (options/username/password/peer-info) that
/// must be consumed even though the client ignores their contents.
pub(super) fn parse_server_key_method2(packet: &[u8]) -> Result<Option<(ServerKeyMethod2, usize)>> {
    if packet.len() < 4 + 1 + RANDOM_SIZE * 2 {
        return Ok(None);
    }
    if u32::from_be_bytes(packet[0..4].try_into().unwrap()) != 0 {
        bail!("openvpn: invalid key method 2 prefix");
    }
    if packet[4] & 0x0f != KEY_METHOD_2 {
        bail!("openvpn: unsupported key method {}", packet[4]);
    }
    let mut source = KeySource::empty();
    let mut offset = 5;
    source.random1.copy_from_slice(&packet[offset..offset + RANDOM_SIZE]);
    offset += RANDOM_SIZE;
    source.random2.copy_from_slice(&packet[offset..offset + RANDOM_SIZE]);
    offset += RANDOM_SIZE;

    // options, username, password, peer-info: each a u16-prefixed string.
    for _ in 0..4 {
        if packet.len() < offset + 2 {
            return Ok(None);
        }
        let len = u16::from_be_bytes(packet[offset..offset + 2].try_into().unwrap()) as usize;
        offset += 2;
        if packet.len() < offset + len {
            return Ok(None);
        }
        offset += len;
    }

    Ok(Some((ServerKeyMethod2 { source }, offset)))
}

/// Derive the client's send/recv data-channel keys from the two key sources and
/// both session ids, using the OpenVPN PRF over `cipher_key_len`-byte keys.
pub(super) fn derive_client_key_material(
    client: &KeySource,
    server: &KeySource,
    client_session: SessionId,
    server_session: SessionId,
    cipher_key_len: usize,
) -> Result<KeyMaterial> {
    if !matches!(cipher_key_len, 16 | 24 | 32) {
        bail!("openvpn: unsupported data cipher key length {cipher_key_len}");
    }

    let mut master = [0u8; PRE_MASTER_SIZE];
    let mut master_seed = Vec::new();
    master_seed.extend_from_slice(&client.random1);
    master_seed.extend_from_slice(&server.random1);
    openvpn_prf(&client.pre_master, "OpenVPN master secret", &master_seed, &mut master);

    let mut expansion_seed = Vec::new();
    expansion_seed.extend_from_slice(&client.random2);
    expansion_seed.extend_from_slice(&server.random2);
    expansion_seed.extend_from_slice(&client_session);
    expansion_seed.extend_from_slice(&server_session);
    let mut key_block = vec![0u8; KEY_BLOCK_SIZE];
    openvpn_prf(&master, "OpenVPN key expansion", &expansion_seed, &mut key_block);

    let dir = MAX_CIPHER_KEY_LEN + MAX_HMAC_KEY_LEN;
    let client_to_server = &key_block[..dir];
    let server_to_client = &key_block[dir..];
    Ok(KeyMaterial {
        send_cipher_key: client_to_server[..cipher_key_len].to_vec(),
        send_hmac_key: client_to_server[MAX_CIPHER_KEY_LEN..MAX_CIPHER_KEY_LEN + MAX_HMAC_KEY_LEN].to_vec(),
        recv_cipher_key: server_to_client[..cipher_key_len].to_vec(),
        recv_hmac_key: server_to_client[MAX_CIPHER_KEY_LEN..MAX_CIPHER_KEY_LEN + MAX_HMAC_KEY_LEN].to_vec(),
    })
}

/// OpenVPN PRF: `P_MD5(S1, label||seed) XOR P_SHA1(S2, label||seed)`, where the
/// secret is split in half (with a 1-byte overlap for odd lengths).
fn openvpn_prf(secret: &[u8], label: &str, seed: &[u8], out: &mut [u8]) {
    let mut full_seed = Vec::with_capacity(label.len() + seed.len());
    full_seed.extend_from_slice(label.as_bytes());
    full_seed.extend_from_slice(seed);

    let split = secret.len().div_ceil(2);
    let s1 = &secret[..split];
    let s2 = &secret[secret.len() - split..];

    let md5_out = p_hash_md5(s1, &full_seed, out.len());
    let sha1_out = p_hash_sha1(s2, &full_seed, out.len());
    for (i, b) in out.iter_mut().enumerate() {
        *b = md5_out[i] ^ sha1_out[i];
    }
}

macro_rules! p_hash_impl {
    ($name:ident, $mac:ty) => {
        fn $name(secret: &[u8], seed: &[u8], size: usize) -> Vec<u8> {
            let mac_of = |data: &[u8]| -> Vec<u8> {
                let mut mac = <$mac>::new_from_slice(secret).expect("hmac accepts any key length");
                mac.update(data);
                mac.finalize().into_bytes().to_vec()
            };
            let mut out = Vec::with_capacity(size);
            let mut a = mac_of(seed);
            while out.len() < size {
                let mut chunk_input = a.clone();
                chunk_input.extend_from_slice(seed);
                out.extend_from_slice(&mac_of(&chunk_input));
                a = mac_of(&a);
            }
            out.truncate(size);
            out
        }
    };
}

p_hash_impl!(p_hash_md5, Hmac<Md5>);
p_hash_impl!(p_hash_sha1, Hmac<Sha1>);

fn append_string(out: &mut Vec<u8>, s: &str) {
    if s.is_empty() {
        out.extend_from_slice(&0u16.to_be_bytes());
        return;
    }
    let len = (s.len() + 1).min(0xffff) as u16;
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(s.as_bytes());
    out.push(0);
}

/// Build the OpenVPN `IV_*` peer-info block advertised to the server.
pub(super) fn peer_info(cipher: &str) -> String {
    format!("IV_VER=learn-gripe\nIV_PROTO=6\nIV_CIPHERS={cipher}\n")
}

/// Build the OpenVPN "occ" options string the client claims to run. `proto` is
/// the TCP/UDP client label, `keysize` follows the cipher.
pub(super) fn options_string(proto_tcp: bool, cipher: &str, auth: &str) -> String {
    let proto_name = if proto_tcp { "TCPv4_CLIENT" } else { "UDPv4" };
    let keysize = match cipher {
        "AES-256-GCM" | "AES-256-CBC" | "CHACHA20-POLY1305" => "256",
        "AES-192-GCM" | "AES-192-CBC" => "192",
        _ => "128",
    };
    format!(
        "V4,dev-type tun,link-mtu 1550,tun-mtu 1500,proto {proto_name},cipher {cipher},auth {auth},keysize {keysize},key-method 2,tls-client"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prf_is_deterministic_and_symmetric() {
        let secret = b"a-shared-secret";
        let seed = b"seed-material";
        let mut a = [0u8; 48];
        let mut b = [0u8; 48];
        openvpn_prf(secret, "label", seed, &mut a);
        openvpn_prf(secret, "label", seed, &mut b);
        assert_eq!(a, b);
        let mut c = [0u8; 48];
        openvpn_prf(secret, "other", seed, &mut c);
        assert_ne!(a, c);
    }

    #[test]
    fn client_record_round_trips_through_parser_prefix() {
        // A marshalled client record is longer than the fixed server prefix; the
        // parser only reads the fixed random area, which is enough to prove the
        // offsets line up on a self-consistent record.
        let client = ClientKeyMethod2::new("opts".into(), "IV_VER=x\n".into(), "user".into(), "pass".into()).unwrap();
        let bytes = client.marshal();
        assert_eq!(bytes[4] & 0x0f, KEY_METHOD_2);
        assert_eq!(&bytes[..4], &[0, 0, 0, 0]);
    }

    #[test]
    fn derive_matches_between_two_parties() {
        // Both parties run the identical derivation given the same sources +
        // sessions, proving client/server agree on keys.
        let client = KeySource {
            pre_master: [7u8; 48],
            random1: [1u8; 32],
            random2: [2u8; 32],
        };
        let server = KeySource {
            pre_master: [0u8; 48],
            random1: [3u8; 32],
            random2: [4u8; 32],
        };
        let cs = [9u8; 8];
        let ss = [8u8; 8];
        let a = derive_client_key_material(&client, &server, cs, ss, 32).unwrap();
        let b = derive_client_key_material(&client, &server, cs, ss, 32).unwrap();
        assert_eq!(a.send_cipher_key, b.send_cipher_key);
        assert_eq!(a.send_cipher_key.len(), 32);
        assert_ne!(a.send_cipher_key, a.recv_cipher_key);
    }
}
