import type { ObfuscationStrategy } from './obfuscation-strategies'

/**
 * 流量混淆服务
 * 负责流量特征的混淆，包括包大小、时序等
 */

export interface TrafficObfuscationConfig {
  enabled: boolean
  strategy: ObfuscationStrategy
}

export class TrafficObfuscationService {
  private config: TrafficObfuscationConfig

  constructor(config: TrafficObfuscationConfig) {
    this.config = config
  }

  /**
   * 更新配置
   */
  updateConfig(config: Partial<TrafficObfuscationConfig>) {
    this.config = { ...this.config, ...config }
  }

  /**
   * 生成随机填充大小
   */
  generatePaddingSize(): number {
    if (!this.config.enabled) return 0

    const { minPaddingSize, maxPaddingSize } = this.config.strategy.config
    return (
      Math.floor(Math.random() * (maxPaddingSize - minPaddingSize + 1)) +
      minPaddingSize
    )
  }

  /**
   * 生成时序抖动
   */
  generateTimingJitter(): number {
    if (!this.config.enabled) return 0

    const { timingJitter } = this.config.strategy.config
    return Math.floor(Math.random() * timingJitter)
  }

  /**
   * 计算混淆后的包大小
   */
  obfuscatePacketSize(originalSize: number): number {
    if (!this.config.enabled) return originalSize

    const { packetSizeVariation } = this.config.strategy.config
    const variation = (originalSize * packetSizeVariation) / 100
    const randomVariation = Math.random() * variation * 2 - variation

    return Math.max(1, Math.floor(originalSize + randomVariation))
  }

  /**
   * 生成随机填充数据
   */
  generatePadding(size: number): Uint8Array {
    const padding = new Uint8Array(size)
    crypto.getRandomValues(padding)
    return padding
  }

  /**
   * 获取混淆统计信息
   */
  getStats() {
    return {
      enabled: this.config.enabled,
      level: this.config.strategy.level,
      avgPaddingSize:
        (this.config.strategy.config.minPaddingSize +
          this.config.strategy.config.maxPaddingSize) /
        2,
      avgTimingJitter: this.config.strategy.config.timingJitter / 2,
      packetSizeVariation: this.config.strategy.config.packetSizeVariation,
    }
  }
}
