//! SSR UDP relay.
//
// SSR UDP is fundamentally different from the TCP stack above: each datagram is
// encrypted independently (a fresh random IV + a one-shot stream cipher per
// packet, never a continuous keystream) and the obfuscation layer does not
// apply at all — only the stream cipher and the protocol layer's UDP framing
// take part. The on-wire format mirrors upstream shadowsocksr's `encrypt_all` /
// `*_udp_pre_encrypt` / `*_udp_post_decrypt`:
//
// ```text
// send: framed = protocol_udp_pre(socks5_addr(target) ++ payload)
//       wire   = iv(random) ++ stream_cipher(key, iv).encrypt(framed)
// recv: framed = stream_cipher(key, datagram[..iv_len]).decrypt(datagram[iv_len..])
//       inner  = protocol_udp_post(framed)        // strips/verifies auth tags
//       payload = inner[after socks5_addr ..]
// ```

use anyhow::{Context, Result, bail};
use tokio::net::{UdpSocket, lookup_host};

use crate::address::TargetAddr;
use crate::inbound::socks5;

use super::SsrOutboundConfig;
use super::cipher::{SsrCipher, StreamCryptor};
use super::crypto::{base64_encode, hmac_digest, hmac_md5, random_bytes, rc4_apply};
use super::protocol::{AuthHashKind, SsrProtocol};

/// A connected SSR UDP association: a single OS UDP socket to the SSR server
/// that seals each datagram for `target` and opens each reply.
pub struct SsrUdp {
    socket: UdpSocket,
    cipher: SsrCipher,
    key: Vec<u8>,
    protocol: SsrProtocol,
    target: TargetAddr,
    /// Random 4-byte user id, fixed for the association; consulted only by the
    /// `auth_*` protocols (single-user mode keys off the master key regardless).
    user_id: [u8; 4],
}

impl SsrUdp {
    /// Resolve the SSR server, bind a UDP socket and connect it. `target` is the
    /// eventual destination sealed into every datagram sent on this socket.
    pub async fn connect(config: &SsrOutboundConfig, target: &TargetAddr) -> Result<Self> {
        let server = lookup_host((config.server.as_str(), config.port))
            .await
            .with_context(|| format!("ssr udp: resolve {}:{}", config.server, config.port))?
            .next()
            .ok_or_else(|| anyhow::anyhow!("ssr udp: no address for {}:{}", config.server, config.port))?;
        let socket = crate::udp::bind_egress(server).await?;
        socket
            .connect(server)
            .await
            .with_context(|| format!("ssr udp: connect {server}"))?;
        let mut user_id = [0u8; 4];
        random_bytes(&mut user_id);
        Ok(Self {
            socket,
            cipher: config.cipher,
            key: config.key.clone(),
            protocol: config.protocol,
            target: target.clone(),
            user_id,
        })
    }

    /// Seal `payload` for the destination and send it to the server.
    pub async fn send(&self, payload: &[u8]) -> Result<()> {
        let packet = self.seal(payload);
        self.socket.send(&packet).await.context("ssr udp: send")?;
        Ok(())
    }

    /// Receive one reply datagram, open it, and return the application payload
    /// (the source-address prefix is discarded).
    pub async fn recv(&self) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; 64 * 1024];
        let n = self.socket.recv(&mut buf).await.context("ssr udp: recv")?;
        self.open(&buf[..n])
    }

    fn seal(&self, payload: &[u8]) -> Vec<u8> {
        let mut inner = Vec::with_capacity(1 + 256 + 2 + payload.len());
        socks5::encode_address(&mut inner, &self.target);
        inner.extend_from_slice(payload);

        let mut framed = udp_pre_encrypt(self.protocol, &self.key, &self.user_id, &inner);

        let iv_len = self.cipher.iv_size();
        let mut iv = vec![0u8; iv_len];
        random_bytes(&mut iv);
        StreamCryptor::new_encrypt(self.cipher, &self.key, &iv).update(&mut framed);

        let mut packet = iv;
        packet.extend_from_slice(&framed);
        packet
    }

    fn open(&self, datagram: &[u8]) -> Result<Vec<u8>> {
        let iv_len = self.cipher.iv_size();
        if datagram.len() < iv_len {
            bail!("ssr udp: datagram shorter than IV");
        }
        let (iv, body) = datagram.split_at(iv_len);
        let mut framed = body.to_vec();
        StreamCryptor::new_decrypt(self.cipher, &self.key, iv).update(&mut framed);

        let inner = udp_post_decrypt(self.protocol, &self.key, &framed)?;
        let (_source, offset) = socks5::decode_address(&inner)?;
        Ok(inner[offset..].to_vec())
    }
}

/// Apply the protocol layer's UDP framing to an outgoing (client→server) packet.
fn udp_pre_encrypt(protocol: SsrProtocol, key: &[u8], user_id: &[u8; 4], inner: &[u8]) -> Vec<u8> {
    match protocol {
        SsrProtocol::Origin => inner.to_vec(),
        SsrProtocol::AuthAes128Sha1 => auth_aes128_udp_pre(AuthHashKind::Sha1, key, user_id, inner),
        SsrProtocol::AuthAes128Md5 => auth_aes128_udp_pre(AuthHashKind::Md5, key, user_id, inner),
        SsrProtocol::AuthChainA => auth_chain_a_udp_pre(key, user_id, inner),
    }
}

/// Strip and verify the protocol layer's UDP framing from a server reply.
fn udp_post_decrypt(protocol: SsrProtocol, key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    match protocol {
        SsrProtocol::Origin => Ok(data.to_vec()),
        SsrProtocol::AuthAes128Sha1 => auth_aes128_udp_post(AuthHashKind::Sha1, key, data),
        SsrProtocol::AuthAes128Md5 => auth_aes128_udp_post(AuthHashKind::Md5, key, data),
        SsrProtocol::AuthChainA => auth_chain_a_udp_post(key, data),
    }
}

