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
 */

export interface ObfuscationConfig {
  enabled: boolean
  level: ObfuscationLevel
  autoAdjust: boolean // 根据网络环境自动调整
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

    // 保存到 localStorage
    this.saveConfig()
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

  /**
   * 生成 Clash 配置
   */
  generateClashConfig() {
    if (!this.config.enabled) {
      return null
    }

    const httpMaskConfig = this.protocolObfuscation.generateHttpMaskConfig()
    const tlsFingerprint = this.protocolObfuscation.getRandomTlsFingerprint()

    return {
      'client-fingerprint': tlsFingerprint,
      ...(httpMaskConfig && {
        'http-opts': {
          headers: httpMaskConfig.headers,
          path: [httpMaskConfig.path],
        },
      }),
    }
  }

  /**
   * 保存配置到 localStorage
   */
  private saveConfig() {
    try {
      localStorage.setItem(
        'obfuscation-config',
        JSON.stringify(this.config),
      )
    } catch (error) {
      console.error('Failed to save obfuscation config:', error)
    }
  }

  /**
   * 从 localStorage 加载配置
   */
  static loadConfig(): ObfuscationConfig {
    try {
      const saved = localStorage.getItem('obfuscation-config')
      if (saved) {
        return JSON.parse(saved)
      }
    } catch (error) {
      console.error('Failed to load obfuscation config:', error)
    }

    // 默认配置
    return {
      enabled: false,
      level: 'medium',
      autoAdjust: false,
    }
  }
}

// 导出单例
let obfuscationManager: ObfuscationManager | null = null

export function getObfuscationManager(): ObfuscationManager {
  if (!obfuscationManager) {
    const config = ObfuscationManager.loadConfig()
    obfuscationManager = new ObfuscationManager(config)
  }
  return obfuscationManager
}

// 导出类型和策略
export * from './obfuscation-strategies'
export * from './traffic-obfuscation'
export * from './protocol-obfuscation'
