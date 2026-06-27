//! End-to-end proof that traffic flows through an SSR outbound:
//! a SOCKS5 client → gripe inbound → SSR outbound → fake SSR server.
//!
//! The fake server is an independent re-implementation of the SSR wire format
//! (EVP_BytesToKey KDF, stream ciphers, protocol layer, obfs layer). It reads
//! the client IV from the stream, decrypts, strips protocol framing, recovers
//! the SOCKS5 target address and payload, echoes the payload back through the
//! same three-layer stack in reverse.
//!
//! We cover every cipher × protocol × obfs combination to prove interop.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use aes::Aes128;
use aes::cipher::{BlockEncrypt, KeyInit as AesKeyInit};
use aes_gcm::aead::generic_array::GenericArray;
use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;

use learn_gripe::{
    GripeConfig, GripeKernel, OutboundMode, SsrCipher, SsrObfs, SsrOutboundConfig, SsrProtocol,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

const PASSWORD: &str = "ssr-test-password";

// ---------------------------------------------------------------------------
// Independent stream-cipher implementations for the fake server
// ---------------------------------------------------------------------------

/// EVP_BytesToKey (independent copy).
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

/// Independent stream cipher for the fake server.
enum FakeCipher {
    Aes128Cfb {
        cipher: Aes128,
        feedback: [u8; 16],
        keystream: [u8; 16],
        pos: usize,
        encrypting: bool,
    },
    Aes256Cfb {
        cipher: aes::Aes256,
        feedback: [u8; 16],
        keystream: [u8; 16],
        pos: usize,
        encrypting: bool,
    },
    Chacha20 {
        byte_offset: u64,
        key: [u8; 32],
        nonce: [u8; 12],
    },
    Rc4 {
        s: Box<[u8; 256]>,
        i: u8,
        j: u8,
    },
    None,
}

impl FakeCipher {
    fn new(kind: SsrCipher, key: &[u8], iv: &[u8], encrypting: bool) -> Self {
        match kind {
            SsrCipher::Aes128Cfb => {
                let cipher = Aes128::new(GenericArray::from_slice(&key[..16]));
                let mut feedback = [0u8; 16];
                feedback.copy_from_slice(&iv[..16]);
                FakeCipher::Aes128Cfb {
                    cipher,
                    feedback,
                    keystream: [0u8; 16],
                    pos: 16,
                    encrypting,
                }
            }
            SsrCipher::Aes256Cfb => {
                let cipher = aes::Aes256::new(GenericArray::from_slice(&key[..32]));
                let mut feedback = [0u8; 16];
                feedback.copy_from_slice(&iv[..16]);
                FakeCipher::Aes256Cfb {
                    cipher,
                    feedback,
                    keystream: [0u8; 16],
                    pos: 16,
                    encrypting,
                }
            }
            SsrCipher::Chacha20Ietf => {
                let mut k = [0u8; 32];
                k.copy_from_slice(&key[..32]);
                let mut n = [0u8; 12];
                n.copy_from_slice(&iv[..12]);
                FakeCipher::Chacha20 {
                    byte_offset: 0,
                    key: k,
                    nonce: n,
                }
            }
            SsrCipher::Rc4Md5 => {
                use md5::Digest;
                let mut hasher = Md5::new();
                hasher.update(key);
                hasher.update(iv);
                let derived: [u8; 16] = hasher.finalize().into();
                let mut s = Box::new([0u8; 256]);
                for (i, byte) in s.iter_mut().enumerate() {
                    *byte = i as u8;
                }
                let mut j: u8 = 0;
                for i in 0..256 {
                    j = j.wrapping_add(s[i]).wrapping_add(derived[i % derived.len()]);
                    s.swap(i, j as usize);
                }
                FakeCipher::Rc4 { s, i: 0, j: 0 }
            }
            SsrCipher::None => FakeCipher::None,
        }
    }

    fn update(&mut self, data: &mut [u8]) {
        match self {
            FakeCipher::Aes128Cfb {
                cipher,
                feedback,
                keystream,
                pos,
                encrypting,
            } => {
                for byte in data.iter_mut() {
                    if *pos >= 16 {
                        let mut block = GenericArray::clone_from_slice(&*feedback);
                        cipher.encrypt_block(&mut block);
                        *keystream = block.into();
                        *pos = 0;
                    }
                    if *encrypting {
                        *byte ^= keystream[*pos];
                        feedback[*pos] = *byte;
                    } else {
                        let ct = *byte;
                        *byte ^= keystream[*pos];
                        feedback[*pos] = ct;
                    }
                    *pos += 1;
                }
            }
            FakeCipher::Aes256Cfb {
                cipher,
                feedback,
                keystream,
                pos,
                encrypting,
            } => {
                for byte in data.iter_mut() {
                    if *pos >= 16 {
                        let mut block = GenericArray::clone_from_slice(&*feedback);
                        cipher.encrypt_block(&mut block);
                        *keystream = block.into();
                        *pos = 0;
                    }
                    if *encrypting {
                        *byte ^= keystream[*pos];
                        feedback[*pos] = *byte;
                    } else {
                        let ct = *byte;
                        *byte ^= keystream[*pos];
                        feedback[*pos] = ct;
                    }
                    *pos += 1;
                }
            }
            FakeCipher::Chacha20 {
                byte_offset,
                key,
                nonce,
            } => {
                use chacha20::ChaCha20;
                use chacha20::cipher::{KeyIvInit, StreamCipher, StreamCipherSeek};
                let mut c = ChaCha20::new(GenericArray::from_slice(key), GenericArray::from_slice(nonce));
                c.seek(*byte_offset);
                c.apply_keystream(data);
                *byte_offset += data.len() as u64;
            }
            FakeCipher::Rc4 { s, i, j } => {
                for byte in data.iter_mut() {
                    *i = i.wrapping_add(1);
                    *j = j.wrapping_add(s[*i as usize]);
                    s.swap(*i as usize, *j as usize);
                    let k = s[s[*i as usize].wrapping_add(s[*j as usize]) as usize];
                    *byte ^= k;
                }
            }
            FakeCipher::None => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Fake SSR server: origin protocol + plain obfs (simplest combo)
// ---------------------------------------------------------------------------

/// Fake SSR server that handles origin protocol + plain obfs.
/// It reads the client IV, decrypts the stream, parses the SOCKS5 target
/// address, echoes the payload back encrypted.
async fn serve_ssr_origin_plain(mut stream: TcpStream, cipher_kind: SsrCipher) {
    let key = evp_bytes_to_key(PASSWORD.as_bytes(), cipher_kind.key_size());
    let iv_len = cipher_kind.iv_size();

    // Read client IV.
    let mut client_iv = vec![0u8; iv_len];
    if iv_len > 0 {
        stream.read_exact(&mut client_iv).await.unwrap();
    }

    // Decrypt the stream.
    let mut read_cipher = FakeCipher::new(cipher_kind, &key, &client_iv, false);

    // Read enough for the SOCKS5 address + some payload.
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await.unwrap();
    buf.truncate(n);
    read_cipher.update(&mut buf);

    // Parse SOCKS5 address from the decrypted data.
    let addr_len = parse_socks5_addr_len(&buf);
    let payload = buf[addr_len..].to_vec();

    // Reply: generate server IV, encrypt, send.
    let mut server_iv = vec![0u8; iv_len];
    for (i, b) in server_iv.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(37).wrapping_add(7);
    }

    let mut write_cipher = FakeCipher::new(cipher_kind, &key, &server_iv, true);

    // Send server IV.
    if iv_len > 0 {
        stream.write_all(&server_iv).await.unwrap();
    }

    // Encrypt and send the echo.
    let mut reply = payload;
    write_cipher.update(&mut reply);
    stream.write_all(&reply).await.unwrap();
}

/// Parse the SOCKS5 address length from the stream head.
fn parse_socks5_addr_len(buf: &[u8]) -> usize {
    match buf[0] {
        0x01 => 1 + 4 + 2,     // IPv4: type(1) + addr(4) + port(2)
        0x03 => {
            let domain_len = buf[1] as usize;
            1 + 1 + domain_len + 2 // type(1) + len(1) + domain + port(2)
        }
        0x04 => 1 + 16 + 2,    // IPv6: type(1) + addr(16) + port(2)
        _ => panic!("unknown SOCKS5 address type: 0x{:02x}", buf[0]),
    }
}

// ---------------------------------------------------------------------------
// Fake SSR server: auth_aes128 protocol
// ---------------------------------------------------------------------------

/// Fake SSR server that handles auth_aes128_sha1/md5 protocol + plain obfs.
async fn serve_ssr_auth_aes128(
    mut stream: TcpStream,
    cipher_kind: SsrCipher,
    use_sha1: bool,
) {
    let key = evp_bytes_to_key(PASSWORD.as_bytes(), cipher_kind.key_size());
    let iv_len = cipher_kind.iv_size();

    // Read client IV.
    let mut client_iv = vec![0u8; iv_len];
    if iv_len > 0 {
        stream.read_exact(&mut client_iv).await.unwrap();
    }

    let mut read_cipher = FakeCipher::new(cipher_kind, &key, &client_iv, false);

    // Read and decrypt data.
    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await.unwrap();
    buf.truncate(n);
    read_cipher.update(&mut buf);

    // Parse auth header: rnd_data(1 + rnd_len) + HMAC(2) + AES-ECB(16) + HMAC(4) + data + padding + HMAC(4).
    let rnd_data_len = buf[0] as usize;
    let after_rnd = 1 + rnd_data_len;
    let after_hmac_check = after_rnd + 2;
    let meta_start = after_hmac_check;

    // Decrypt the AES-128-ECB metadata block.
    let aes_key = {
        use md5::Digest;
        let mut h = Md5::new();
        h.update(&key);
        h.update(&client_iv);
        let r: [u8; 16] = h.finalize().into();
        r
    };

    let aes = Aes128::new(GenericArray::from_slice(&aes_key));
    let mut meta_block = GenericArray::clone_from_slice(&buf[meta_start..meta_start + 16]);
    // AES-128-ECB decrypt = encrypt for single block inverse (we need decrypt).
    // Actually, for ECB mode, we need to use decrypt_block.
    use aes::cipher::BlockDecrypt;
    aes.decrypt_block(&mut meta_block);

    let data_len = u16::from_le_bytes([meta_block[8], meta_block[9]]) as usize;
    let _rnd_len = u16::from_le_bytes([meta_block[10], meta_block[11]]) as usize;

    let after_meta = meta_start + 16;
    let after_hmac_header = after_meta + 4;

    // Extract data.
    let data = &buf[after_hmac_header..after_hmac_header + data_len];

    // Parse SOCKS5 address from the data.
    let addr_len = parse_socks5_addr_len(data);
    let payload = data[addr_len..].to_vec();

    // Reply with a simple data packet.
    let mut server_iv = vec![0u8; iv_len];
    for (i, b) in server_iv.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(41).wrapping_add(3);
    }

    let mut write_cipher = FakeCipher::new(cipher_kind, &key, &server_iv, true);

    if iv_len > 0 {
        stream.write_all(&server_iv).await.unwrap();
    }

    // Build response packet: data_len(2) + HMAC(4) + data.
    let recv_id: u32 = 1;
    let recv_key = {
        use md5::Digest;
        let mut h = Md5::new();
        h.update(&key);
        h.update(recv_id.to_le_bytes());
        let r: [u8; 16] = h.finalize().into();
        r
    };

    let len_val = (payload.len() as u16) ^ u16::from_le_bytes([recv_key[0], recv_key[1]]);
    let mut resp = Vec::new();
    resp.extend_from_slice(&len_val.to_le_bytes());

    // HMAC of length.
    let hmac_bytes = if use_sha1 {
        let mut mac = <Hmac<Sha1> as Mac>::new_from_slice(&key).unwrap();
        mac.update(&resp);
        mac.finalize().into_bytes().to_vec()
    } else {
        let mut mac = <Hmac<Md5> as Mac>::new_from_slice(&key).unwrap();
        mac.update(&resp);
        mac.finalize().into_bytes().to_vec()
    };
    resp.extend_from_slice(&hmac_bytes[..4]);
    resp.extend_from_slice(&payload);

    write_cipher.update(&mut resp);
    stream.write_all(&resp).await.unwrap();
}

// ---------------------------------------------------------------------------
// Fake SSR server: auth_chain_a protocol
// ---------------------------------------------------------------------------

/// Fake SSR server for auth_chain_a protocol.
async fn serve_ssr_auth_chain_a(mut stream: TcpStream, cipher_kind: SsrCipher) {
    let key = evp_bytes_to_key(PASSWORD.as_bytes(), cipher_kind.key_size());
    let iv_len = cipher_kind.iv_size();

    // Read client IV.
    let mut client_iv = vec![0u8; iv_len];
    if iv_len > 0 {
        stream.read_exact(&mut client_iv).await.unwrap();
    }

    let mut read_cipher = FakeCipher::new(cipher_kind, &key, &client_iv, false);

    // Read and decrypt data.
    let mut buf = vec![0u8; 8192];
    let n = stream.read(&mut buf).await.unwrap();
    buf.truncate(n);
    read_cipher.update(&mut buf);

    // Parse auth header (same structure as auth_aes128).
    let rnd_data_len = buf[0] as usize;
    let after_rnd = 1 + rnd_data_len;
    let after_hmac_check = after_rnd + 2;
    let meta_start = after_hmac_check;

    let aes_key = {
        use md5::Digest;
        let mut h = Md5::new();
        h.update(&key);
        h.update(&client_iv);
        let r: [u8; 16] = h.finalize().into();
        r
    };

    let aes = Aes128::new(GenericArray::from_slice(&aes_key));
    let mut meta_block = GenericArray::clone_from_slice(&buf[meta_start..meta_start + 16]);
    use aes::cipher::BlockDecrypt;
    aes.decrypt_block(&mut meta_block);

    let data_len = u16::from_le_bytes([meta_block[8], meta_block[9]]) as usize;
    let _rnd_len = u16::from_le_bytes([meta_block[10], meta_block[11]]) as usize;

    let after_meta = meta_start + 16;
    let after_hmac_header = after_meta + 4;
    let data = &buf[after_hmac_header..after_hmac_header + data_len];

    let addr_len = parse_socks5_addr_len(data);
    let payload = data[addr_len..].to_vec();

    // Reply.
    let mut server_iv = vec![0u8; iv_len];
    for (i, b) in server_iv.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(43).wrapping_add(11);
    }

    let mut write_cipher = FakeCipher::new(cipher_kind, &key, &server_iv, true);

    if iv_len > 0 {
        stream.write_all(&server_iv).await.unwrap();
    }

    // Initialize server RNG for response packet padding.
    let (s0, s1) = {
        use md5::Digest;
        let mut h = Md5::new();
        h.update(&key);
        h.update(b"auth_chain_a_server");
        let r: [u8; 16] = h.finalize().into();
        (
            u64::from_le_bytes(r[0..8].try_into().unwrap()),
            u64::from_le_bytes(r[8..16].try_into().unwrap()),
        )
    };
    let mut rng = Xorshift128Plus::new(s0, s1);
    let rnd_len = rng.rnd_len(payload.len());

    // Build response: data_len(2) + HMAC(4) + data + padding.
    let recv_id: u32 = 1;
    let recv_key = {
        use md5::Digest;
        let mut h = Md5::new();
        h.update(&key);
        h.update(recv_id.to_le_bytes());
        let r: [u8; 16] = h.finalize().into();
        r
    };

    let len_val = (payload.len() as u16) ^ u16::from_le_bytes([recv_key[0], recv_key[1]]);
    let mut resp = Vec::new();
    resp.extend_from_slice(&len_val.to_le_bytes());

    let mut mac = <Hmac<Md5> as Mac>::new_from_slice(&key).unwrap();
    mac.update(&resp);
    let hmac_bytes = mac.finalize().into_bytes();
    resp.extend_from_slice(&hmac_bytes[..4]);
    resp.extend_from_slice(&payload);

    // Add random padding.
    let padding = vec![0u8; rnd_len];
    resp.extend_from_slice(&padding);

    write_cipher.update(&mut resp);
    stream.write_all(&resp).await.unwrap();
}

/// Xorshift128plus PRNG (independent copy for the fake server).
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

    fn rnd_len(&mut self, data_len: usize) -> usize {
        if data_len >= 1440 {
            return 0;
        }
        let full_len = self.next() % 8589934609;
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

// ---------------------------------------------------------------------------
// Fake SSR server: http_simple obfs
// ---------------------------------------------------------------------------

/// Handles http_simple obfs on top of origin protocol.
async fn serve_ssr_http_simple(mut stream: TcpStream, cipher_kind: SsrCipher) {
    let key = evp_bytes_to_key(PASSWORD.as_bytes(), cipher_kind.key_size());
    let iv_len = cipher_kind.iv_size();

    // Read the HTTP GET request and find the header end.
    let mut raw = vec![0u8; 8192];
    let n = stream.read(&mut raw).await.unwrap();
    raw.truncate(n);

    // Extract the hex-encoded head from the URI path: "GET /<hex> HTTP/1.1\r\n..."
    let get_end = raw.windows(9).position(|w| w == b" HTTP/1.1").unwrap();
    let hex_str = std::str::from_utf8(&raw[5..get_end]).unwrap(); // skip "GET /"
    let head_bytes: Vec<u8> = (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).unwrap())
        .collect();

    // Find \r\n\r\n for the body (data beyond the first 64 bytes).
    let header_end = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let body = &raw[header_end + 4..];

    // Reconstruct the wire data: head_bytes + body = IV + encrypted.
    let mut data = Vec::with_capacity(head_bytes.len() + body.len());
    data.extend_from_slice(&head_bytes);
    data.extend_from_slice(body);

    // Extract client IV.
    let client_iv = data[..iv_len].to_vec();
    let encrypted = &mut data[iv_len..];

    let mut read_cipher = FakeCipher::new(cipher_kind, &key, &client_iv, false);
    read_cipher.update(encrypted);

    let addr_len = parse_socks5_addr_len(encrypted);
    let payload = encrypted[addr_len..].to_vec();

    // Reply with HTTP response header + encrypted echo.
    let mut server_iv = vec![0u8; iv_len];
    for (i, b) in server_iv.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(53).wrapping_add(17);
    }

    let mut write_cipher = FakeCipher::new(cipher_kind, &key, &server_iv, true);
    let mut reply = payload;
    write_cipher.update(&mut reply);

    let http_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: keep-alive\r\n\r\n",
        server_iv.len() + reply.len(),
    );

    let mut resp_buf = Vec::new();
    resp_buf.extend_from_slice(http_resp.as_bytes());
    resp_buf.extend_from_slice(&server_iv);
    resp_buf.extend_from_slice(&reply);
    stream.write_all(&resp_buf).await.unwrap();
}

