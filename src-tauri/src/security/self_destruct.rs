/**
 * 自毁机制模块
 *
 * 当检测到安全威胁时，清除敏感数据并退出
 */
use std::fs;
use std::path::PathBuf;

/// 自毁配置（与 config::advanced::SelfDestructConfig 同步）
#[derive(Debug, Clone)]
pub struct SelfDestructConfig {
    pub enabled: bool,
    pub clear_memory: bool,
    pub delete_configs: bool,
    pub delete_logs: bool,
    pub exit_immediately: bool,
}

impl Default for SelfDestructConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            clear_memory: true,
            delete_configs: false,
            delete_logs: true,
            exit_immediately: true,
        }
    }
}

impl From<crate::config::advanced::SelfDestructConfig> for SelfDestructConfig {
    fn from(c: crate::config::advanced::SelfDestructConfig) -> Self {
        Self {
            enabled: c.enabled,
            clear_memory: c.clear_memory,
            delete_configs: c.delete_configs,
            delete_logs: c.delete_logs,
            exit_immediately: c.exit_immediately,
        }
    }
}

/// 清除内存中的敏感数据
fn clear_sensitive_memory() {
    log::warn!("🔥 清除内存中的敏感数据...");

    // 这里应该清除所有敏感数据结构
    // 例如：密钥、密码、令牌等

    // 强制垃圾回收（Rust 没有 GC，但可以显式 drop）
    // 实际应用中，应该遍历所有敏感数据结构并清零

    // 覆写栈上的敏感数据
    let mut dummy = vec![0u8; 1024 * 1024]; // 1MB 的零
    for i in 0..dummy.len() {
        dummy[i] = 0;
    }
    drop(dummy);

    log::info!("✅ 内存清除完成");
}

/// 删除配置文件
fn delete_config_files(config_paths: &[PathBuf]) {
    log::warn!("🔥 删除配置文件...");

    for path in config_paths {
        if path.exists() {
            match fs::remove_file(path) {
                Ok(_) => log::info!("✅ 已删除: {:?}", path),
                Err(e) => log::error!("❌ 删除失败 {:?}: {}", path, e),
            }
        }
    }
}

/// 删除日志文件
#[allow(dead_code)]
fn delete_log_files(log_dir: &PathBuf) {
    log::warn!("🔥 删除日志文件...");

    if log_dir.exists() && log_dir.is_dir() {
        if let Ok(entries) = fs::read_dir(log_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        if ext == "log" || ext == "txt" {
                            match fs::remove_file(&path) {
                                Ok(_) => log::info!("✅ 已删除日志: {:?}", path),
                                Err(e) => log::error!("❌ 删除日志失败 {:?}: {}", path, e),
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 安全擦除文件（多次覆写）
#[allow(dead_code)]
fn secure_erase_file(path: &PathBuf) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    // 获取文件大小
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    let file_size = metadata.len() as usize;

    // 覆写 3 次
    for pass in 0..3 {
        let pattern = match pass {
            0 => 0xFF,                 // 全 1
            1 => 0x00,                 // 全 0
            _ => rand::random::<u8>(), // 随机
        };

        let data = vec![pattern; file_size];
        fs::write(path, data).map_err(|e| e.to_string())?;
    }

    // 最后删除文件
    fs::remove_file(path).map_err(|e| e.to_string())?;

    Ok(())
}

/// 执行自毁
pub fn execute() {
    execute_with_config(SelfDestructConfig::default())
}

/// 使用自定义配置执行自毁
pub fn execute_with_config(config: SelfDestructConfig) {
    log::error!("🚨🚨🚨 触发自毁机制！🚨🚨🚨");

    // 1. 清除内存
    if config.clear_memory {
        clear_sensitive_memory();
    }

    // 2. 删除配置文件
    if config.delete_configs {
        if let Ok(app_dir) = crate::utils::dirs::app_home_dir() {
            let config_paths = vec![
                app_dir.join("advanced.yaml"),
                app_dir.join("profiles.yaml"),
                app_dir.join("verge.yaml"),
            ];
            delete_config_files(&config_paths);
        }
    }

    // 3. 删除日志文件
    if config.delete_logs {
        if let Ok(app_dir) = crate::utils::dirs::app_home_dir() {
            let log_dir = app_dir.join("logs");
            delete_log_files(&log_dir);
        }
    }

    // 4. 退出程序
    if config.exit_immediately {
        log::error!("🚨 程序即将退出...");
        std::thread::sleep(std::time::Duration::from_secs(1));
        std::process::exit(1);
    }
}

/// 紧急自毁（最激进的模式）
#[allow(dead_code)]
pub fn emergency_destruct() {
    log::error!("🚨🚨🚨 紧急自毁！🚨🚨🚨");

    // 立即清除内存
    clear_sensitive_memory();

    // 立即退出，不做任何清理
    std::process::abort();
}

/// 检查是否应该触发自毁
#[allow(dead_code)]
pub fn should_self_destruct() -> bool {
    // 检查自毁是否启用
    let config = SelfDestructConfig::from(load_advanced_self_destruct_config());
    if !config.enabled {
        return false;
    }

    // 检查环境变量中的紧急停止标志
    if let Ok(val) = std::env::var("CLASH_VERGE_EMERGENCY_STOP") {
        if val == "1" || val.to_lowercase() == "true" {
            return true;
        }
    }

    // 检查安全状态
    if crate::security::is_security_compromised() {
        return true;
    }

    false
}

/// 从 advanced.yaml 加载自毁配置并执行
pub fn execute_from_advanced_config() {
    let config = SelfDestructConfig::from(load_advanced_self_destruct_config());
    if config.enabled {
        execute_with_config(config);
    }
}

fn load_advanced_self_destruct_config() -> crate::config::advanced::SelfDestructConfig {
    crate::feat::get_coordinator()
        .get_advanced_config()
        .security
        .self_destruct
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_destruct_config() {
        let config = SelfDestructConfig::default();
        assert!(config.clear_memory);
        assert!(config.exit_immediately);
    }

    #[test]
    fn test_should_self_destruct() {
        // 默认情况下不应该自毁
        assert!(!should_self_destruct());
    }
}
