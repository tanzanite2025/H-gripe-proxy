use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeProxyNode {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub protocol: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FakeConfig {
    pub version: String,
    pub proxies: Vec<FakeProxyNode>,
    pub note: String,
}

impl FakeConfig {
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

    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), String> {
        let yaml = serde_yaml_ng::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(path, yaml).map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct ConfigDecoy {
    decoy_path: PathBuf,
    enabled: bool,
}

impl ConfigDecoy {
    pub fn new(decoy_path: PathBuf) -> Self {
        Self {
            decoy_path,
            enabled: true,
        }
    }

    pub fn deploy(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }

        let fake_config = FakeConfig::generate();
        fake_config.save_to_file(&self.decoy_path)?;

        log::info!("Fake config decoy deployed to {:?}", self.decoy_path);
        Ok(())
    }

    pub fn cleanup(&self) -> Result<(), String> {
        if self.decoy_path.exists() {
            std::fs::remove_file(&self.decoy_path).map_err(|e| e.to_string())?;
            log::info!("Fake config decoy cleaned up");
        }
        Ok(())
    }

    pub fn check_access(&self) -> bool {
        if !self.decoy_path.exists() {
            return false;
        }

        if let Ok(metadata) = std::fs::metadata(&self.decoy_path) {
            if let Ok(accessed) = metadata.accessed() {
                if let Ok(modified) = metadata.modified() {
                    if accessed > modified {
                        log::warn!("Fake config decoy was accessed");
                        return true;
                    }
                }
            }
        }

        false
    }
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
}