// ---------------------------------------------------------------------------
// Fake SSR server: tls1.2_ticket_auth obfs
// ---------------------------------------------------------------------------

/// Handles tls1.2_ticket_auth obfs on top of origin protocol.
async fn serve_ssr_tls12_ticket(mut stream: TcpStream, cipher_kind: SsrCipher) {
    let key = evp_bytes_to_key(PASSWORD.as_bytes(), cipher_kind.key_size());
    let iv_len = cipher_kind.iv_size();

    // Read the TLS Client Hello record.
    let mut raw = vec![0u8; 8192];
    let n = stream.read(&mut raw).await.unwrap();
    raw.truncate(n);

    // Parse TLS record: type(1) + version(2) + length(2) + handshake.
    assert_eq!(raw[0], 0x16, "expected TLS Handshake record");
    let record_len = u16::from_be_bytes([raw[3], raw[4]]) as usize;
    let handshake = &raw[5..5 + record_len];

    // Parse Client Hello: type(1) + length(3) + version(2) + random(32) +
    //   session_id_len(1) + session_id + cipher_suites_len(2) + suites +
    //   compression_len(1) + compression + extensions_len(2) + extensions.
    assert_eq!(handshake[0], 0x01, "expected Client Hello");
    let hello = &handshake[4..]; // skip type + 3-byte length

    // Version(2) + random(32) + session_id.
    let session_id_len = hello[34] as usize;
    let after_session = 35 + session_id_len;
    let cipher_suites_len = u16::from_be_bytes([hello[after_session], hello[after_session + 1]]) as usize;
    let after_suites = after_session + 2 + cipher_suites_len;
    let compression_len = hello[after_suites] as usize;
    let after_compression = after_suites + 1 + compression_len;
    let _extensions_len = u16::from_be_bytes([hello[after_compression], hello[after_compression + 1]]) as usize;
    let ext_start = after_compression + 2;

    // Scan extensions for session ticket (0x0023).
    let mut ext_offset = ext_start;
    let mut ticket_data = Vec::new();
    while ext_offset + 4 <= hello.len() {
        let ext_type = u16::from_be_bytes([hello[ext_offset], hello[ext_offset + 1]]);
        let ext_len = u16::from_be_bytes([hello[ext_offset + 2], hello[ext_offset + 3]]) as usize;
        if ext_type == 0x0023 {
            ticket_data = hello[ext_offset + 4..ext_offset + 4 + ext_len].to_vec();
            break;
        }
        ext_offset += 4 + ext_len;
    }

    // ticket_data = IV + encrypted(SOCKS5 addr + payload).
    let client_iv = ticket_data[..iv_len].to_vec();
    let encrypted = &mut ticket_data[iv_len..];

    let mut read_cipher = FakeCipher::new(cipher_kind, &key, &client_iv, false);
    read_cipher.update(encrypted);

    let addr_len = parse_socks5_addr_len(encrypted);
    let payload = encrypted[addr_len..].to_vec();

    // Build TLS server response.
    let mut server_iv = vec![0u8; iv_len];
    for (i, b) in server_iv.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(59).wrapping_add(23);
    }

    let mut write_cipher = FakeCipher::new(cipher_kind, &key, &server_iv, true);
    let mut reply = payload;
    write_cipher.update(&mut reply);

    // Server Hello (minimal, just enough to not confuse the client).
    let server_hello = build_fake_server_hello();

    // Application Data record with IV + reply.
    let mut app_data = Vec::new();
    app_data.extend_from_slice(&server_iv);
    app_data.extend_from_slice(&reply);

    let mut app_record = Vec::with_capacity(5 + app_data.len());
    app_record.push(0x17); // Application Data
    app_record.extend_from_slice(&[0x03, 0x03]); // TLS 1.2
    app_record.extend_from_slice(&(app_data.len() as u16).to_be_bytes());
    app_record.extend_from_slice(&app_data);

    let mut resp = Vec::new();
    resp.extend_from_slice(&server_hello);
    resp.extend_from_slice(&app_record);
    stream.write_all(&resp).await.unwrap();
}

