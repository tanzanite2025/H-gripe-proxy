/**
 * 网络状态监控服务
 * 监听网络在线/离线状态，并定期检测网络质量
 */

import { fetch } from '@tauri-apps/plugin-http'

export type NetworkQuality = 'good' | 'poor' | 'offline'

export interface NetworkStatus {
  online: boolean
  quality: NetworkQuality
  lastCheck: number
}

type NetworkStatusListener = (status: NetworkStatus) => void

class NetworkMonitor {
  private online = navigator.onLine
  private quality: NetworkQuality = 'good'
  private lastCheck = Date.now()
  private listeners = new Set<NetworkStatusListener>()
  private checkInterval: ReturnType<typeof setInterval> | null = null
  private readonly CHECK_INTERVAL = 30000 // 30秒检测一次
  private readonly QUALITY_CHECK_TIMEOUT = 3000 // 3秒超时

  constructor() {
    this.initialize()
  }

  private initialize() {
    // 监听在线/离线事件
    window.addEventListener('online', this.handleOnline)
    window.addEventListener('offline', this.handleOffline)

    // 初始化时检测一次
    this.checkNetworkQuality()

    // 定期检测网络质量
    this.startQualityCheck()
  }

  private handleOnline = () => {
    console.debug('[NetworkMonitor] 网络已连接')
    this.online = true
    this.checkNetworkQuality()
  }

  private handleOffline = () => {
    console.debug('[NetworkMonitor] 网络已断开')
    this.online = false
    this.quality = 'offline'
    this.notifyListeners()
  }

  private async checkNetworkQuality() {
    if (!this.online) {
      this.quality = 'offline'
      this.notifyListeners()
      return
    }

    const start = Date.now()
    try {
      // 使用小文件测试网络质量
      const controller = new AbortController()
      const timeoutId = setTimeout(() => {
        controller.abort()
      }, this.QUALITY_CHECK_TIMEOUT)

      await fetch('https://cp.cloudflare.com/generate_204', {
        method: 'HEAD',
        signal: controller.signal,
      })

      clearTimeout(timeoutId)

      const latency = Date.now() - start
      const previousQuality = this.quality

      // 根据延迟判断网络质量
      this.quality = latency < 500 ? 'good' : 'poor'
      this.lastCheck = Date.now()

      console.debug(
        `[NetworkMonitor] 网络质量检测完成: ${this.quality} (${latency}ms)`,
      )

      // 只有质量变化时才通知
      if (previousQuality !== this.quality) {
        this.notifyListeners()
      }
    } catch (ignore) {
      console.debug('[NetworkMonitor] 网络质量检测失败，判定为弱网')
      this.quality = 'poor'
      this.lastCheck = Date.now()
      this.notifyListeners()
    }
  }

  private startQualityCheck() {
    if (this.checkInterval) {
      clearInterval(this.checkInterval)
    }

    this.checkInterval = setInterval(() => {
      this.checkNetworkQuality()
    }, this.CHECK_INTERVAL)
  }

  private notifyListeners() {
    const status: NetworkStatus = {
      online: this.online,
      quality: this.quality,
      lastCheck: this.lastCheck,
    }

    this.listeners.forEach((listener) => {
      try {
        listener(status)
      } catch (error) {
        console.error('[NetworkMonitor] 通知监听器失败', error)
      }
    })
  }

  /**
   * 获取当前网络质量
   */
  getQuality(): NetworkQuality {
    return this.quality
  }

  /**
   * 获取当前在线状态
   */
  isOnline(): boolean {
    return this.online
  }

  /**
   * 获取完整的网络状态
   */
  getStatus(): NetworkStatus {
    return {
      online: this.online,
      quality: this.quality,
      lastCheck: this.lastCheck,
    }
  }

  /**
   * 订阅网络状态变化
   * @returns 取消订阅的函数
   */
  subscribe(listener: NetworkStatusListener): () => void {
    this.listeners.add(listener)
    // 立即通知当前状态
    listener(this.getStatus())
    return () => this.listeners.delete(listener)
  }

  /**
   * 手动触发网络质量检测
   */
  async checkNow(): Promise<NetworkStatus> {
    await this.checkNetworkQuality()
    return this.getStatus()
  }

  /**
   * 清理资源
   */
  destroy() {
    window.removeEventListener('online', this.handleOnline)
    window.removeEventListener('offline', this.handleOffline)
    if (this.checkInterval) {
      clearInterval(this.checkInterval)
    }
    this.listeners.clear()
  }
}

// 导出单例
export const networkMonitor = new NetworkMonitor()
