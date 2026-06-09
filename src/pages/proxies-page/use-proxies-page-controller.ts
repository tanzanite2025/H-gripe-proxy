import { useLockFn } from 'ahooks'
import { useEffect, useState } from 'react'

import { clearProxyChainRuntimeConfig } from '@/components/proxy/proxy-chain-runtime'
import { loadProxyChainRuntimeExitNode } from '@/components/proxy/proxy-chain-types'
import { useRuntimeConfig } from '@/hooks/data/use-clash'
import {
  useAppRefreshers,
  useClashConfigData,
} from '@/providers/app-data-context'
import {
  DEFAULT_CLASH_MODE,
  type ClashMode,
  resolveClashMode,
} from '@/services/clash-mode'
import { getRuntimeProxyChainConfig, patchClashMode } from '@/services/cmds'
import { queryClient } from '@/services/query-client'
import { debugLog } from '@/utils/misc'

import type { ProxyChainMode } from './shared'

const resolveProxyPageMode = (
  mode: ClashMode | undefined,
): ProxyChainMode => (mode === 'global' ? 'global' : 'rule')

export const useProxiesPageController = () => {
  const [isChainMode, setIsChainMode] = useState(false)
  const [chainConfigData, setChainConfigData] = useState<string | null>(null)
  const [optimisticMode, setOptimisticMode] = useState<ClashMode | undefined>()

  const { clashConfig } = useClashConfigData()
  const { refreshClashConfig } = useAppRefreshers()
  const { data: runtimeConfig } = useRuntimeConfig()

  const currentMode = resolveClashMode(clashConfig?.mode, runtimeConfig?.mode)
  const displayMode = optimisticMode ?? currentMode
  const proxyDisplayMode = resolveProxyPageMode(displayMode)

  const onChangeMode = useLockFn(async (mode: ProxyChainMode) => {
    setOptimisticMode(mode)
    queryClient.setQueryData(['getClashConfig'], (old: IConfigData | undefined) =>
      old ? { ...old, mode } : old,
    )
    queryClient.setQueryData(
      ['getRuntimeConfig'],
      (old: IConfigData | undefined) => (old ? { ...old, mode } : old),
    )

    try {
      await patchClashMode(mode)
    } finally {
      await Promise.all([
        refreshClashConfig(),
        queryClient.invalidateQueries({ queryKey: ['getRuntimeConfig'] }),
      ])
      setOptimisticMode(undefined)
    }
  })

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

  useEffect(() => {
    const hasMode =
      typeof clashConfig?.mode === 'string' ||
      typeof runtimeConfig?.mode === 'string'

    if (hasMode && !resolveClashMode(clashConfig?.mode, runtimeConfig?.mode)) {
      void onChangeMode(resolveProxyPageMode(DEFAULT_CLASH_MODE))
    }
  }, [clashConfig?.mode, runtimeConfig?.mode, onChangeMode])

  return {
    isChainMode,
    chainConfigData,
    proxyDisplayMode,
    onChangeMode,
    onToggleChainMode,
  }
}