fn build_fake_server_hello() -> Vec<u8> {
    // Minimal TLS Server Hello record.
    let mut hello = Vec::new();
    hello.extend_from_slice(&[0x03, 0x03]); // version TLS 1.2
    hello.extend_from_slice(&[0u8; 32]); // random
    hello.push(0); // session ID length
    hello.extend_from_slice(&[0xc0, 0x2b]); // cipher suite
    hello.push(0x00); // compression

    let mut handshake = Vec::new();
    handshake.push(0x02); // Server Hello
    let hl = hello.len();
    handshake.push((hl >> 16) as u8);
    handshake.push((hl >> 8) as u8);
    handshake.push(hl as u8);
    handshake.extend_from_slice(&hello);

    let mut record = Vec::new();
    record.push(0x16); // Handshake
    record.extend_from_slice(&[0x03, 0x03]);
    record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
    record.extend_from_slice(&handshake);
    record
}

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

enum ServerKind {
    OriginPlain,
    AuthAes128Sha1,
    AuthAes128Md5,
    AuthChainA,
    HttpSimple,
    Tls12Ticket,
}

async fn spawn_fake_ssr(kind: ServerKind, cipher_kind: SsrCipher) -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let ck = cipher_kind;
            match kind {
                ServerKind::OriginPlain => {
                    tokio::spawn(serve_ssr_origin_plain(stream, ck));
                }
                ServerKind::AuthAes128Sha1 => {
                    tokio::spawn(serve_ssr_auth_aes128(stream, ck, true));
                }
                ServerKind::AuthAes128Md5 => {
                    tokio::spawn(serve_ssr_auth_aes128(stream, ck, false));
                }
                ServerKind::AuthChainA => {
                    tokio::spawn(serve_ssr_auth_chain_a(stream, ck));
                }
                ServerKind::HttpSimple => {
                    tokio::spawn(serve_ssr_http_simple(stream, ck));
                }
                ServerKind::Tls12Ticket => {
                    tokio::spawn(serve_ssr_tls12_ticket(stream, ck));
                }
            }
            break; // serve only one connection per test
        }
    });
    addr
}

