/**
 * 混淆策略定义
 */

export type ObfuscationLevel = 'none' | 'low' | 'medium' | 'high' | 'paranoid'

export interface ObfuscationStrategy {
  level: ObfuscationLevel
  name: string
  description: string
  features: {
    trafficObfuscation: boolean
    protocolObfuscation: boolean
    timingObfuscation: boolean
    packetSizeObfuscation: boolean
    tlsFingerprintRandomization: boolean
    httpHeaderObfuscation: boolean
  }
  config: {
    minPaddingSize: number
    maxPaddingSize: number
    timingJitter: number // ms
    packetSizeVariation: number // %
  }
}

/**
 * 预定义的混淆策略
 */
export const OBFUSCATION_STRATEGIES: Record<
  ObfuscationLevel,
  ObfuscationStrategy
> = {
  none: {
    level: 'none',
    name: '无混淆',
    description: '不使用任何混淆技术',
    features: {
      trafficObfuscation: false,
      protocolObfuscation: false,
      timingObfuscation: false,
      packetSizeObfuscation: false,
      tlsFingerprintRandomization: false,
      httpHeaderObfuscation: false,
    },
    config: {
      minPaddingSize: 0,
      maxPaddingSize: 0,
      timingJitter: 0,
      packetSizeVariation: 0,
    },
  },

  low: {
    level: 'low',
    name: '低级混淆',
    description: '基础的流量混淆，对性能影响最小',
    features: {
      trafficObfuscation: true,
      protocolObfuscation: false,
      timingObfuscation: false,
      packetSizeObfuscation: true,
      tlsFingerprintRandomization: false,
      httpHeaderObfuscation: false,
    },
    config: {
      minPaddingSize: 0,
      maxPaddingSize: 64,
      timingJitter: 0,
      packetSizeVariation: 10,
    },
  },

  medium: {
    level: 'medium',
    name: '中级混淆',
    description: '平衡性能和隐私，推荐使用',
    features: {
      trafficObfuscation: true,
      protocolObfuscation: true,
      timingObfuscation: true,
      packetSizeObfuscation: true,
      tlsFingerprintRandomization: false,
      httpHeaderObfuscation: true,
    },
    config: {
      minPaddingSize: 0,
      maxPaddingSize: 256,
      timingJitter: 50,
      packetSizeVariation: 20,
    },
  },

  high: {
    level: 'high',
    name: '高级混淆',
    description: '强力混淆，适合高审查环境',
    features: {
      trafficObfuscation: true,
      protocolObfuscation: true,
      timingObfuscation: true,
      packetSizeObfuscation: true,
      tlsFingerprintRandomization: true,
      httpHeaderObfuscation: true,
    },
    config: {
      minPaddingSize: 64,
      maxPaddingSize: 512,
      timingJitter: 100,
      packetSizeVariation: 30,
    },
  },

  paranoid: {
    level: 'paranoid',
    name: '偏执级混淆',
    description: '最强混淆，性能影响较大',
    features: {
      trafficObfuscation: true,
      protocolObfuscation: true,
      timingObfuscation: true,
      packetSizeObfuscation: true,
      tlsFingerprintRandomization: true,
      httpHeaderObfuscation: true,
    },
    config: {
      minPaddingSize: 128,
      maxPaddingSize: 1024,
      timingJitter: 200,
      packetSizeVariation: 50,
    },
  },
}

/**
 * 获取混淆策略
 */
export function getObfuscationStrategy(
  level: ObfuscationLevel,
): ObfuscationStrategy {
  return OBFUSCATION_STRATEGIES[level]
}

/**
 * 获取所有混淆策略
 */
export function getAllObfuscationStrategies(): ObfuscationStrategy[] {
  return Object.values(OBFUSCATION_STRATEGIES)
}
