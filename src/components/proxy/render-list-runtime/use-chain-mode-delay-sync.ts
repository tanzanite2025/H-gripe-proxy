import { useEffect } from 'react'

import delayManager from '@/services/delay'
import { debugLog } from '@/utils/misc'

import type { ProxyItem } from '../render-list/types'

interface UseChainModeDelaySyncOptions {
  isChainMode?: boolean
  latencyTimeout: number
  onProxies: () => void
  runtimeConfig: unknown
}

export function useChainModeDelaySync({
  isChainMode,
  latencyTimeout,
  onProxies,
  runtimeConfig,
}: UseChainModeDelaySyncOptions) {
  useEffect(() => {
    if (!isChainMode || !runtimeConfig) return

    const allProxies: ProxyItem[] = Object.values(
      ((runtimeConfig as any).proxies || {}) as Record<string, ProxyItem>,
    )
    if (allProxies.length === 0) return

    const groupListener = () => {
      debugLog('[ChainMode] delay updated, refreshing proxy view')
      onProxies()
    }

    delayManager.setGroupListener('chain-mode', groupListener)

    const calculateDelays = async () => {
      try {
        const proxyNames = allProxies.map((proxy) => proxy.name)

        debugLog(
          `[ChainMode] calculating delay for ${proxyNames.length} proxies`,
        )
        delayManager.checkListDelay(proxyNames, 'chain-mode', latencyTimeout)
      } catch (error) {
        console.error('Failed to calculate delays for chain mode:', error)
      }
    }

    const handle = setTimeout(calculateDelays, 100)

    return () => {
      clearTimeout(handle)
      delayManager.removeGroupListener('chain-mode')
    }
  }, [isChainMode, latencyTimeout, onProxies, runtimeConfig])
}
