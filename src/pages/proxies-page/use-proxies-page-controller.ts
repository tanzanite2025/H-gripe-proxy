import { useLockFn } from 'ahooks'
import { useEffect, useState } from 'react'

import { clearProxyChainRuntimeConfig } from '@/components/proxy/proxy-chain-runtime'
import { loadProxyChainRuntimeExitNode } from '@/components/proxy/proxy-chain-types'
import { getRuntimeProxyChainConfig } from '@/services/cmds'
import { debugLog } from '@/utils/misc'

export const useProxiesPageController = () => {
  const [isChainMode, setIsChainMode] = useState(false)
  const [chainConfigData, setChainConfigData] = useState<string | null>(null)

  const onToggleChainMode = useLockFn(async () => {
    const nextChainMode = !isChainMode
    setIsChainMode(nextChainMode)

    if (!nextChainMode) {
      try {
        debugLog('Exiting chain mode, clearing chain configuration')
        await clearProxyChainRuntimeConfig()
        debugLog('Chain configuration cleared successfully')
      } catch (error) {
        console.error('Failed to clear chain configuration:', error)
      }
    }
  })

  useEffect(() => {
    if (!isChainMode) {
      setChainConfigData(null)
      return
    }

    let cancelled = false

    const fetchChainConfig = async () => {
      try {
        const exitNode = loadProxyChainRuntimeExitNode()

        if (!exitNode) {
          console.error('No proxy chain exit node found in localStorage')
          if (!cancelled) {
            setChainConfigData('')
          }
          return
        }

        const configData = await getRuntimeProxyChainConfig(exitNode)
        if (!cancelled) {
          setChainConfigData(configData || '')
        }
      } catch (error) {
        console.error('Failed to get runtime proxy chain config:', error)
        if (!cancelled) {
          setChainConfigData('')
        }
      }
    }

    void fetchChainConfig()

    return () => {
      cancelled = true
    }
  }, [isChainMode])

  return {
    isChainMode,
    chainConfigData,
    onToggleChainMode,
  }
}
