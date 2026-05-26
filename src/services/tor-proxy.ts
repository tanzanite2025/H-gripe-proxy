/**
 * Tor 代理服务
 * 通过 SOCKS5 代理使用 Tor 网络
 */

/**
 * Tor 配置
 */
export interface TorConfig {
  enabled: boolean
  socksHost: string
  socksPort: number
  controlPort?: number
  useBridges?: boolean
  bridges?: string[]
}

/**
 * Tor 状态
 */
export interface TorStatus {
  enabled: boolean
  connected: boolean
  circuitEstablished: boolean
  currentIp?: string
  exitNode?: string
}

/**
 * Tor 统计信息
 */
export interface TorStats {
  bytesRead: number
  bytesWritten: number
  circuitsBuilt: number
  uptime: number
}

class TorProxyService {
  private config: TorConfig = {
    enabled: false,
    socksHost: '127.0.0.1',
    socksPort: 9050,
    controlPort: 9051,
    useBridges: false,
    bridges: [],
  }

  private status: TorStatus = {
    enabled: false,
    connected: false,
    circuitEstablished: false,
  }

  /**
   * 启用 Tor
   */
  enable(config?: Partial<TorConfig>): void {
    if (config) {
      this.config = { ...this.config, ...config }
    }
    this.config.enabled = true
    this.status.enabled = true
    console.log('Tor proxy enabled:', this.config)
  }

  /**
   * 禁用 Tor
   */
  disable(): void {
    this.config.enabled = false
    this.status.enabled = false
    this.status.connected = false
    this.status.circuitEstablished = false
    console.log('Tor proxy disabled')
  }

  /**
   * 检查 Tor 是否启用
   */
  isEnabled(): boolean {
    return this.config.enabled
  }

  /**
   * 获取 Tor 配置
   */
  getConfig(): TorConfig {
    return { ...this.config }
  }

  /**
   * 设置 Tor 配置
   */
  setConfig(config: Partial<TorConfig>): void {
    this.config = { ...this.config, ...config }
    console.log('Tor config updated:', this.config)
  }

  /**
   * 获取 SOCKS5 代理地址
   */
  getSocksProxyUrl(): string {
    return `socks5://${this.config.socksHost}:${this.config.socksPort}`
  }

  /**
   * 获取 SOCKS5 代理配置（用于 Clash）
   */
  getSocksProxyConfig(): {
    type: 'socks5'
    server: string
    port: number
  } {
    return {
      type: 'socks5',
      server: this.config.socksHost,
      port: this.config.socksPort,
    }
  }

  /**
   * 检查 Tor 连接状态
   */
  async checkConnection(): Promise<boolean> {
    try {
      // 尝试通过 Tor 获取当前 IP
      // 注意：这需要通过 Clash 的 SOCKS5 代理进行
      // 实际实现需要调用后端 API
      
      // 模拟检查（实际应该调用后端）
      const isConnected = this.config.enabled
      
      this.status.connected = isConnected
      this.status.circuitEstablished = isConnected
      
      return isConnected
    } catch (err) {
      console.error('Failed to check Tor connection:', err)
      this.status.connected = false
      this.status.circuitEstablished = false
      return false
    }
  }

  /**
   * 获取当前 IP（通过 Tor）
   */
  async getCurrentIp(): Promise<string | null> {
    if (!this.config.enabled) {
      return null
    }

    try {
      // 实际实现需要通过 Tor SOCKS5 代理访问 check.torproject.org
      // 或者 https://api.ipify.org
      
      // 模拟返回（实际应该调用后端）
      return null
    } catch (err) {
      console.error('Failed to get current IP through Tor:', err)
      return null
    }
  }

  /**
   * 获取 Tor 状态
   */
  getStatus(): TorStatus {
    return { ...this.status }
  }

  /**
   * 更新状态
   */
  updateStatus(status: Partial<TorStatus>): void {
    this.status = { ...this.status, ...status }
  }

  /**
   * 添加网桥
   */
  addBridge(bridge: string): void {
    if (!this.config.bridges) {
      this.config.bridges = []
    }
    
    if (!this.config.bridges.includes(bridge)) {
      this.config.bridges.push(bridge)
      console.log(`Tor bridge added: ${bridge}`)
    }
  }

  /**
   * 移除网桥
   */
  removeBridge(bridge: string): void {
    if (!this.config.bridges) return
    
    this.config.bridges = this.config.bridges.filter((b) => b !== bridge)
    console.log(`Tor bridge removed: ${bridge}`)
  }

  /**
   * 清空网桥
   */
  clearBridges(): void {
    this.config.bridges = []
    console.log('Tor bridges cleared')
  }

  /**
   * 获取所有网桥
   */
  getBridges(): string[] {
    return this.config.bridges || []
  }

  /**
   * 启用网桥模式
   */
  enableBridges(): void {
    this.config.useBridges = true
    console.log('Tor bridges enabled')
  }

  /**
   * 禁用网桥模式
   */
  disableBridges(): void {
    this.config.useBridges = false
    console.log('Tor bridges disabled')
  }

  /**
   * 生成 Tor 配置文件内容
   */
  generateTorConfig(): string {
    const lines: string[] = []
    
    // SOCKS 端口
    lines.push(`SocksPort ${this.config.socksHost}:${this.config.socksPort}`)
    
    // 控制端口
    if (this.config.controlPort) {
      lines.push(`ControlPort ${this.config.controlPort}`)
    }
    
    // 网桥配置
    if (this.config.useBridges && this.config.bridges && this.config.bridges.length > 0) {
      lines.push('UseBridges 1')
      this.config.bridges.forEach((bridge) => {
        lines.push(`Bridge ${bridge}`)
      })
    }
    
    return lines.join('\n')
  }

  /**
   * 获取使用说明
   */
  getUsageInstructions(): {
    title: string
    steps: string[]
    notes: string[]
  } {
    return {
      title: 'Tor 使用说明',
      steps: [
        '1. 下载并安装 Tor Browser 或 Tor Expert Bundle',
        '2. 启动 Tor，确保 SOCKS5 代理运行在 127.0.0.1:9050',
        '3. 在 Clash Verge 中启用 Tor 代理',
        '4. 配置代理规则使用 Tor',
        '5. （可选）配置 DoH 防止 DNS 泄露',
      ],
      notes: [
        '• Tor 会显著降低网络速度（通常 < 1 Mbps）',
        '• 建议配合 DoH 使用，防止 DNS 泄露',
        '• 在某些地区可能需要使用网桥（Bridges）',
        '• 不要在 Tor 上进行大流量下载',
        '• 定期更换 Tor 电路以提高匿名性',
      ],
    }
  }
}

// 导出单例
export const torProxyService = new TorProxyService()

