use super::SsrOutboundConfig;
use super::cipher::{SsrCipher, StreamCryptor};
use super::crypto::evp_bytes_to_key;
use super::obfs::{HttpSimpleState, SsrObfs, Tls12TicketAuthState};
use super::protocol::SsrProtocol;
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
    let entry = parse_entry("name: s\ntype: ssr\nserver: s\nport: 1\ncipher: none\npassword: p\n");
    let config = SsrOutboundConfig::from_proxy(&entry).expect("valid");
    assert_eq!(config.cipher, SsrCipher::None);
    assert_eq!(config.protocol, SsrProtocol::Origin);
    assert_eq!(config.obfs, SsrObfs::Plain);
}

#[test]
fn rejects_unsupported_cipher() {
    let entry = parse_entry("name: s\ntype: ssr\nserver: s\nport: 1\ncipher: aes-256-gcm\npassword: p\n");
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
        [
            0x5f, 0x4d, 0xcc, 0x3b, 0x5a, 0xa7, 0x65, 0xd6, 0x1d, 0x83, 0x27, 0xde, 0xb8, 0x82, 0xcf, 0x99
        ]
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
