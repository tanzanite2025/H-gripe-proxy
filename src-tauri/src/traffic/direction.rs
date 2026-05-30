/**
 * 方向混淆模块
 *
 * 功能：
 * 1. 镜像流量 - 发送方向相反的假流量，打破方向特征
 * 2. 统一填充 - 双向包填充到相同大小，使方向无法通过包大小推断
 * 3. 随机组合 - 每个包随机选择镜像或填充
 */

use rand::Rng;
use serde::{Deserialize, Serialize};

/// 方向混淆配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectionObfuscationConfig {
    /// 启用方向混淆
    pub enabled: bool,
    /// 混淆模式
    pub mode: DirectionMode,
    /// 镜像触发概率 (0.0 - 1.0)
    pub mirror_ratio: f32,
    /// 将双向包填充到统一大小（0 = 不填充）
    pub pad_to_size: usize,
}

impl Default for DirectionObfuscationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: DirectionMode::Mirror,
            mirror_ratio: 0.3,
            pad_to_size: 0,
        }
    }
}

impl DirectionObfuscationConfig {
    pub fn conservative() -> Self {
        Self {
            enabled: false,
            mode: DirectionMode::Pad,
            mirror_ratio: 0.0,
            pad_to_size: 0,
        }
    }

    pub fn aggressive() -> Self {
        Self {
            enabled: true,
            mode: DirectionMode::Mirror,
            mirror_ratio: 0.3,
            pad_to_size: 0,
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.enabled {
            return Ok(());
        }
        if self.mirror_ratio < 0.0 || self.mirror_ratio > 1.0 {
            return Err("镜像触发概率必须在 0.0 到 1.0 之间".to_string());
        }
        if matches!(self.mode, DirectionMode::Pad | DirectionMode::Random) && self.pad_to_size == 0 {
            return Err("Pad/Random 模式下 pad_to_size 必须大于 0".to_string());
        }
        Ok(())
    }
}

/// 方向混淆模式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DirectionMode {
    /// 发送方向相反的假流量
    Mirror,
    /// 双向包填充到相同大小
    Pad,
    /// 随机组合 Mirror + Pad
    Random,
}

/// 方向混淆统计
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectionStats {
    /// 镜像次数
    pub mirror_count: u64,
    /// 填充次数
    pub pad_count: u64,
    /// 镜像总字节数
    pub total_mirror_bytes: u64,
    /// 填充总字节数
    pub total_pad_bytes: u64,
}

impl Default for DirectionStats {
    fn default() -> Self {
        Self {
            mirror_count: 0,
            pad_count: 0,
            total_mirror_bytes: 0,
            total_pad_bytes: 0,
        }
    }
}

/// 流量方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrafficDirection {
    Upstream,
    Downstream,
}

/// 方向混淆引擎
pub struct DirectionObfuscator {
    config: DirectionObfuscationConfig,
    stats: DirectionStats,
}

impl DirectionObfuscator {
    pub fn new(config: DirectionObfuscationConfig) -> Self {
        Self {
            config,
            stats: DirectionStats::default(),
        }
    }

    /// 更新配置
    pub fn update_config(&mut self, config: DirectionObfuscationConfig) {
        self.config = config;
    }

    /// 获取当前配置
    pub fn config(&self) -> &DirectionObfuscationConfig {
        &self.config
    }

    /// 获取统计
    pub fn stats(&self) -> &DirectionStats {
        &self.stats
    }

    /// 重置统计
    pub fn reset_stats(&mut self) {
        self.stats = DirectionStats::default();
        log::info!("📊 Direction obfuscation stats reset");
    }

    /// 处理一个真实流量包，返回需要发送的假数据
    /// `direction`: 真实流量的方向
    /// `size`: 真实流量包的大小
    /// 返回：需要发送的假数据（方向与真实流量相反）
    pub fn process_packet(&mut self, direction: TrafficDirection, size: usize) -> Option<DirectionAction> {
        if !self.config.enabled {
            return None;
        }

        match self.config.mode {
            DirectionMode::Mirror => self.handle_mirror(direction, size),
            DirectionMode::Pad => self.handle_pad(direction, size),
            DirectionMode::Random => {
                let mut rng = rand::thread_rng();
                if rng.r#gen::<f32>() < 0.5 {
                    self.handle_mirror(direction, size)
                } else {
                    self.handle_pad(direction, size)
                }
            }
        }
    }

    /// 镜像处理：以 mirror_ratio 概率生成方向相反的假流量
    fn handle_mirror(&mut self, direction: TrafficDirection, size: usize) -> Option<DirectionAction> {
        let mut rng = rand::thread_rng();
        if rng.r#gen::<f32>() < self.config.mirror_ratio {
            self.stats.mirror_count += 1;
            self.stats.total_mirror_bytes += size as u64;

            let fake_direction = match direction {
                TrafficDirection::Upstream => TrafficDirection::Downstream,
                TrafficDirection::Downstream => TrafficDirection::Upstream,
            };

            Some(DirectionAction {
                direction: fake_direction,
                size,
                action_type: DirectionActionType::Mirror,
            })
        } else {
            None
        }
    }

    /// 填充处理：将包填充到 pad_to_size
    fn handle_pad(&mut self, direction: TrafficDirection, size: usize) -> Option<DirectionAction> {
        let target = self.config.pad_to_size;
        if target > size {
            let pad_size = target - size;
            self.stats.pad_count += 1;
            self.stats.total_pad_bytes += pad_size as u64;

            Some(DirectionAction {
                direction,
                size: pad_size,
                action_type: DirectionActionType::Pad,
            })
        } else {
            None
        }
    }
}

