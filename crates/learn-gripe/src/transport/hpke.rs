//! RFC 9180 HPKE provider for Encrypted Client Hello (ECH).
//!
//! rustls only ships an [`Hpke`] implementation under its `aws-lc-rs` backend,
//! but learn-gripe builds rustls with the pure-Rust `ring` backend so it stays
//! free of a C toolchain. ECH (`with_ech`) nonetheless needs an HPKE provider,
//! so this module composes one from the vetted RustCrypto primitives already
//! used elsewhere in the kernel — X25519 key agreement (`x25519-dalek`),
//! HKDF-SHA256 (`hkdf` + `sha2`) and the AEAD crates (`aes-gcm`,
//! `chacha20poly1305`). Only the RFC 9180 *key schedule* is assembled here; the
//! cryptographic primitives themselves are delegated to those crates.
//!
//! Scope: the single KEM that real-world ECH deployments use — DHKEM(X25519,
//! HKDF-SHA256) — paired with HKDF-SHA256 and the three standard AEADs
//! (AES-128-GCM, AES-256-GCM, ChaCha20Poly1305). The NIST P-curve KEMs are not
//! implemented; [`rustls::client::EchConfig::new`] simply skips configs whose
//! suite is unsupported. The construction is checked against the RFC 9180
//! Appendix A.1 base-mode test vector in the unit tests.

use aes_gcm::aead::{self, Aead, KeyInit, Payload};
use aes_gcm::{Aes128Gcm, Aes256Gcm};
use chacha20poly1305::ChaCha20Poly1305;
use hkdf::Hkdf;
use rustls::Error;
use rustls::crypto::hpke::{
    EncapsulatedSecret, Hpke, HpkeOpener, HpkePrivateKey, HpkePublicKey, HpkeSealer, HpkeSuite,
};
use rustls::internal::msgs::enums::{HpkeAead, HpkeKdf, HpkeKem};
use rustls::internal::msgs::handshake::HpkeSymmetricCipherSuite;
use sha2::Sha256;
use x25519_dalek::{X25519_BASEPOINT_BYTES, x25519};

/// HKDF-SHA256 output length (`Nh`), also the X25519 KEM shared-secret length
/// (`Nsecret`).
const NH: usize = 32;
/// X25519 public/private key and encapsulated-key length (`Npk` = `Nenc`).
const NPK: usize = 32;
/// AEAD nonce length (`Nn`) for all three supported AEADs.
const NN: usize = 12;

/// The HPKE suites this provider implements, for
/// [`rustls::client::EchConfig::new`].
pub static ALL_SUPPORTED_SUITES: &[&dyn Hpke] = &[
    &HPKE_X25519_SHA256_AES128,
    &HPKE_X25519_SHA256_AES256,
    &HPKE_X25519_SHA256_CHACHA20,
];

static HPKE_X25519_SHA256_AES128: HpkeX25519 = HpkeX25519::new(AeadAlg::Aes128Gcm, HpkeAead::AES_128_GCM);
static HPKE_X25519_SHA256_AES256: HpkeX25519 = HpkeX25519::new(AeadAlg::Aes256Gcm, HpkeAead::AES_256_GCM);
static HPKE_X25519_SHA256_CHACHA20: HpkeX25519 =
    HpkeX25519::new(AeadAlg::ChaCha20Poly1305, HpkeAead::CHACHA20_POLY_1305);

/// The AEAD half of an HPKE suite. The KDF (HKDF-SHA256) and KEM (X25519) are
/// fixed for every suite this provider offers, so only the AEAD varies.
#[derive(Clone, Copy, Debug)]
enum AeadAlg {
    Aes128Gcm,
    Aes256Gcm,
    ChaCha20Poly1305,
}

impl AeadAlg {
    /// AEAD key length (`Nk`).
    const fn key_len(self) -> usize {
        match self {
            Self::Aes128Gcm => 16,
            Self::Aes256Gcm | Self::ChaCha20Poly1305 => 32,
        }
    }

    fn seal(self, key: &[u8], nonce: &[u8; NN], aad: &[u8], pt: &[u8]) -> Result<Vec<u8>, Error> {
        match self {
            Self::Aes128Gcm => aead_seal::<Aes128Gcm>(key, nonce, aad, pt),
            Self::Aes256Gcm => aead_seal::<Aes256Gcm>(key, nonce, aad, pt),
            Self::ChaCha20Poly1305 => aead_seal::<ChaCha20Poly1305>(key, nonce, aad, pt),
        }
    }

