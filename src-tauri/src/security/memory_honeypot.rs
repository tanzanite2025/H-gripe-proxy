/**
 * 内存蜜罐模块
 * 
 * 在内存中放置诱饵数据，检测是否有进程在扫描内存
 */

use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 蜜罐令牌（Canary Token）
#[repr(C)]
pub struct HoneypotToken {
    /// 魔术数字（用于识别）
    magic: u64,
    /// 假密钥
    fake_key: [u8; 32],
    /// 假服务器地址
    fake_server: [u8; 256],
    /// 访问计数器
    access_count: AtomicU64,
    /// 最后访问时间
    last_access: AtomicU64,
}

impl HoneypotToken {
    const MAGIC: u64 = 0xDEADBEEFCAFEBABE;

    /// 创建新的蜜罐令牌
    pub fn new() -> Self {
        let mut token = Self {
            magic: Self::MAGIC,
            fake_key: [0u8; 32],
            fake_server: [0u8; 256],
            access_count: AtomicU64::new(0),
            last_access: AtomicU64::new(0),
        };

        // 生成假密钥
        for i in 0..32 {
            token.fake_key[i] = (i as u8).wrapping_mul(7).wrapping_add(13);
        }

        // 生成假服务器地址（看起来像真的）
        let fake_addr = b"https://fake-proxy-server-honeypot.example.com:8443/api/v1/connect?token=FAKE_TOKEN_DO_NOT_USE";
        let len = fake_addr.len().min(256);
        token.fake_server[..len].copy_from_slice(&fake_addr[..len]);

        token
    }

    /// 检查是否被访问
    pub fn check_access(&self) -> bool {
        let count = self.access_count.load(Ordering::Relaxed);
        count > 0
    }

    /// 获取访问次数
    pub fn get_access_count(&self) -> u64 {
        self.access_count.load(Ordering::Relaxed)
    }

    /// 记录访问（由内存保护机制调用）
    #[allow(dead_code)]
    pub fn record_access(&self) {
        self.access_count.fetch_add(1, Ordering::Relaxed);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.last_access.store(now, Ordering::Relaxed);
    }
}

impl Default for HoneypotToken {
    fn default() -> Self {
        Self::new()
    }
}

/// 内存蜜罐管理器
pub struct MemoryHoneypot {
    tokens: Vec<Box<HoneypotToken>>,
    enabled: bool,
}

impl MemoryHoneypot {
    /// 创建新的内存蜜罐
    pub fn new(token_count: usize) -> Self {
        let mut tokens = Vec::with_capacity(token_count);
        for _ in 0..token_count {
            tokens.push(Box::new(HoneypotToken::new()));
        }

        Self {
            tokens,
            enabled: true,
        }
    }

    /// 检查是否有令牌被访问
    pub fn check_compromise(&self) -> bool {
        if !self.enabled {
            return false;
        }

        for token in &self.tokens {
            if token.check_access() {
                log::warn!(
                    "🚨 内存蜜罐被触发！访问次数: {}",
                    token.get_access_count()
                );
                return true;
            }
        }

        false
    }

    /// 获取统计信息
    #[allow(dead_code)]
    pub fn get_stats(&self) -> HoneypotStats {
        let mut total_accesses = 0;
        let mut compromised_tokens = 0;

        for token in &self.tokens {
            let count = token.get_access_count();
            if count > 0 {
                compromised_tokens += 1;
                total_accesses += count;
            }
        }

        HoneypotStats {
            total_tokens: self.tokens.len(),
            compromised_tokens,
            total_accesses,
        }
    }
}

impl Default for MemoryHoneypot {
    fn default() -> Self {
        Self::new(5) // 默认 5 个蜜罐令牌
    }
}

/// 蜜罐统计信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HoneypotStats {
    pub total_tokens: usize,
    pub compromised_tokens: usize,
    pub total_accesses: u64,
}

/// 全局内存蜜罐实例（线程安全）
static GLOBAL_HONEYPOT: Lazy<RwLock<Option<MemoryHoneypot>>> =
    Lazy::new(|| RwLock::new(None));

/// 初始化全局蜜罐（使用配置中的令牌数量）
pub fn init_global_honeypot_with_count(token_count: usize) {
    let mut guard = GLOBAL_HONEYPOT.write().unwrap();
    *guard = Some(MemoryHoneypot::new(token_count));
}

