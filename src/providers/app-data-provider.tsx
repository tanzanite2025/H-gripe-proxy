import { useQuery } from '@tanstack/react-query'
import { listen } from '@tauri-apps/api/event'
import React, { useCallback, useEffect, useMemo, useRef } from 'react'
import {
  getBaseConfig,
  getRuleProviders,
  getRules,
} from 'tauri-plugin-mihomo-api'

import { useVerge } from '@/hooks/system'
import {
  getAppUptime,
  getRunningMode,
  getSystemProxy,
} from '@/services/cmds'
import {
  calcuProxies,
  calcuProxyProviders,
} from '@/services/proxy-runtime'
import { queryClient } from '@/services/query-client'

import {
  ClashConfigContext,
  CoreDataStatusContext,
  ProxiesContext,
  RefreshersContext,
  RulesContext,
  SystemContext,
  UptimeContext,
} from './app-data-context'

const TQ_MIHOMO = {
  refetchOnWindowFocus: false,
  refetchOnReconnect: false,
  staleTime: 1500,
  retry: 3,
  retryDelay: (attempt: number) => Math.min(200 * 2 ** attempt, 3000),
} as const

const TQ_DEFAULTS = {
  refetchOnWindowFocus: false,
  refetchOnReconnect: false,
  staleTime: 5000,
  retry: 2,
} as const

function useStableFn<T extends (...args: any[]) => any>(fn: T): T {
  const ref = useRef(fn)
  ref.current = fn
  return useCallback((...args: Parameters<T>) => ref.current(...args), []) as T
}

