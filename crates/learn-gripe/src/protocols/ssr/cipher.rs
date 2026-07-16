//! SSR legacy (non-AEAD) stream-cipher layer.

use aes::Aes128;
use aes::Aes256;
use aes::cipher::{BlockEncrypt, KeyInit as AesKeyInit};
use aes_gcm::aead::generic_array::GenericArray;
use md5::Md5;

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
pub(super) enum StreamCryptor {
    Aes128Cfb(Box<Aes128CfbState>),
    Aes256Cfb(Box<Aes256CfbState>),
    Chacha20(Box<Chacha20State>),
    Rc4(Box<Rc4State>),
    None,
}

impl StreamCryptor {
    pub(super) fn new_encrypt(cipher: SsrCipher, key: &[u8], iv: &[u8]) -> Self {
        match cipher {
            SsrCipher::Aes128Cfb => StreamCryptor::Aes128Cfb(Box::new(Aes128CfbState::new(key, iv, true))),
            SsrCipher::Aes256Cfb => StreamCryptor::Aes256Cfb(Box::new(Aes256CfbState::new(key, iv, true))),
            SsrCipher::Chacha20Ietf => StreamCryptor::Chacha20(Box::new(Chacha20State::new(key, iv))),
            SsrCipher::Rc4Md5 => StreamCryptor::Rc4(Box::new(Rc4State::new(key, iv))),
            SsrCipher::None => StreamCryptor::None,
        }
    }

    pub(super) fn new_decrypt(cipher: SsrCipher, key: &[u8], iv: &[u8]) -> Self {
        match cipher {
            SsrCipher::Aes128Cfb => StreamCryptor::Aes128Cfb(Box::new(Aes128CfbState::new(key, iv, false))),
            SsrCipher::Aes256Cfb => StreamCryptor::Aes256Cfb(Box::new(Aes256CfbState::new(key, iv, false))),
            SsrCipher::Chacha20Ietf => StreamCryptor::Chacha20(Box::new(Chacha20State::new(key, iv))),
            SsrCipher::Rc4Md5 => StreamCryptor::Rc4(Box::new(Rc4State::new(key, iv))),
            SsrCipher::None => StreamCryptor::None,
        }
    }

    pub(super) fn update(&mut self, data: &mut [u8]) {
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

pub(super) struct Aes128CfbState {
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

pub(super) struct Aes256CfbState {
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

pub(super) struct Chacha20State {
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
pub(super) struct Rc4State {
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
            j = j.wrapping_add(s[i]).wrapping_add(derived[i % derived.len()]);
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
