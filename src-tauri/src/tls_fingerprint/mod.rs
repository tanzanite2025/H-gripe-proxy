/**
 * TLS 指纹伪装模块（Parrot Mode）
 * 
 * 与 Mihomo 内核 component/tls/utls.go 的 fingerprints map 完全对齐。
 * 指纹名称即为 Mihomo 配置中 `global-client-fingerprint` / `client-fingerprint` 的合法值。
 */

use serde::{Deserialize, Serialize};

/// TLS 指纹配置
/// name 为 Mihomo 配置值，description 供 UI 展示
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsFingerprint {
    /// Mihomo 配置值（如 "chrome", "firefox"）
    pub name: String,
    /// UI 展示描述
    pub description: String,
    /// 分类（浏览器 / 移动端 / 随机 / 经典）
    pub category: String,
}

/// 预定义的 TLS 指纹库 — 与 Mihomo utls.go fingerprints map 1:1 对齐
pub struct TlsFingerprintLibrary;

impl TlsFingerprintLibrary {
    fn chrome() -> TlsFingerprint {
        TlsFingerprint {
            name: "chrome".to_string(),
            description: "Chrome (Auto)".to_string(),
            category: "browser".to_string(),
        }
    }

    fn firefox() -> TlsFingerprint {
        TlsFingerprint {
            name: "firefox".to_string(),
            description: "Firefox (Auto)".to_string(),
            category: "browser".to_string(),
        }
    }

    fn safari() -> TlsFingerprint {
        TlsFingerprint {
            name: "safari".to_string(),
            description: "Safari (Auto)".to_string(),
            category: "browser".to_string(),
        }
    }

    fn ios() -> TlsFingerprint {
        TlsFingerprint {
            name: "ios".to_string(),
            description: "iOS Safari (Auto)".to_string(),
            category: "mobile".to_string(),
        }
    }

    fn android() -> TlsFingerprint {
        TlsFingerprint {
            name: "android".to_string(),
            description: "Android OkHttp".to_string(),
            category: "mobile".to_string(),
        }
    }

    fn edge() -> TlsFingerprint {
        TlsFingerprint {
            name: "edge".to_string(),
            description: "Edge (Auto)".to_string(),
            category: "browser".to_string(),
        }
    }

    fn q360() -> TlsFingerprint {
        TlsFingerprint {
            name: "360".to_string(),
            description: "360 Browser (Auto)".to_string(),
            category: "browser".to_string(),
        }
    }

    fn qq() -> TlsFingerprint {
        TlsFingerprint {
            name: "qq".to_string(),
            description: "QQ Browser (Auto)".to_string(),
            category: "browser".to_string(),
        }
    }

    fn random() -> TlsFingerprint {
        TlsFingerprint {
            name: "random".to_string(),
            description: "Random (Weighted)".to_string(),
            category: "random".to_string(),
        }
    }

    fn randomized() -> TlsFingerprint {
        TlsFingerprint {
            name: "randomized".to_string(),
            description: "Randomized (Full)".to_string(),
            category: "random".to_string(),
        }
    }

    fn chrome120() -> TlsFingerprint {
        TlsFingerprint {
            name: "chrome120".to_string(),
            description: "Chrome 120 (Classic)".to_string(),
            category: "classic".to_string(),
        }
    }

    fn firefox120() -> TlsFingerprint {
        TlsFingerprint {
            name: "firefox120".to_string(),
            description: "Firefox 120 (Classic)".to_string(),
            category: "classic".to_string(),
        }
    }

    fn safari16() -> TlsFingerprint {
        TlsFingerprint {
            name: "safari16".to_string(),
            description: "Safari 16 (Classic)".to_string(),
            category: "classic".to_string(),
        }
    }

    /// 获取所有预定义指纹（与 Mihomo utls.go fingerprints map 一致）
    pub fn get_all() -> Vec<TlsFingerprint> {
        vec![
            Self::chrome(),
            Self::firefox(),
            Self::safari(),
            Self::ios(),
            Self::android(),
            Self::edge(),
            Self::q360(),
            Self::qq(),
            Self::random(),
            Self::randomized(),
            Self::chrome120(),
            Self::firefox120(),
            Self::safari16(),
        ]
    }

    /// 根据名称获取指纹
    pub fn get_by_name(name: &str) -> Option<TlsFingerprint> {
        Self::get_all().into_iter().find(|fp| fp.name == name)
    }

    /// 检查名称是否为合法的 Mihomo 指纹值
    pub fn is_valid(name: &str) -> bool {
        Self::get_all().iter().any(|fp| fp.name == name)
    }
}

/// TLS 指纹伪装服务
pub struct TlsFingerprintService {
    current_fingerprint: parking_lot::RwLock<Option<TlsFingerprint>>,
}

impl TlsFingerprintService {
    pub fn new() -> Self {
        Self {
            current_fingerprint: parking_lot::RwLock::new(None),
        }
    }

    /// 设置当前指纹
    pub fn set_fingerprint(&self, fingerprint: TlsFingerprint) {
        *self.current_fingerprint.write() = Some(fingerprint);
    }

    /// 根据名称设置指纹
    pub fn set_by_name(&self, name: &str) -> Result<(), String> {
        if let Some(fp) = TlsFingerprintLibrary::get_by_name(name) {
            self.set_fingerprint(fp);
            Ok(())
        } else {
            Err(format!("未找到指纹: {}", name))
        }
    }

    /// 获取当前指纹
    pub fn get_fingerprint(&self) -> Option<TlsFingerprint> {
        self.current_fingerprint.read().clone()
    }

    /// 清除当前指纹
    pub fn clear(&self) {
        *self.current_fingerprint.write() = None;
    }

    /// 生成 Clash 配置 — 输出 Mihomo 合法的 global-client-fingerprint 值
    pub fn generate_clash_config(&self) -> Option<serde_json::Value> {
        self.current_fingerprint.read().as_ref().map(|fp| {
            serde_json::json!({
                "global-client-fingerprint": fp.name,
            })
        })
    }
}

impl Default for TlsFingerprintService {
    fn default() -> Self {
        Self::new()
    }
}