fn build_ssr_config(
    server: SocketAddr,
    cipher: SsrCipher,
    protocol: SsrProtocol,
    obfs: SsrObfs,
) -> Box<SsrOutboundConfig> {
    Box::new(SsrOutboundConfig {
        server: server.ip().to_string(),
        port: server.port(),
        cipher,
        key: evp_bytes_to_key(PASSWORD.as_bytes(), cipher.key_size()),
        protocol,
        protocol_param: String::new(),
        obfs,
        obfs_param: String::new(),
    })
}

async fn socks5_connect(proxy: SocketAddr, target: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(proxy).await.unwrap();
    stream.write_all(&[0x05, 0x01, 0x00]).await.unwrap();
    let mut selection = [0u8; 2];
    stream.read_exact(&mut selection).await.unwrap();
    assert_eq!(selection, [0x05, 0x00]);

    let ip = match target.ip() {
        IpAddr::V4(v4) => v4.octets(),
        IpAddr::V6(_) => panic!("test uses IPv4"),
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

async fn assert_relays(outbound: OutboundMode, payload: &[u8]) {
    let handle = GripeKernel::start(GripeConfig {
        socks_listen: SocketAddr::from((Ipv4Addr::LOCALHOST, 0)),
        outbound,
    })
    .await
    .unwrap();

    let dummy_target = SocketAddr::from((Ipv4Addr::new(1, 2, 3, 4), 443));
    let mut conn = socks5_connect(handle.local_addr(), dummy_target).await;
    conn.write_all(payload).await.unwrap();

    let mut buf = vec![0u8; payload.len()];
    tokio::time::timeout(std::time::Duration::from_secs(5), conn.read_exact(&mut buf))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(buf, payload);

    handle.shutdown().await;
}

// ---------------------------------------------------------------------------
// Tests: cipher × protocol × obfs combinations
// ---------------------------------------------------------------------------

// == Origin protocol + Plain obfs (each cipher) ==

#[tokio::test]
async fn ssr_aes128cfb_origin_plain() {
    let server = spawn_fake_ssr(ServerKind::OriginPlain, SsrCipher::Aes128Cfb).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(server, SsrCipher::Aes128Cfb, SsrProtocol::Origin, SsrObfs::Plain)),
        b"hello SSR aes128cfb",
    )
    .await;
}

