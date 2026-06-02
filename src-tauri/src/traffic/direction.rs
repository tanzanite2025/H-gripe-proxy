/**
 * 方向混淆配置模块
 *
 * 注意：真正的方向混淆由 Go 内核（mihomo）的 ObfuscatedConn 在连接层执行。
 * 此模块仅定义配置结构，供前端面板和 profile 派生使用。
 */
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