// 鍏ㄥ眬鏁版嵁鎻愪緵鑰呯粍浠?
export const AppDataProvider = ({
  children,
}: {
  children: React.ReactNode
}) => {
  const { verge } = useVerge()

  const {
    data: proxiesData,
    isPending: isProxiesPending,
    refetch: _refetchProxy,
  } = useQuery({
    queryKey: ['getRuntimeProxyTopology'],
    queryFn: calcuProxies,
    ...TQ_MIHOMO,
    refetchInterval: 3000,
    refetchIntervalInBackground: false,
  })

  const {
    data: clashConfig,
    isPending: isClashConfigPending,
    refetch: _refetchClashConfig,
  } = useQuery({
    queryKey: ['getClashConfig'],
    queryFn: getBaseConfig,
    ...TQ_MIHOMO,
  })

  const { data: proxyProviders, refetch: _refetchProxyProviders } = useQuery({
    queryKey: ['getRuntimeProxyProviders'],
    queryFn: calcuProxyProviders,
    ...TQ_MIHOMO,
  })

  const { data: ruleProviders, refetch: _refetchRuleProviders } = useQuery({
    queryKey: ['getRuleProviders'],
    queryFn: getRuleProviders,
    ...TQ_MIHOMO,
  })

  const { data: rulesData, refetch: _refetchRules } = useQuery({
    queryKey: ['getRules'],
    queryFn: getRules,
    ...TQ_MIHOMO,
  })

  const { data: sysproxy, refetch: _refetchSysproxy } = useQuery({
    queryKey: ['getSystemProxy'],
    queryFn: getSystemProxy,
    ...TQ_DEFAULTS,
  })

  const { data: runningMode } = useQuery({
    queryKey: ['getRunningMode'],
    queryFn: getRunningMode,
    ...TQ_DEFAULTS,
  })

  const { data: uptimeData } = useQuery({
    queryKey: ['appUptime'],
    queryFn: getAppUptime,
    ...TQ_DEFAULTS,
    refetchInterval: 3000,
    retry: 1,
  })

  const refreshProxy = useStableFn(_refetchProxy)
  const refreshClashConfig = useStableFn(_refetchClashConfig)
  const refreshRules = useStableFn(_refetchRules)
  const refreshSysproxy = useStableFn(_refetchSysproxy)
  const refreshProxyProviders = useStableFn(_refetchProxyProviders)
  const refreshRuleProviders = useStableFn(_refetchRuleProviders)

  const refreshCoreData = useCallback(async () => {
    await Promise.all([
      refreshProxy().catch(() => {}),
      refreshClashConfig().catch(() => {}),
      refreshProxyProviders().catch(() => {}),
      refreshRules().catch(() => {}),
      refreshRuleProviders().catch(() => {}),
      queryClient.invalidateQueries({ queryKey: ['getRuntimeConfig'] }).catch(() => {}),
    ])
  }, [
    refreshProxy,
    refreshClashConfig,
    refreshProxyProviders,
    refreshRules,
    refreshRuleProviders,
  ])

  useEffect(() => {
    let lastProfileId: string | null = null
    let lastUpdateTime = 0
    const refreshThrottle = 800
    const cleanupFns: Array<() => void> = []

    const handleProfileChanged = (event: { payload: string }) => {
      const newProfileId = event.payload
      const now = Date.now()
      if (
        lastProfileId === newProfileId &&
        now - lastUpdateTime < refreshThrottle
      ) {
        return
      }
      lastProfileId = newProfileId
      lastUpdateTime = now
      void refreshCoreData()
    }

    const handleRefreshProxy = () => {
      const now = Date.now()
      if (now - lastUpdateTime <= refreshThrottle) return
      lastUpdateTime = now
      refreshProxy().catch(() => {})
    }

    const handleRefreshClash = () => {
      const now = Date.now()
      if (now - lastUpdateTime <= refreshThrottle) return
      lastUpdateTime = now
      void refreshCoreData()
    }

    const initializeListeners = async () => {
      try {
        const unlistenProfile = await listen<string>(
          'profile-changed',
          handleProfileChanged,
        )
        cleanupFns.push(unlistenProfile)
      } catch (error) {
        console.error(
          '[AppDataProvider] Failed to listen for profile updates:',
          error,
        )
      }

      try {
        const unlistenClash = await listen(
          'verge://refresh-clash-config',
          handleRefreshClash,
        )
        cleanupFns.push(unlistenClash)
      } catch (error) {
        console.warn(
          '[AppDataProvider] Failed to listen for Clash refresh events:',
          error,
        )
      }

      try {
        const unlistenProxy = await listen(
          'verge://refresh-proxy-config',
          handleRefreshProxy,
        )
        cleanupFns.push(unlistenProxy)
      } catch (error) {
        console.warn(
          '[AppDataProvider] Failed to listen for proxy refresh events:',
          error,
        )
      }
    }

    void initializeListeners()

    return () => {
      cleanupFns.forEach((fn) => {
        try {
          fn()
        } catch (error) {
          console.error('[DataProvider] Cleanup error:', error)
        }
      })
    }
  }, [refreshCoreData, refreshProxy])

  const refreshAll = useCallback(async () => {
    await Promise.all([
      refreshProxy(),
      refreshClashConfig(),
      refreshRules(),
      refreshSysproxy(),
      refreshProxyProviders(),
      refreshRuleProviders(),
    ])
  }, [
    refreshProxy,
    refreshClashConfig,
    refreshRules,
    refreshSysproxy,
    refreshProxyProviders,
    refreshRuleProviders,
  ])

  const proxiesValue = useMemo(
    () => ({
      proxies: proxiesData,
      proxyProviders: proxyProviders || {},
      isProxiesPending,
    }),
    [proxiesData, proxyProviders, isProxiesPending],
  )

  const rulesValue = useMemo(
    () => ({
      rules: rulesData?.rules ?? [],
      ruleProviders: ruleProviders?.providers || {},
    }),
    [rulesData, ruleProviders],
  )

  const clashConfigValue = useMemo(
    () => ({
      clashConfig,
      isClashConfigPending,
    }),
    [clashConfig, isClashConfigPending],
  )

  const systemValue = useMemo(() => {
    const calculateSystemProxyAddress = () => {
      if (!verge || !clashConfig) return '-'

      const isPacMode = verge.proxy_auto_config ?? false

      if (isPacMode) {
        // In PAC mode we show the proxy address we expect to configure.
        const proxyHost = verge.proxy_host || '127.0.0.1'
        const proxyPort =
          verge.verge_mixed_port || clashConfig.mixedPort || 7897
        return `${proxyHost}:${proxyPort}`
      } else {
        // In HTTP mode prefer the system address when it looks valid.
        const systemServer = sysproxy?.server
        if (
          systemServer &&
          systemServer !== '-' &&
          !systemServer.startsWith(':')
        ) {
          return systemServer
        } else {
          // Fall back to the expected proxy address when the system value is invalid.
          const proxyHost = verge.proxy_host || '127.0.0.1'
          const proxyPort =
            verge.verge_mixed_port || clashConfig.mixedPort || 7897
          return `${proxyHost}:${proxyPort}`
        }
      }
    }

    return {
      sysproxy,
      runningMode,
      systemProxyAddress: calculateSystemProxyAddress(),
    }
  }, [sysproxy, runningMode, verge, clashConfig])

  const uptimeValue = useMemo(() => ({ uptime: uptimeData || 0 }), [uptimeData])

  const coreDataStatusValue = useMemo(
    () => ({ isCoreDataPending: isProxiesPending || isClashConfigPending }),
    [isProxiesPending, isClashConfigPending],
  )

  const refreshersValue = useMemo(
    () => ({
      refreshProxy,
      refreshClashConfig,
      refreshRules,
      refreshSysproxy,
      refreshProxyProviders,
      refreshRuleProviders,
      refreshAll,
    }),
    [
      refreshProxy,
      refreshClashConfig,
      refreshRules,
      refreshSysproxy,
      refreshProxyProviders,
      refreshRuleProviders,
      refreshAll,
    ],
  )

  return (
    <ProxiesContext value={proxiesValue}>
      <RulesContext value={rulesValue}>
        <ClashConfigContext value={clashConfigValue}>
          <SystemContext value={systemValue}>
            <UptimeContext value={uptimeValue}>
              <CoreDataStatusContext value={coreDataStatusValue}>
                <RefreshersContext value={refreshersValue}>
                  {children}
                </RefreshersContext>
              </CoreDataStatusContext>
            </UptimeContext>
          </SystemContext>
        </ClashConfigContext>
      </RulesContext>
    </ProxiesContext>
  )
}
