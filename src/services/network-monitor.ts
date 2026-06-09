import { fetch } from '@tauri-apps/plugin-http'

import { DEFAULT_DELAY_TEST_URL } from './delay-config'

export type NetworkQuality = 'good' | 'poor' | 'offline'

export interface NetworkStatus {
  online: boolean
  quality: NetworkQuality
  lastCheck: number
}

type NetworkStatusListener = (status: NetworkStatus) => void

const isWebTestSandboxPath =
  typeof window !== 'undefined' &&
  /^\/web-test(?:\/|$)/.test(window.location.pathname)

class NetworkMonitor {
  private online = typeof navigator === 'undefined' ? true : navigator.onLine
  private quality: NetworkQuality = 'good'
  private lastCheck = Date.now()
  private listeners = new Set<NetworkStatusListener>()
  private checkInterval: ReturnType<typeof setInterval> | null = null
  private readonly checkIntervalMs = 30_000
  private readonly qualityCheckTimeoutMs = 3_000

  constructor() {
    if (isWebTestSandboxPath) {
      return
    }

    this.initialize()
  }

  private initialize() {
    window.addEventListener('online', this.handleOnline)
    window.addEventListener('offline', this.handleOffline)

    void this.checkNetworkQuality()
    this.startQualityCheck()
  }

  private handleOnline = () => {
    console.debug('[NetworkMonitor] 网络已连接')
    this.online = true
    void this.checkNetworkQuality()
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
      const controller = new AbortController()
      const timeoutId = setTimeout(() => {
        controller.abort()
      }, this.qualityCheckTimeoutMs)

      await fetch(DEFAULT_DELAY_TEST_URL, {
        method: 'HEAD',
        signal: controller.signal,
      })

      clearTimeout(timeoutId)

      const latency = Date.now() - start
      const previousQuality = this.quality

      this.quality = latency < 500 ? 'good' : 'poor'
      this.lastCheck = Date.now()

      console.debug(
        `[NetworkMonitor] 网络质量检测完成: ${this.quality} (${latency}ms)`,
      )

      if (previousQuality !== this.quality) {
        this.notifyListeners()
      }
    } catch {
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
      void this.checkNetworkQuality()
    }, this.checkIntervalMs)
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

  getQuality(): NetworkQuality {
    return this.quality
  }

  isOnline(): boolean {
    return this.online
  }

  getStatus(): NetworkStatus {
    return {
      online: this.online,
      quality: this.quality,
      lastCheck: this.lastCheck,
    }
  }

  subscribe(listener: NetworkStatusListener): () => void {
    this.listeners.add(listener)
    listener(this.getStatus())
    return () => this.listeners.delete(listener)
  }

  async checkNow(): Promise<NetworkStatus> {
    await this.checkNetworkQuality()
    return this.getStatus()
  }

  destroy() {
    window.removeEventListener('online', this.handleOnline)
    window.removeEventListener('offline', this.handleOffline)

    if (this.checkInterval) {
      clearInterval(this.checkInterval)
    }

    this.listeners.clear()
  }
}

export const networkMonitor = new NetworkMonitor()