    fn open(self, key: &[u8], nonce: &[u8; NN], aad: &[u8], ct: &[u8]) -> Result<Vec<u8>, Error> {
        match self {
            Self::Aes128Gcm => aead_open::<Aes128Gcm>(key, nonce, aad, ct),
            Self::Aes256Gcm => aead_open::<Aes256Gcm>(key, nonce, aad, ct),
            Self::ChaCha20Poly1305 => aead_open::<ChaCha20Poly1305>(key, nonce, aad, ct),
        }
    }
}

fn aead_seal<C: KeyInit + Aead>(key: &[u8], nonce: &[u8; NN], aad: &[u8], pt: &[u8]) -> Result<Vec<u8>, Error> {
    let cipher = C::new_from_slice(key).map_err(|_| err("hpke: bad AEAD key length"))?;
    let nonce = aead::Nonce::<C>::from_slice(nonce);
    cipher
        .encrypt(nonce, Payload { msg: pt, aad })
        .map_err(|_| err("hpke: AEAD seal failed"))
}

fn aead_open<C: KeyInit + Aead>(key: &[u8], nonce: &[u8; NN], aad: &[u8], ct: &[u8]) -> Result<Vec<u8>, Error> {
    let cipher = C::new_from_slice(key).map_err(|_| err("hpke: bad AEAD key length"))?;
    let nonce = aead::Nonce::<C>::from_slice(nonce);
    cipher
        .decrypt(nonce, Payload { msg: ct, aad })
        .map_err(|_| err("hpke: AEAD open failed"))
}

/// An HPKE suite over DHKEM(X25519, HKDF-SHA256) + HKDF-SHA256 + `aead`.
#[derive(Debug)]
struct HpkeX25519 {
    suite: HpkeSuite,
    aead: AeadAlg,
}

impl HpkeX25519 {
    const fn new(aead: AeadAlg, aead_id: HpkeAead) -> Self {
        Self {
            suite: HpkeSuite {
                kem: HpkeKem::DHKEM_X25519_HKDF_SHA256,
                sym: HpkeSymmetricCipherSuite {
                    kdf_id: HpkeKdf::HKDF_SHA256,
                    aead_id,
                },
            },
            aead,
        }
    }

    /// "HPKE" suite-id used to label the key-schedule extracts/expands
    /// (RFC 9180 §5.1).
    fn hpke_suite_id(&self) -> [u8; 10] {
        let mut id = [0u8; 10];
        id[..4].copy_from_slice(b"HPKE");
        id[4..6].copy_from_slice(&u16::from(self.suite.kem).to_be_bytes());
        id[6..8].copy_from_slice(&u16::from(self.suite.sym.kdf_id).to_be_bytes());
        id[8..10].copy_from_slice(&u16::from(self.suite.sym.aead_id).to_be_bytes());
        id
    }

    /// Derive the AEAD key and base nonce for base-mode HPKE (RFC 9180 §5.1
    /// "KeyScheduleS/R", with an empty PSK).
    fn key_schedule(&self, shared_secret: &[u8; NH], info: &[u8]) -> KeySchedule {
        let suite_id = self.hpke_suite_id();
        let psk_id_hash = labeled_extract(&[], &suite_id, b"psk_id_hash", &[]);
        let info_hash = labeled_extract(&[], &suite_id, b"info_hash", info);

        let mut ks_context = Vec::with_capacity(1 + 2 * NH);
        ks_context.push(0x00); // mode_base
        ks_context.extend_from_slice(&psk_id_hash);
        ks_context.extend_from_slice(&info_hash);

        let secret = labeled_extract(shared_secret, &suite_id, b"secret", &[]);

        let mut key = vec![0u8; self.aead.key_len()];
        labeled_expand(&secret, &suite_id, b"key", &ks_context, &mut key);

        let mut base_nonce = [0u8; NN];
        labeled_expand(&secret, &suite_id, b"base_nonce", &ks_context, &mut base_nonce);

        KeySchedule {
            aead: self.aead,
            key,
            base_nonce,
            seq: 0,
        }
    }