/// 方向混淆动作
#[derive(Debug, Clone)]
pub struct DirectionAction {
    /// 假数据的发送方向
    pub direction: TrafficDirection,
    /// 假数据大小
    pub size: usize,
    /// 动作类型
    pub action_type: DirectionActionType,
}

/// 动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirectionActionType {
    Mirror,
    Pad,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DirectionObfuscationConfig::default();
        assert!(!config.enabled);
        assert!(matches!(config.mode, DirectionMode::Mirror));
    }

    #[test]
    fn test_conservative_config() {
        let config = DirectionObfuscationConfig::conservative();
        assert!(!config.enabled);
    }

    #[test]
    fn test_aggressive_config() {
        let config = DirectionObfuscationConfig::aggressive();
        assert!(config.enabled);
        assert!(matches!(config.mode, DirectionMode::Mirror));
        assert!((config.mirror_ratio - 0.3).abs() < f32::EPSILON);
    }

    #[test]
    fn test_validate_ok() {
        assert!(DirectionObfuscationConfig::default().validate().is_ok());
        let config = DirectionObfuscationConfig {
            enabled: true,
            mode: DirectionMode::Pad,
            mirror_ratio: 0.0,
            pad_to_size: 1024,
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_mirror_ratio_out_of_range() {
        let config = DirectionObfuscationConfig {
            enabled: true,
            mode: DirectionMode::Mirror,
            mirror_ratio: 1.5,
            pad_to_size: 0,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validate_pad_mode_zero_size() {
        let config = DirectionObfuscationConfig {
            enabled: true,
            mode: DirectionMode::Pad,
            mirror_ratio: 0.0,
            pad_to_size: 0,
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_process_packet_disabled() {
        let config = DirectionObfuscationConfig::default();
        let mut obfuscator = DirectionObfuscator::new(config);
        let result = obfuscator.process_packet(TrafficDirection::Upstream, 100);
        assert!(result.is_none());
    }

    #[test]
    fn test_process_packet_mirror() {
        let config = DirectionObfuscationConfig {
            enabled: true,
            mode: DirectionMode::Mirror,
            mirror_ratio: 1.0, // 100% 触发
            pad_to_size: 0,
        };
        let mut obfuscator = DirectionObfuscator::new(config);
        let result = obfuscator.process_packet(TrafficDirection::Upstream, 200);
        assert!(result.is_some());
        let action = result.unwrap();
        assert_eq!(action.direction, TrafficDirection::Downstream);
        assert_eq!(action.size, 200);
        assert_eq!(action.action_type, DirectionActionType::Mirror);
    }

    #[test]
    fn test_process_packet_pad() {
        let config = DirectionObfuscationConfig {
            enabled: true,
            mode: DirectionMode::Pad,
            mirror_ratio: 0.0,
            pad_to_size: 1024,
        };
        let mut obfuscator = DirectionObfuscator::new(config);
        let result = obfuscator.process_packet(TrafficDirection::Upstream, 200);
        assert!(result.is_some());
        let action = result.unwrap();
        assert_eq!(action.direction, TrafficDirection::Upstream);
        assert_eq!(action.size, 824); // 1024 - 200
        assert_eq!(action.action_type, DirectionActionType::Pad);
    }

    #[test]
    fn test_process_packet_pad_no_overflow() {
        let config = DirectionObfuscationConfig {
            enabled: true,
            mode: DirectionMode::Pad,
            mirror_ratio: 0.0,
            pad_to_size: 100,
        };
        let mut obfuscator = DirectionObfuscator::new(config);
        // 包大小 >= pad_to_size，不需要填充
        let result = obfuscator.process_packet(TrafficDirection::Upstream, 200);
        assert!(result.is_none());
    }

    #[test]
    fn test_stats() {
        let config = DirectionObfuscationConfig {
            enabled: true,
            mode: DirectionMode::Mirror,
            mirror_ratio: 1.0,
            pad_to_size: 0,
        };
        let mut obfuscator = DirectionObfuscator::new(config);
        obfuscator.process_packet(TrafficDirection::Upstream, 100);
        obfuscator.process_packet(TrafficDirection::Downstream, 200);

        let stats = obfuscator.stats();
        assert_eq!(stats.mirror_count, 2);
        assert_eq!(stats.total_mirror_bytes, 300);
    }

    #[test]
    fn test_reset_stats() {
        let config = DirectionObfuscationConfig {
            enabled: true,
            mode: DirectionMode::Mirror,
            mirror_ratio: 1.0,
            pad_to_size: 0,
        };
        let mut obfuscator = DirectionObfuscator::new(config);
        obfuscator.process_packet(TrafficDirection::Upstream, 100);
        obfuscator.reset_stats();

        let stats = obfuscator.stats();
        assert_eq!(stats.mirror_count, 0);
        assert_eq!(stats.total_mirror_bytes, 0);
    }

    #[test]
    fn test_update_config() {
        let config = DirectionObfuscationConfig::conservative();
        let mut obfuscator = DirectionObfuscator::new(config);
        obfuscator.update_config(DirectionObfuscationConfig::aggressive());
        assert!(obfuscator.config().enabled);
    }
}