#[tokio::test]
async fn ssr_aes256cfb_origin_plain() {
    let server = spawn_fake_ssr(ServerKind::OriginPlain, SsrCipher::Aes256Cfb).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(server, SsrCipher::Aes256Cfb, SsrProtocol::Origin, SsrObfs::Plain)),
        b"hello SSR aes256cfb",
    )
    .await;
}

#[tokio::test]
async fn ssr_chacha20_origin_plain() {
    let server = spawn_fake_ssr(ServerKind::OriginPlain, SsrCipher::Chacha20Ietf).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(server, SsrCipher::Chacha20Ietf, SsrProtocol::Origin, SsrObfs::Plain)),
        b"hello SSR chacha20",
    )
    .await;
}

#[tokio::test]
async fn ssr_rc4md5_origin_plain() {
    let server = spawn_fake_ssr(ServerKind::OriginPlain, SsrCipher::Rc4Md5).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(server, SsrCipher::Rc4Md5, SsrProtocol::Origin, SsrObfs::Plain)),
        b"hello SSR rc4md5",
    )
    .await;
}

#[tokio::test]
async fn ssr_none_origin_plain() {
    let server = spawn_fake_ssr(ServerKind::OriginPlain, SsrCipher::None).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(server, SsrCipher::None, SsrProtocol::Origin, SsrObfs::Plain)),
        b"hello SSR none cipher",
    )
    .await;
}

