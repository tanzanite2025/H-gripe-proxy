/**
 * 反主动探测模块
 * 
 * 功能：
 * 1. 幻影无响应（Drop on Probe）
 * 2. 严格白名单机制
 * 3. 基于时间戳和私钥的握手暗号
 */

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

/// 反探测配置
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AntiProbeConfig {
    /// 启用反探测
    pub enabled: bool,
    /// 私钥（用于生成握手暗号）
    pub secret_key: String,
    /// 时间窗口（秒）
    pub time_window: u64,
    /// 白名单 IP
    pub whitelist: Vec<IpAddr>,
    /// 是否启用严格模式（非白名单直接丢弃）
    pub strict_mode: bool,
}

impl Default for AntiProbeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            secret_key: generate_random_key(),
            time_window: 300, // 5 分钟
            whitelist: Vec::new(),
            strict_mode: false,
        }
    }
}

/// 反探测服务
pub struct AntiProbeService {
    config: Arc<RwLock<AntiProbeConfig>>,
    /// 已验证的连接缓存
    verified_connections: Arc<RwLock<HashMap<String, u64>>>,
}

impl AntiProbeService {
    pub fn new(config: AntiProbeConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            verified_connections: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 验证握手暗号
    pub fn verify_handshake(&self, client_ip: &IpAddr, token: &str) -> bool {
        let config = self.config.read().unwrap();

        if !config.enabled {
            return true; // 未启用时允许所有连接
        }

        // 检查白名单
        if config.whitelist.contains(client_ip) {
            return true;
        }

        // 验证 token
        if self.verify_token(token, &config.secret_key, config.time_window) {
            // 缓存已验证的连接
            let key = format!("{}", client_ip);
            let now = current_timestamp();
            self.verified_connections
                .write()
                .unwrap()
                .insert(key, now);
            return true;
        }

        // 严格模式下，未验证的连接直接拒绝
        if config.strict_mode {
            return false;
        }

        // 检查是否已验证过
        let key = format!("{}", client_ip);
        if let Some(&verified_time) = self.verified_connections.read().unwrap().get(&key) {
            let now = current_timestamp();
            if now - verified_time < config.time_window {
                return true;
            }
        }

        false
    }

    /// 生成握手暗号
    pub fn generate_token(&self) -> String {
        let config = self.config.read().unwrap();
        let timestamp = current_timestamp();
        generate_token(&config.secret_key, timestamp)
    }

    /// 验证 token
    fn verify_token(&self, token: &str, secret_key: &str, time_window: u64) -> bool {
        let now = current_timestamp();

        // 尝试在时间窗口内的所有可能时间戳
        for offset in 0..=time_window {
            let test_timestamp = now - offset;
            let expected_token = generate_token(secret_key, test_timestamp);
            if token == expected_token {
                return true;
            }
        }

        false
    }

    /// 清理过期的验证缓存
    pub fn cleanup_expired(&self) {
        let config = self.config.read().unwrap();
        let now = current_timestamp();
        let mut connections = self.verified_connections.write().unwrap();

        connections.retain(|_, &mut verified_time| {
            now - verified_time < config.time_window
        });
    }

    /// 更新配置
    pub fn update_config(&self, config: AntiProbeConfig) {
        *self.config.write().unwrap() = config;
    }

    /// 获取配置
    pub fn get_config(&self) -> AntiProbeConfig {
        self.config.read().unwrap().clone()
    }
}

/// 生成握手 token
fn generate_token(secret_key: &str, timestamp: u64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret_key.as_bytes());
    hasher.update(timestamp.to_string().as_bytes());
    let result = hasher.finalize();
    hex::encode(result)
}

/// 生成随机密钥
fn generate_random_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let key: Vec<u8> = (0..32).map(|_| rng.r#gen()).collect();
    hex::encode(key)
}

/// 获取当前时间戳（秒）
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let secret = "test_secret";
        let timestamp = 1234567890;
        let token = generate_token(secret, timestamp);
        assert!(!token.is_empty());
    }

    #[test]
    fn test_token_verification() {
        let config = AntiProbeConfig {
            enabled: true,
            secret_key: "test_secret".to_string(),
            time_window: 60,
            whitelist: Vec::new(),
            strict_mode: false,
        };

        let service = AntiProbeService::new(config);
        let token = service.generate_token();
        let ip = "127.0.0.1".parse().unwrap();

        assert!(service.verify_handshake(&ip, &token));
    }

    #[test]
    fn test_whitelist() {
        let config = AntiProbeConfig {
            enabled: true,
            secret_key: "test_secret".to_string(),
            time_window: 60,
            whitelist: vec!["127.0.0.1".parse().unwrap()],
            strict_mode: true,
        };

        let service = AntiProbeService::new(config);
        let ip = "127.0.0.1".parse().unwrap();

        // 白名单 IP 应该直接通过
        assert!(service.verify_handshake(&ip, "invalid_token"));
    }
}
