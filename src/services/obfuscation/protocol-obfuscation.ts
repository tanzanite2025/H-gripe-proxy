import type { ObfuscationStrategy } from './obfuscation-strategies'

/**
 * 协议混淆服务
 * 负责协议层面的混淆，包括 HTTP 头、TLS 指纹等
 */

export interface ProtocolObfuscationConfig {
  enabled: boolean
  strategy: ObfuscationStrategy
}

// 常见的 User-Agent 列表
const USER_AGENTS = [
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
  'Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0',
  'Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15',
  'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36',
]

// 常见的 Accept-Language 列表
const ACCEPT_LANGUAGES = [
  'en-US,en;q=0.9',
  'zh-CN,zh;q=0.9,en;q=0.8',
  'ja-JP,ja;q=0.9,en;q=0.8',
  'ko-KR,ko;q=0.9,en;q=0.8',
  'de-DE,de;q=0.9,en;q=0.8',
]

// TLS 指纹配置
const TLS_FINGERPRINTS = [
  'chrome',
  'firefox',
  'safari',
  'edge',
  'ios',
  'android',
  'random',
]

export class ProtocolObfuscationService {
  private config: ProtocolObfuscationConfig

  constructor(config: ProtocolObfuscationConfig) {
    this.config = config
  }

  /**
   * 更新配置
   */
  updateConfig(config: Partial<ProtocolObfuscationConfig>) {
    this.config = { ...this.config, ...config }
  }

  /**
   * 生成随机 HTTP 头
   */
  generateHttpHeaders(): Record<string, string> {
    if (!this.config.enabled || !this.config.strategy.features.httpHeaderObfuscation) {
      return {}
    }

    const headers: Record<string, string> = {}

    // User-Agent
    headers['User-Agent'] =
      USER_AGENTS[Math.floor(Math.random() * USER_AGENTS.length)]

    // Accept-Language
    headers['Accept-Language'] =
      ACCEPT_LANGUAGES[Math.floor(Math.random() * ACCEPT_LANGUAGES.length)]

    // Accept
    headers['Accept'] =
      'text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8'

    // Accept-Encoding
    headers['Accept-Encoding'] = 'gzip, deflate, br'

    // Cache-Control (随机)
    if (Math.random() > 0.5) {
      headers['Cache-Control'] = 'no-cache'
    }

    // DNT (随机)
    if (Math.random() > 0.5) {
      headers['DNT'] = '1'
    }

    return headers
  }

  /**
   * 获取随机 TLS 指纹
   */
  getRandomTlsFingerprint(): string {
    if (
      !this.config.enabled ||
      !this.config.strategy.features.tlsFingerprintRandomization
    ) {
      return 'chrome' // 默认
    }

    return TLS_FINGERPRINTS[
      Math.floor(Math.random() * TLS_FINGERPRINTS.length)
    ]
  }

  /**
   * 生成 HTTP/HTTPS 伪装配置
   */
  generateHttpMaskConfig() {
    if (!this.config.enabled || !this.config.strategy.features.protocolObfuscation) {
      return null
    }

    return {
      enabled: true,
      headers: this.generateHttpHeaders(),
      path: this.generateRandomPath(),
      host: this.generateRandomHost(),
    }
  }

  /**
   * 生成随机路径
   */
  private generateRandomPath(): string {
    const paths = [
      '/api/v1/data',
      '/static/js/main.js',
      '/assets/images/logo.png',
      '/cdn/lib/jquery.min.js',
      '/resources/style.css',
    ]
    return paths[Math.floor(Math.random() * paths.length)]
  }

  /**
   * 生成随机主机名
   */
  private generateRandomHost(): string {
    const hosts = [
      'www.google.com',
      'www.cloudflare.com',
      'www.microsoft.com',
      'www.apple.com',
      'www.amazon.com',
    ]
    return hosts[Math.floor(Math.random() * hosts.length)]
  }

  /**
   * 获取混淆统计信息
   */
  getStats() {
    return {
      enabled: this.config.enabled,
      level: this.config.strategy.level,
      httpHeaderObfuscation:
        this.config.strategy.features.httpHeaderObfuscation,
      tlsFingerprintRandomization:
        this.config.strategy.features.tlsFingerprintRandomization,
      protocolObfuscation: this.config.strategy.features.protocolObfuscation,
    }
  }
}