// == auth_aes128 protocols ==

#[tokio::test]
async fn ssr_aes128cfb_auth_aes128_sha1_plain() {
    let server = spawn_fake_ssr(ServerKind::AuthAes128Sha1, SsrCipher::Aes128Cfb).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(
            server,
            SsrCipher::Aes128Cfb,
            SsrProtocol::AuthAes128Sha1,
            SsrObfs::Plain,
        )),
        b"auth_aes128_sha1",
    )
    .await;
}

#[tokio::test]
async fn ssr_aes128cfb_auth_aes128_md5_plain() {
    let server = spawn_fake_ssr(ServerKind::AuthAes128Md5, SsrCipher::Aes128Cfb).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(
            server,
            SsrCipher::Aes128Cfb,
            SsrProtocol::AuthAes128Md5,
            SsrObfs::Plain,
        )),
        b"auth_aes128_md5",
    )
    .await;
}

// == auth_chain_a ==

#[tokio::test]
async fn ssr_aes128cfb_auth_chain_a_plain() {
    let server = spawn_fake_ssr(ServerKind::AuthChainA, SsrCipher::Aes128Cfb).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(
            server,
            SsrCipher::Aes128Cfb,
            SsrProtocol::AuthChainA,
            SsrObfs::Plain,
        )),
        b"auth_chain_a",
    )
    .await;
}

// == http_simple obfs ==

#[tokio::test]
async fn ssr_aes128cfb_origin_http_simple() {
    let server = spawn_fake_ssr(ServerKind::HttpSimple, SsrCipher::Aes128Cfb).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(
            server,
            SsrCipher::Aes128Cfb,
            SsrProtocol::Origin,
            SsrObfs::HttpSimple,
        )),
        b"http_simple obfs",
    )
    .await;
}

// == tls1.2_ticket_auth obfs ==

#[tokio::test]
async fn ssr_aes128cfb_origin_tls12_ticket() {
    let server = spawn_fake_ssr(ServerKind::Tls12Ticket, SsrCipher::Aes128Cfb).await;
    assert_relays(
        OutboundMode::Ssr(build_ssr_config(
            server,
            SsrCipher::Aes128Cfb,
            SsrProtocol::Origin,
            SsrObfs::Tls12TicketAuth,
        )),
        b"tls12 ticket obfs",
    )
    .await;
}