/// 初始化全局蜜罐（默认 10 个令牌）
pub fn init_global_honeypot() {
    init_global_honeypot_with_count(10);
}

/// 检查全局蜜罐
pub fn check_global_honeypot() -> bool {
    let guard = GLOBAL_HONEYPOT.read().unwrap();
    guard.as_ref().map(|h| h.check_compromise()).unwrap_or(false)
}

/// 获取全局蜜罐统计
pub fn get_global_honeypot_stats() -> HoneypotStats {
    let guard = GLOBAL_HONEYPOT.read().unwrap();
    guard.as_ref().map(|h| h.get_stats()).unwrap_or(HoneypotStats {
        total_tokens: 0,
        compromised_tokens: 0,
        total_accesses: 0,
    })
}

/// 内存扫描检测
pub fn detect_memory_scanning() -> bool {
    #[cfg(target_os = "windows")]
    {
        detect_memory_scanning_windows()
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        detect_memory_scanning_unix()
    }

    #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
    {
        false
    }
}

#[cfg(target_os = "windows")]
fn detect_memory_scanning_windows() -> bool {
    use crate::utils::command::hidden_command;

    // 检查是否有可疑的内存扫描工具在运行
    let suspicious_tools = [
        "cheatengine",
        "processhacker",
        "procexp",
        "procmon",
        "wireshark",
        "fiddler",
    ];

    for tool in &suspicious_tools {
        if let Ok(output) = hidden_command("tasklist")
            .args(&["/FI", &format!("IMAGENAME eq {}.exe", tool)])
            .output()
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                if output_str.to_lowercase().contains(tool) {
                    log::warn!("🚨 检测到可疑工具: {}", tool);
                    return true;
                }
            }
        }
    }

    false
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn detect_memory_scanning_unix() -> bool {
    use std::process::Command;

    // 检查是否有进程在读取我们的内存
    let pid = std::process::id();
    
    #[cfg(target_os = "linux")]
    {
        // 检查 /proc/[pid]/maps 的访问
        if let Ok(output) = Command::new("lsof")
            .args(&["-p", &pid.to_string()])
            .output()
        {
            if let Ok(output_str) = String::from_utf8(output.stdout) {
                // 检查是否有其他进程在访问我们的内存映射
                let lines: Vec<&str> = output_str.lines().collect();
                if lines.len() > 50 {
                    // 异常多的文件描述符可能表示被监控
                    log::warn!("🚨 检测到异常多的文件描述符");
                    return true;
                }
            }
        }
    }

    // 检查可疑工具
    let suspicious_tools = ["gdb", "lldb", "valgrind", "strace"];
    for tool in &suspicious_tools {
        if let Ok(output) = Command::new("pgrep").arg(tool).output() {
            if !output.stdout.is_empty() {
                log::warn!("🚨 检测到可疑工具: {}", tool);
                return true;
            }
        }
    }

    false
}

/// 内存蜜罐监控循环
pub fn monitor_loop(enabled: Arc<AtomicBool>) {
    // 初始化全局蜜罐
    init_global_honeypot();

    while enabled.load(Ordering::Relaxed) {
        // 检查蜜罐是否被触发
        if check_global_honeypot() {
            log::error!("🚨 内存蜜罐被触发！可能有进程在扫描内存！");
            crate::security::mark_security_compromised();
            crate::security::self_destruct::execute();
            break;
        }

        // 检测内存扫描工具
        if detect_memory_scanning() {
            log::error!("🚨 检测到内存扫描工具！");
            crate::security::mark_security_compromised();
            crate::security::self_destruct::execute();
            break;
        }

        std::thread::sleep(Duration::from_secs(2));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_honeypot_token() {
        let token = HoneypotToken::new();
        assert_eq!(token.magic, HoneypotToken::MAGIC);
        assert_eq!(token.get_access_count(), 0);
    }

    #[test]
    fn test_memory_honeypot() {
        let honeypot = MemoryHoneypot::new(3);
        assert!(!honeypot.check_compromise());
        
        let stats = honeypot.get_stats();
        assert_eq!(stats.total_tokens, 3);
        assert_eq!(stats.compromised_tokens, 0);
    }
}
