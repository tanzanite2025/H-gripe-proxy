/**
 * DNS 管理器 Hook
 * 用于初始化和管理 DNS 服务
 */

import { useEffect, useState } from 'react'

import { dnsManager } from '@/services/dns-manager'

interface UseDnsManagerOptions {
  enableCache?: boolean
  enablePrefetch?: boolean
  enableHealthCheck?: boolean
  autoInitialize?: boolean
}

export const useDnsManager = (options: UseDnsManagerOptions = {}) => {
  const {
    enableCache = true,
    enablePrefetch = true,
    enableHealthCheck = true,
    autoInitialize = true,
  } = options

  const [initialized, setInitialized] = useState(false)
  const [error, setError] = useState<Error | null>(null)

  useEffect(() => {
    if (!autoInitialize) return

    const initialize = async () => {
      try {
        await dnsManager.initialize({
          enableCache,
          enablePrefetch,
          enableHealthCheck,
          prefetchInterval: 300000, // 5 分钟
          healthCheckInterval: 60000, // 1 分钟
        })
        setInitialized(true)
        console.log('DNS Manager initialized')
      } catch (err) {
        setError(err as Error)
        console.error('Failed to initialize DNS Manager', err)
      }
    }

    void initialize()

    // 清理
    return () => {
      dnsManager.shutdown()
      setInitialized(false)
    }
  }, [autoInitialize, enableCache, enablePrefetch, enableHealthCheck])

  return {
    initialized,
    error,
  }
}