    /// DHKEM(X25519) encapsulation (RFC 9180 §4.1): generate an ephemeral key,
    /// agree with `pk_r`, and derive the shared secret. Returns the shared
    /// secret and the serialized ephemeral public key (`enc`).
    fn encap(&self, pk_r: &[u8; NPK]) -> Result<([u8; NH], [u8; NPK]), Error> {
        let mut sk_e = [0u8; NPK];
        random_bytes(&mut sk_e)?;
        let result = self.encap_with(pk_r, &sk_e);
        sk_e.fill(0);
        Ok(result)
    }

    fn encap_with(&self, pk_r: &[u8; NPK], sk_e: &[u8; NPK]) -> ([u8; NH], [u8; NPK]) {
        let enc = x25519(*sk_e, X25519_BASEPOINT_BYTES);
        let dh = x25519(*sk_e, *pk_r);
        let mut kem_context = [0u8; 2 * NPK];
        kem_context[..NPK].copy_from_slice(&enc);
        kem_context[NPK..].copy_from_slice(pk_r);
        let shared_secret = extract_and_expand(&dh, &kem_context);
        (shared_secret, enc)
    }

    /// DHKEM(X25519) decapsulation (RFC 9180 §4.1).
    fn decap(&self, enc: &[u8; NPK], sk_r: &[u8; NPK]) -> [u8; NH] {
        let dh = x25519(*sk_r, *enc);
        let pk_r = x25519(*sk_r, X25519_BASEPOINT_BYTES);
        let mut kem_context = [0u8; 2 * NPK];
        kem_context[..NPK].copy_from_slice(enc);
        kem_context[NPK..].copy_from_slice(&pk_r);
        extract_and_expand(&dh, &kem_context)
    }
}

impl Hpke for HpkeX25519 {
    fn seal(
        &self,
        info: &[u8],
        aad: &[u8],
        plaintext: &[u8],
        pub_key: &HpkePublicKey,
    ) -> Result<(EncapsulatedSecret, Vec<u8>), Error> {
        let (encap, mut sealer) = self.setup_sealer(info, pub_key)?;
        Ok((encap, sealer.seal(aad, plaintext)?))
    }

    fn setup_sealer(
        &self,
        info: &[u8],
        pub_key: &HpkePublicKey,
    ) -> Result<(EncapsulatedSecret, Box<dyn HpkeSealer + 'static>), Error> {
        let pk_r = fixed_key(&pub_key.0)?;
        let (shared_secret, enc) = self.encap(&pk_r)?;
        let key_schedule = self.key_schedule(&shared_secret, info);
        Ok((EncapsulatedSecret(enc.to_vec()), Box::new(key_schedule)))
    }

    fn open(
        &self,
        enc: &EncapsulatedSecret,
        info: &[u8],
        aad: &[u8],
        ciphertext: &[u8],
        secret_key: &HpkePrivateKey,
    ) -> Result<Vec<u8>, Error> {
        let mut opener = self.setup_opener(enc, info, secret_key)?;
        opener.open(aad, ciphertext)
    }

    fn setup_opener(
        &self,
        enc: &EncapsulatedSecret,
        info: &[u8],
        secret_key: &HpkePrivateKey,
    ) -> Result<Box<dyn HpkeOpener + 'static>, Error> {
        let enc = fixed_key(&enc.0)?;
        let sk_r = fixed_key(secret_key.secret_bytes())?;
        let shared_secret = self.decap(&enc, &sk_r);
        Ok(Box::new(self.key_schedule(&shared_secret, info)))
    }

    fn generate_key_pair(&self) -> Result<(HpkePublicKey, HpkePrivateKey), Error> {
        let mut sk = [0u8; NPK];
        random_bytes(&mut sk)?;
        let pk = x25519(sk, X25519_BASEPOINT_BYTES);
        let pair = (HpkePublicKey(pk.to_vec()), HpkePrivateKey::from(sk.to_vec()));
        sk.fill(0);
        Ok(pair)
    }

    fn suite(&self) -> HpkeSuite {
        self.suite
    }
}

