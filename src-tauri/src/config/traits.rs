/**
 * 通用配置文件管理 Trait
 * 
 * 提供统一的配置加载、保存、备份功能
 */

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 配置文件管理 trait
/// 
/// 所有配置结构体都应该实现这个 trait，以获得统一的文件操作能力
pub trait ConfigFile: Serialize + for<'de> Deserialize<'de> + Default {
    /// 从文件加载配置
    /// 
    /// 如果文件不存在，返回默认配置
    fn load_from_file(path: &PathBuf) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        
        let content = std::fs::read_to_string(path)?;
        let config: Self = serde_yaml_ng::from_str(&content)?;
        Ok(config)
    }

    /// 保存配置到文件
    /// 
    /// 自动创建父目录
    fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        // 创建父目录
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // 序列化并保存
        let content = serde_yaml_ng::to_string(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// 保存配置到文件（带备份）
    /// 
    /// 如果文件已存在，先创建 .bak 备份
    fn save_to_file_with_backup(&self, path: &PathBuf) -> Result<()> {
        // 如果文件存在，先备份
        if path.exists() {
            let backup_path = path.with_extension("yaml.bak");
            std::fs::copy(path, backup_path)?;
        }
        
        self.save_to_file(path)
    }

    /// 从备份恢复配置
    /// 
    /// 如果备份文件存在，从备份恢复
    fn restore_from_backup(path: &PathBuf) -> Result<Self> {
        let backup_path = path.with_extension("yaml.bak");
        
        if !backup_path.exists() {
            anyhow::bail!("备份文件不存在");
        }
        
        let content = std::fs::read_to_string(&backup_path)?;
        let config: Self = serde_yaml_ng::from_str(&content)?;
        
        // 恢复到原文件
        std::fs::copy(&backup_path, path)?;
        
        Ok(config)
    }

    /// 验证配置文件
    /// 
    /// 尝试加载配置，检查是否有效
    fn validate_file(path: &PathBuf) -> Result<()> {
        if !path.exists() {
            anyhow::bail!("配置文件不存在");
        }
        
        let content = std::fs::read_to_string(path)?;
        let _config: Self = serde_yaml_ng::from_str(&content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;

    #[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
    struct TestConfig {
        name: String,
        value: i32,
    }

    impl ConfigFile for TestConfig {}

    #[test]
    fn test_load_nonexistent_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        
        let config = TestConfig::load_from_file(&path).unwrap();
        assert_eq!(config, TestConfig::default());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        
        let config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };
        
        config.save_to_file(&path).unwrap();
        let loaded = TestConfig::load_from_file(&path).unwrap();
        
        assert_eq!(config, loaded);
    }

    #[test]
    fn test_save_with_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        
        // 第一次保存
        let config1 = TestConfig {
            name: "test1".to_string(),
            value: 1,
        };
        config1.save_to_file(&path).unwrap();
        
        // 第二次保存（带备份）
        let config2 = TestConfig {
            name: "test2".to_string(),
            value: 2,
        };
        config2.save_to_file_with_backup(&path).unwrap();
        
        // 验证备份文件存在
        let backup_path = path.with_extension("yaml.bak");
        assert!(backup_path.exists());
        
        // 验证备份内容是第一次的配置
        let backup_config = TestConfig::load_from_file(&backup_path).unwrap();
        assert_eq!(backup_config, config1);
        
        // 验证当前文件是第二次的配置
        let current_config = TestConfig::load_from_file(&path).unwrap();
        assert_eq!(current_config, config2);
    }

    #[test]
    fn test_restore_from_backup() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        
        // 保存原始配置
        let config1 = TestConfig {
            name: "original".to_string(),
            value: 1,
        };
        config1.save_to_file(&path).unwrap();
        
        // 保存新配置（带备份）
        let config2 = TestConfig {
            name: "modified".to_string(),
            value: 2,
        };
        config2.save_to_file_with_backup(&path).unwrap();
        
        // 从备份恢复
        let restored = TestConfig::restore_from_backup(&path).unwrap();
        assert_eq!(restored, config1);
        
        // 验证文件已恢复
        let current = TestConfig::load_from_file(&path).unwrap();
        assert_eq!(current, config1);
    }

    #[test]
    fn test_validate_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.yaml");
        
        // 不存在的文件
        assert!(TestConfig::validate_file(&path).is_err());
        
        // 有效的文件
        let config = TestConfig {
            name: "test".to_string(),
            value: 42,
        };
        config.save_to_file(&path).unwrap();
        assert!(TestConfig::validate_file(&path).is_ok());
        
        // 无效的文件
        std::fs::write(&path, "invalid yaml content: [[[").unwrap();
        assert!(TestConfig::validate_file(&path).is_err());
    }
}
