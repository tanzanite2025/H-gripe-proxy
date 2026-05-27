/**
 * 配置文件欺骗模块
 * 
 * 生成假的配置文件来误导扫描软件
 */

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 假代理节点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeProxyNode {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub protocol: String,
    pub password: String,
}

/// 假配置文件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeConfig {
    pub version: String,
    pub proxies: Vec<FakeProxyNode>,
    pub note: String,
}

impl FakeConfig {
    /// 生成假配置
    pub fn generate() -> Self {
        Self {
            version: "1.0.0".to_string(),
            proxies: vec![
                FakeProxyNode {
                    name: "HK-Expired-Node-1".to_string(),
                    server: "expired-hk1.example.com".to_string(),
                    port: 8388,
                    protocol: "ss".to_string(),
                    password: "fake_password_123456".to_string(),
                },
                FakeProxyNode {
                    name: "US-Expired-Node-2".to_string(),
                    server: "expired-us1.example.com".to_string(),
                    port: 443,
                    protocol: "vmess".to_string(),
                    password: "00000000-0000-0000-0000-000000000000".to_string(),
                },
                FakeProxyNode {
                    name: "JP-Expired-Node-3".to_string(),
                    server: "expired-jp1.example.com".to_string(),
                    port: 10086,
                    protocol: "trojan".to_string(),
                    password: "expired_trojan_password".to_string(),
                },
                FakeProxyNode {
                    name: "SG-Test-Node".to_string(),
                    server: "test-sg.example.com".to_string(),
                    port: 8080,
                    protocol: "http".to_string(),
                    password: "test123".to_string(),
                },
            ],
            note: "This is a decoy configuration file. Real configuration is encrypted in memory.".to_string(),
        }
    }

    /// 保存到文件
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), String> {
        let yaml = serde_yaml_ng::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(path, yaml).map_err(|e| e.to_string())?;
        Ok(())
    }
}

/// 配置欺骗管理器
pub struct ConfigDecoy {
    decoy_path: PathBuf,
    enabled: bool,
}

impl ConfigDecoy {
    /// 创建新的配置欺骗管理器
    pub fn new(decoy_path: PathBuf) -> Self {
        Self {
            decoy_path,
            enabled: true,
        }
    }

    /// 部署假配置文件
    pub fn deploy(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        let fake_config = FakeConfig::generate();
        fake_config.save_to_file(&self.decoy_path)?;

        log::info!("✅ 假配置文件已部署到: {:?}", self.decoy_path);
        Ok(())
    }

    /// 清除假配置文件
    pub fn cleanup(&self) -> Result<(), String> {
        if self.decoy_path.exists() {
            std::fs::remove_file(&self.decoy_path).map_err(|e| e.to_string())?;
            log::info!("✅ 假配置文件已清除");
        }
        Ok(())
    }

    /// 检查假配置是否被访问
    pub fn check_access(&self) -> bool {
        if !self.decoy_path.exists() {
            return false;
        }

        // 检查文件的访问时间
        if let Ok(metadata) = std::fs::metadata(&self.decoy_path) {
            if let Ok(accessed) = metadata.accessed() {
                if let Ok(modified) = metadata.modified() {
                    // 如果访问时间晚于修改时间，说明文件被读取过
                    if accessed > modified {
                        log::warn!("🚨 假配置文件被访问！");
                        return true;
                    }
                }
            }
        }

        false
    }
}

/// 真实配置加密存储
pub struct SecureConfigStorage {
    /// 加密密钥（从环境变量或内存注入）
    encryption_key: Option<Vec<u8>>,
}

impl SecureConfigStorage {
    pub fn new() -> Self {
        Self {
            encryption_key: Self::load_key_from_env(),
        }
    }

    /// 从环境变量加载密钥
    fn load_key_from_env() -> Option<Vec<u8>> {
        // 从环境变量读取加密密钥
        if let Ok(key_hex) = std::env::var("CLASH_VERGE_SECURE_KEY") {
            if let Ok(key) = hex::decode(key_hex) {
                return Some(key);
            }
        }
        None
    }

    /// 加密配置
    pub fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(ref key) = self.encryption_key {
            // 使用 AES-256-GCM 加密
            use aes_gcm::{
                aead::{Aead, KeyInit},
                Aes256Gcm, Nonce,
            };

            let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
            
            // 生成随机 nonce
            let nonce_bytes: [u8; 12] = rand::random();
            let nonce = Nonce::from_slice(&nonce_bytes);

            let ciphertext = cipher
                .encrypt(nonce, data)
                .map_err(|e| e.to_string())?;

            // 返回 nonce + ciphertext
            let mut result = nonce_bytes.to_vec();
            result.extend_from_slice(&ciphertext);
            Ok(result)
        } else {
            Err("加密密钥未设置".to_string())
        }
    }

    /// 解密配置
    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, String> {
        if let Some(ref key) = self.encryption_key {
            use aes_gcm::{
                aead::{Aead, KeyInit},
                Aes256Gcm, Nonce,
            };

            if data.len() < 12 {
                return Err("数据太短".to_string());
            }

            let cipher = Aes256Gcm::new_from_slice(key).map_err(|e| e.to_string())?;
            
            let nonce = Nonce::from_slice(&data[..12]);
            let ciphertext = &data[12..];

            let plaintext = cipher
                .decrypt(nonce, ciphertext)
                .map_err(|e| e.to_string())?;

            Ok(plaintext)
        } else {
            Err("加密密钥未设置".to_string())
        }
    }

    /// 检查密钥是否可用
    pub fn is_key_available(&self) -> bool {
        self.encryption_key.is_some()
    }
}

impl Default for SecureConfigStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// 生成随机加密密钥
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
    fn test_fake_config_generation() {
        let config = FakeConfig::generate();
        assert_eq!(config.version, "1.0.0");
        assert!(!config.proxies.is_empty());
    }

    #[test]
    fn test_encryption_key_generation() {
        let key = generate_encryption_key();
        assert_eq!(key.len(), 64); // 32 bytes = 64 hex chars
    }

    #[test]
    fn test_secure_storage() {
        // 设置测试密钥
        std::env::set_var("CLASH_VERGE_SECURE_KEY", generate_encryption_key());
        
        let storage = SecureConfigStorage::new();
        assert!(storage.is_key_available());

        let data = b"test data";
        let encrypted = storage.encrypt(data).unwrap();
        let decrypted = storage.decrypt(&encrypted).unwrap();
        
        assert_eq!(data, decrypted.as_slice());
    }
}
