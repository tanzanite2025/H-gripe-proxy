use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsFingerprint {
    pub name: String,
    pub description: String,
    pub category: String,
}

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

    pub fn get_by_name(name: &str) -> Option<TlsFingerprint> {
        Self::get_all().into_iter().find(|fp| fp.name == name)
    }

    pub fn is_valid(name: &str) -> bool {
        Self::get_all().iter().any(|fp| fp.name == name)
    }
}

pub struct TlsFingerprintService {
    current_fingerprint: parking_lot::RwLock<Option<TlsFingerprint>>,
}

impl TlsFingerprintService {
    pub fn new() -> Self {
        Self {
            current_fingerprint: parking_lot::RwLock::new(None),
        }
    }

    pub fn set_fingerprint(&self, fingerprint: TlsFingerprint) {
        *self.current_fingerprint.write() = Some(fingerprint);
    }

    pub fn set_by_name(&self, name: &str) -> Result<(), String> {
        if let Some(fp) = TlsFingerprintLibrary::get_by_name(name) {
            self.set_fingerprint(fp);
            Ok(())
        } else {
            Err(format!("TLS fingerprint not found: {}", name))
        }
    }

    pub fn clear(&self) {
        *self.current_fingerprint.write() = None;
    }
}

impl Default for TlsFingerprintService {
    fn default() -> Self {
        Self::new()
    }
}
