pub struct SecureConfigStorage {
    encryption_key: Option<Vec<u8>>,
}

impl SecureConfigStorage {
    pub fn new() -> Self {
        Self {
            encryption_key: Self::load_key_from_env(),
        }
    }

    fn load_key_from_env() -> Option<Vec<u8>> {
        if let Ok(key_hex) = std::env::var("CLASH_VERGE_SECURE_KEY") {
            if let Ok(key) = hex::decode(key_hex) {
                return Some(key);
            }
        }
        None
    }

    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(ref key) = self.encryption_key {
            use aes_gcm::{
                Aes256Gcm, Nonce,
                aead::{Aead, KeyInit},
            };

            let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
            let nonce_bytes: [u8; 12] = rand::random();
            let nonce = Nonce::from_slice(&nonce_bytes);

            let ciphertext = cipher.encrypt(nonce, data).map_err(|e| e.to_string())?;

            let mut result = nonce_bytes.to_vec();
            result.extend_from_slice(&ciphertext);
            Ok(result)
        } else {
            Err("Encryption key is not configured".to_string())
        }
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(ref key) = self.encryption_key {
            use aes_gcm::{
                Aes256Gcm, Nonce,
                aead::{Aead, KeyInit},
            };

            if data.len() < 12 {
                return Err("Encrypted data is too short".to_string());
            }

            let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
            let nonce = Nonce::from_slice(&data[..12]);
            let ciphertext = &data[12..];

            cipher.decrypt(nonce, ciphertext).map_err(|e| e.to_string())
        } else {
            Err("Encryption key is not configured".to_string())
        }
    }

    pub fn is_key_available(&self) -> bool {
        self.encryption_key.is_some()
    }
}

impl Default for SecureConfigStorage {
    fn default() -> Self {
        Self::new()
    }
}

pub fn generate_encryption_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let key: Vec<u8> = (0..32).map(|_| rng.r#gen()).collect();
    hex::encode(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_key_generation() {
        let key = generate_encryption_key();
        assert_eq!(key.len(), 64);
    }

    #[test]
    fn test_secure_storage() {
        unsafe {
            std::env::set_var("CLASH_VERGE_SECURE_KEY", generate_encryption_key());
        }

        let storage = SecureConfigStorage::new();
        assert!(storage.is_key_available());

        let data = b"test data";
        let encrypted = storage.encrypt(data).unwrap();
        let decrypted = storage.decrypt(&encrypted).unwrap();

        assert_eq!(data, decrypted.as_slice());
    }
}