/// The per-context AEAD state shared by an HPKE sealer and opener: the derived
/// key, the base nonce, and the message sequence number (RFC 9180 §5.2).
#[derive(Debug)]
struct KeySchedule {
    aead: AeadAlg,
    key: Vec<u8>,
    base_nonce: [u8; NN],
    seq: u64,
}

impl KeySchedule {
    /// `ComputeNonce(seq) = base_nonce XOR I2OSP(seq, Nn)`.
    fn compute_nonce(&self) -> [u8; NN] {
        let mut nonce = self.base_nonce;
        let seq_bytes = self.seq.to_be_bytes();
        for (n, &b) in nonce[NN - seq_bytes.len()..].iter_mut().zip(&seq_bytes) {
            *n ^= b;
        }
        nonce
    }
}

impl HpkeSealer for KeySchedule {
    fn seal(&mut self, aad: &[u8], plaintext: &[u8]) -> Result<Vec<u8>, Error> {
        let nonce = self.compute_nonce();
        let ct = self.aead.seal(&self.key, &nonce, aad, plaintext)?;
        self.seq += 1;
        Ok(ct)
    }
}

impl HpkeOpener for KeySchedule {
    fn open(&mut self, aad: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, Error> {
        let nonce = self.compute_nonce();
        let pt = self.aead.open(&self.key, &nonce, aad, ciphertext)?;
        self.seq += 1;
        Ok(pt)
    }
}

/// `ExtractAndExpand(dh, kem_context)` for DHKEM (RFC 9180 §4.1), labeled with
/// the "KEM" suite id.
fn extract_and_expand(dh: &[u8], kem_context: &[u8]) -> [u8; NH] {
    let mut suite_id = [0u8; 5];
    suite_id[..3].copy_from_slice(b"KEM");
    suite_id[3..5].copy_from_slice(&u16::from(HpkeKem::DHKEM_X25519_HKDF_SHA256).to_be_bytes());
    let eae_prk = labeled_extract(&[], &suite_id, b"eae_prk", dh);
    let mut shared_secret = [0u8; NH];
    labeled_expand(&eae_prk, &suite_id, b"shared_secret", kem_context, &mut shared_secret);
    shared_secret
}

/// `LabeledExtract(salt, label, ikm)` (RFC 9180 §4), returning the PRK.
fn labeled_extract(salt: &[u8], suite_id: &[u8], label: &[u8], ikm: &[u8]) -> [u8; NH] {
    let mut labeled_ikm = Vec::with_capacity(7 + suite_id.len() + label.len() + ikm.len());
    labeled_ikm.extend_from_slice(b"HPKE-v1");
    labeled_ikm.extend_from_slice(suite_id);
    labeled_ikm.extend_from_slice(label);
    labeled_ikm.extend_from_slice(ikm);
    let (prk, _) = Hkdf::<Sha256>::extract(Some(salt), &labeled_ikm);
    let mut out = [0u8; NH];
    out.copy_from_slice(&prk);
    out
}

/// `LabeledExpand(prk, label, info, L)` (RFC 9180 §4).
fn labeled_expand(prk: &[u8], suite_id: &[u8], label: &[u8], info: &[u8], out: &mut [u8]) {
    let mut labeled_info = Vec::with_capacity(2 + 7 + suite_id.len() + label.len() + info.len());
    labeled_info.extend_from_slice(&(out.len() as u16).to_be_bytes());
    labeled_info.extend_from_slice(b"HPKE-v1");
    labeled_info.extend_from_slice(suite_id);
    labeled_info.extend_from_slice(label);
    labeled_info.extend_from_slice(info);
    let hk = Hkdf::<Sha256>::from_prk(prk).expect("HKDF-SHA256 PRK is exactly Nh bytes");
    hk.expand(&labeled_info, out)
        .expect("HKDF-SHA256 expand length within 255*Nh");
}

/// Copy a slice that must be exactly an X25519 key length into a fixed array.
fn fixed_key(bytes: &[u8]) -> Result<[u8; NPK], Error> {
    bytes.try_into().map_err(|_| err("hpke: X25519 key must be 32 bytes"))
}

fn random_bytes(buf: &mut [u8]) -> Result<(), Error> {
    getrandom::fill(buf).map_err(|_| err("hpke: system RNG unavailable"))
}

fn err(msg: &str) -> Error {
    Error::General(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unhex(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }

    /// RFC 9180 Appendix A.1: DHKEM(X25519, HKDF-SHA256), HKDF-SHA256,
    /// AES-128-GCM, base mode. Drives the key schedule from the published
    /// ephemeral key and checks `enc`, the derived key/base-nonce, and the
    /// first two sealed records against the official vector.
    #[test]
    fn rfc9180_a1_base_mode_vector() {
        let info = unhex("4f6465206f6e2061204772656369616e2055726e");
        let sk_em = unhex("52c4a758a802cd8b936eceea314432798d5baf2d7e9235dc084ab1b9cfa2f736");
        let pk_rm = unhex("3948cfe0ad1ddb695d780e59077195da6c56506b027329794ab02bca80815c4d");
        let expected_enc = unhex("37fda3567bdbd628e88668c3c8d7e97d1d1253b6d4ea6d44c150f741f1bf4431");
        let expected_key = unhex("4531685d41d65f03dc48f6b8302c05b0");
        let expected_base_nonce = unhex("56d890e5accaaf011cff4b7d");

        let hpke = &HPKE_X25519_SHA256_AES128;
        let pk_r = fixed_key(&pk_rm).unwrap();
        let sk_e = fixed_key(&sk_em).unwrap();

        let (shared_secret, enc) = hpke.encap_with(&pk_r, &sk_e);
        assert_eq!(enc.to_vec(), expected_enc, "enc mismatch");

        let mut ks = hpke.key_schedule(&shared_secret, &info);
        assert_eq!(ks.key, expected_key, "derived key mismatch");
        assert_eq!(ks.base_nonce.to_vec(), expected_base_nonce, "base nonce mismatch");

        // Sequence 0 and 1 ciphertexts from the vector's encryptions.
        let seq0_aad = unhex("436f756e742d30");
        let seq0_pt = unhex("4265617574792069732074727574682c20747275746820626561757479");
        let seq0_ct =
            unhex("f938558b5d72f1a23810b4be2ab4f84331acc02fc97babc53a52ae8218a355a96d8770ac83d07bea87e13c512a");
        assert_eq!(ks.seal(&seq0_aad, &seq0_pt).unwrap(), seq0_ct, "seq 0 ct mismatch");

        let seq1_aad = unhex("436f756e742d31");
        let seq1_pt = unhex("4265617574792069732074727574682c20747275746820626561757479");
        let seq1_ct =
            unhex("af2d7e9ac9ae7e270f46ba1f975be53c09f8d875bdc8535458c2494e8a6eab251c03d0c22a56b8ca42c2063b84");
        assert_eq!(ks.seal(&seq1_aad, &seq1_pt).unwrap(), seq1_ct, "seq 1 ct mismatch");
    }

    /// Each supported suite round-trips a sealed message through a freshly
    /// generated key pair, and a wrong AAD fails to open.
    #[test]
    fn round_trip_all_suites() {
        for hpke in ALL_SUPPORTED_SUITES {
            let (pk, sk) = hpke.generate_key_pair().unwrap();
            let info = b"learn-gripe ech";
            let aad = b"aad";
            let pt = b"the quick brown fox";

            let (enc, ct) = hpke.seal(info, aad, pt, &pk).unwrap();
            let recovered = hpke.open(&enc, info, aad, &ct, &sk).unwrap();
            assert_eq!(recovered, pt);

            assert!(hpke.open(&enc, info, b"wrong", &ct, &sk).is_err());
        }
    }

    /// Multiple seals advance the sequence number, so each record uses a
    /// distinct nonce and the opener must decrypt them in order.
    #[test]
    fn sequence_number_advances() {
        let hpke = &HPKE_X25519_SHA256_CHACHA20;
        let (pk, sk) = hpke.generate_key_pair().unwrap();
        let info = b"info";

        let (enc, mut sealer) = hpke.setup_sealer(info, &pk).unwrap();
        let c0 = sealer.seal(b"", b"m0").unwrap();
        let c1 = sealer.seal(b"", b"m1").unwrap();
        assert_ne!(c0, c1);

        let mut opener = hpke.setup_opener(&enc, info, &sk).unwrap();
        assert_eq!(opener.open(b"", &c0).unwrap(), b"m0");
        assert_eq!(opener.open(b"", &c1).unwrap(), b"m1");
    }
}
