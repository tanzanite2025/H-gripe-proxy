import {
  getObfuscationStrategy,
  type ObfuscationLevel,
  type ObfuscationStrategy,
} from './obfuscation-strategies'
import { ProtocolObfuscationService } from './protocol-obfuscation'
import { TrafficObfuscationService } from './traffic-obfuscation'

/**
 * 混淆管理器
 * 统一管理所有混淆服务
 * 
 * 注意：配置来源已迁移到 Rust 后端 SecurityConfig.obfuscation，
 * 不再使用 localStorage。前端通过 coordinator.ts 的 SecurityConfig 读写。
 */

export interface ObfuscationConfig {
  enabled: boolean
  level: ObfuscationLevel
  autoAdjust: boolean
}

export class ObfuscationManager {
  private config: ObfuscationConfig
  private strategy: ObfuscationStrategy
  private trafficObfuscation: TrafficObfuscationService
  private protocolObfuscation: ProtocolObfuscationService

  constructor(config: ObfuscationConfig) {
    this.config = config
    this.strategy = getObfuscationStrategy(config.level)

    this.trafficObfuscation = new TrafficObfuscationService({
      enabled: config.enabled,
      strategy: this.strategy,
    })

    this.protocolObfuscation = new ProtocolObfuscationService({
      enabled: config.enabled,
      strategy: this.strategy,
    })
  }

  /**
   * 更新配置
   */
  updateConfig(config: Partial<ObfuscationConfig>) {
    this.config = { ...this.config, ...config }

    if (config.level) {
      this.strategy = getObfuscationStrategy(config.level)
    }

    this.trafficObfuscation.updateConfig({
      enabled: this.config.enabled,
      strategy: this.strategy,
    })

    this.protocolObfuscation.updateConfig({
      enabled: this.config.enabled,
      strategy: this.strategy,
    })
  }

  /**
   * 启用混淆
   */
  enable() {
    this.updateConfig({ enabled: true })
  }

  /**
   * 禁用混淆
   */
  disable() {
    this.updateConfig({ enabled: false })
  }

  /**
   * 设置混淆级别
   */
  setLevel(level: ObfuscationLevel) {
    this.updateConfig({ level })
  }

  /**
   * 获取流量混淆服务
   */
  getTrafficObfuscation() {
    return this.trafficObfuscation
  }

  /**
   * 获取协议混淆服务
   */
  getProtocolObfuscation() {
    return this.protocolObfuscation
  }

  /**
   * 获取当前配置
   */
  getConfig() {
    return {
      ...this.config,
      strategy: this.strategy,
    }
  }

  /**
   * 获取混淆统计信息
   */
  getStats() {
    return {
      config: this.config,
      strategy: {
        level: this.strategy.level,
        name: this.strategy.name,
        features: this.strategy.features,
      },
      traffic: this.trafficObfuscation.getStats(),
      protocol: this.protocolObfuscation.getStats(),
    }
  }
}

// 导出单例（配置由 SecurityConfig 驱动，不再持久化到 localStorage）
let obfuscationManager: ObfuscationManager | null = null

export function getObfuscationManager(): ObfuscationManager {
  if (!obfuscationManager) {
    obfuscationManager = new ObfuscationManager({
      enabled: false,
      level: 'medium',
      autoAdjust: false,
    })
  }
  return obfuscationManager
}

/**
 * 从 SecurityConfig 同步混淆配置
 */
export function syncObfuscationFromSecurityConfig(config: ObfuscationConfig) {
  const manager = getObfuscationManager()
  manager.updateConfig(config)
}

// 导出类型和策略
export * from './obfuscation-strategies'
export * from './traffic-obfuscation'
export * from './protocol-obfuscation'