// -- auth_aes128 UDP --------------------------------------------------------

/// `buf ++ user_id ++ HMAC(key, buf ++ user_id)[:4]`.
fn auth_aes128_udp_pre(hash: AuthHashKind, key: &[u8], user_id: &[u8; 4], inner: &[u8]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(inner.len() + 4 + 4);
    buf.extend_from_slice(inner);
    buf.extend_from_slice(user_id);
    let mac = hmac_digest(hash, key, &buf);
    buf.extend_from_slice(&mac[..4]);
    buf
}

/// Verify the trailing 4-byte HMAC and strip it (server replies carry no uid).
fn auth_aes128_udp_post(hash: AuthHashKind, key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    if data.len() < 4 {
        bail!("ssr udp: auth_aes128 reply too short");
    }
    let (body, tag) = data.split_at(data.len() - 4);
    let mac = hmac_digest(hash, key, body);
    if mac[..4] != *tag {
        bail!("ssr udp: auth_aes128 HMAC mismatch");
    }
    Ok(body.to_vec())
}

// -- auth_chain_a UDP -------------------------------------------------------

/// `RC4(rc4_key, inner) ++ rand_pad ++ authdata(3) ++ uid(4) ++ HMAC(key, ·)[:1]`
/// where `rc4_key = base64(key) ++ base64(HMAC-MD5(key, authdata))`, the padding
/// length comes from upstream's xorshift128plus seeded by that HMAC, and `uid`
/// is `user_id XOR md5data[:4]`.
fn auth_chain_a_udp_pre(key: &[u8], user_id: &[u8; 4], inner: &[u8]) -> Vec<u8> {
    let mut authdata = [0u8; 3];
    random_bytes(&mut authdata);
    let md5data = hmac_md5(key, &authdata);

    let uid = u32::from_le_bytes(*user_id) ^ u32::from_le_bytes([md5data[0], md5data[1], md5data[2], md5data[3]]);
    let rand_len = udp_rnd_data_len(&md5data);
    let rc4_key = auth_chain_rc4_key(key, &md5data);

    let mut out = inner.to_vec();
    rc4_apply(&rc4_key, &mut out);

    let mut pad = vec![0u8; rand_len];
    random_bytes(&mut pad);
    out.extend_from_slice(&pad);
    out.extend_from_slice(&authdata);
    out.extend_from_slice(&uid.to_le_bytes());

    let mac = hmac_md5(key, &out);
    out.push(mac[0]);
    out
}

/// Verify the 1-byte HMAC, recover the padding length from the server's 7-byte
/// authdata, and RC4-decrypt the leading payload.
fn auth_chain_a_udp_post(key: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    if data.len() <= 8 {
        bail!("ssr udp: auth_chain_a reply too short");
    }
    let (body, tag) = data.split_at(data.len() - 1);
    let mac = hmac_md5(key, body);
    if mac[0] != tag[0] {
        bail!("ssr udp: auth_chain_a HMAC mismatch");
    }

    // The 7 bytes before the 1-byte HMAC are the server's authdata.
    let authdata = &data[data.len() - 8..data.len() - 1];
    let md5data = hmac_md5(key, authdata);
    let rand_len = udp_rnd_data_len(&md5data);
    let rc4_key = auth_chain_rc4_key(key, &md5data);

    let end = data
        .len()
        .checked_sub(8 + rand_len)
        .ok_or_else(|| anyhow::anyhow!("ssr udp: auth_chain_a padding overruns packet"))?;
    let mut out = data[..end].to_vec();
    rc4_apply(&rc4_key, &mut out);
    Ok(out)
}

/// RC4 key for an auth_chain_a UDP packet: the base64 of the user key
/// concatenated with the base64 of the per-packet HMAC seed.
fn auth_chain_rc4_key(key: &[u8], md5data: &[u8]) -> Vec<u8> {
    let mut rc4_key = base64_encode(key);
    rc4_key.extend_from_slice(&base64_encode(md5data));
    rc4_key
}

/// Per-packet random padding length for auth_chain_a UDP: seed upstream's
/// xorshift128plus with `last_hash` and take `next() % 127`.
fn udp_rnd_data_len(last_hash: &[u8]) -> usize {
    (SsrShiftRng::from_bin(last_hash).next() % 127) as usize
}

/// Upstream shadowsocksr's `xorshift128plus` variant (distinct from the simple
/// generator used by the TCP `auth_chain_a` framing): the shift mixing differs,
/// so UDP must use this exact form to interoperate.
struct SsrShiftRng {
    v0: u64,
    v1: u64,
}

impl SsrShiftRng {
    const MOV_MASK: u64 = (1u64 << (64 - 23)) - 1;

    fn from_bin(bin: &[u8]) -> Self {
        let mut b = [0u8; 16];
        let n = bin.len().min(16);
        b[..n].copy_from_slice(&bin[..n]);
        Self {
            v0: u64::from_le_bytes(b[..8].try_into().expect("8 bytes")),
            v1: u64::from_le_bytes(b[8..16].try_into().expect("8 bytes")),
        }
    }

    fn next(&mut self) -> u64 {
        let mut x = self.v0;
        let y = self.v1;
        self.v0 = y;
        x ^= (x & Self::MOV_MASK) << 23;
        x ^= y ^ (x >> 17) ^ (y >> 26);
        self.v1 = x;
        x.wrapping_add(y)
    }
}
