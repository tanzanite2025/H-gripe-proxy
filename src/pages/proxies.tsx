import { useLockFn } from 'ahooks'
import { Network, AlertTriangle } from 'lucide-react'
import { lazy, useCallback, useEffect, useReducer, useState, Suspense } from 'react'
import { useTranslation } from 'react-i18next'

import { BasePage, TooltipIcon } from '@/components/base'
import { ProviderButton } from '@/components/proxy/provider-button'
import { clearProxyChainRuntimeConfig } from '@/components/proxy/proxy-chain-runtime'
import { loadProxyChainRuntimeExitNode } from '@/components/proxy/proxy-chain-types'
import { ProxyGroups } from '@/components/proxy/proxy-groups'
import { Box, Button, ButtonGroup, Grid, Skeleton } from '@/components/tailwind'
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
import {
  getRuntimeProxyChainConfig,
  patchClashMode,
} from '@/services/cmds'
import { queryClient } from '@/services/query-client'
import { debugLog } from '@/utils/misc'

const LazyProxyDetectionCard = lazy(() =>
  import('@/components/home/proxy-detection-card').then((module) => ({
    default: module.ProxyDetectionCard,
  })),
)

const LazyDNSLeakCard = lazy(() =>
  import('@/components/home/dns-leak-card').then((module) => ({
    default: module.DNSLeakCard,
  })),
)

const PROXY_CHAIN_MODES = ['rule', 'global'] as const
type ProxyChainMode = (typeof PROXY_CHAIN_MODES)[number]
const PROXY_CHAIN_MODE_LABELS = {
  rule: '应用规则',
  global: '不应用规则',
} as const

const ProxyPage = () => {
  const { t } = useTranslation()

  const [isChainMode, setIsChainMode] = useState(false)

  const [chainConfigData, dispatchChainConfigData] = useReducer(
    (_: string | null, action: string | null) => action,
    null as string | null,
  )

  const { clashConfig } = useClashConfigData()
  const { refreshClashConfig } = useAppRefreshers()
  const { data: runtimeConfig } = useRuntimeConfig()
  const [optimisticMode, setOptimisticMode] = useState<ClashMode | undefined>()

  const updateChainConfigData = useCallback((value: string | null) => {
    dispatchChainConfigData(value)
  }, [])
  const curMode = resolveClashMode(clashConfig?.mode, runtimeConfig?.mode)
  const displayMode = optimisticMode ?? curMode
  const proxyDisplayMode = displayMode === 'direct' ? DEFAULT_CLASH_MODE : displayMode
  const chainWarning = t('proxies.page.chain.warning')

  const onChangeMode = useLockFn(async (mode: ProxyChainMode) => {
    // 断开连接
    setOptimisticMode(mode)
    queryClient.setQueryData(['getClashConfig'], (old: any) =>
      old ? { ...old, mode } : old,
    )
    queryClient.setQueryData(['getRuntimeConfig'], (old: any) =>
      old ? { ...old, mode } : old,
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
    const newChainMode = !isChainMode

    setIsChainMode(newChainMode)

    if (!newChainMode) {
      // 退出链式代理模式时，清除链式代理配置
      try {
        debugLog('Exiting chain mode, clearing chain configuration')
        await clearProxyChainRuntimeConfig()
        debugLog('Chain configuration cleared successfully')
      } catch (error) {
        console.error('Failed to clear chain configuration:', error)
      }
    }
  })

  // 当开启链式代理模式时，获取配置数据
  useEffect(() => {
    if (!isChainMode) {
      updateChainConfigData(null)
      return
    }

    let cancelled = false

    const fetchChainConfig = async () => {
      try {
        const exitNode = loadProxyChainRuntimeExitNode()

        if (!exitNode) {
          console.error('No proxy chain exit node found in localStorage')
          if (!cancelled) {
            updateChainConfigData('')
          }
          return
        }

        const configData = await getRuntimeProxyChainConfig(exitNode)
        if (!cancelled) {
          updateChainConfigData(configData || '')
        }
      } catch (error) {
        console.error('Failed to get runtime proxy chain config:', error)
        if (!cancelled) {
          updateChainConfigData('')
        }
      }
    }

    fetchChainConfig()

    return () => {
      cancelled = true
    }
  }, [isChainMode, updateChainConfigData])

  useEffect(() => {
    const hasMode =
      typeof clashConfig?.mode === 'string' ||
      typeof runtimeConfig?.mode === 'string'
    if (hasMode && !resolveClashMode(clashConfig?.mode, runtimeConfig?.mode)) {
      onChangeMode('rule')
    }
  }, [clashConfig?.mode, runtimeConfig?.mode, onChangeMode])

  return (
    <BasePage
      full
      contentStyle={{ height: '100%', paddingTop: '15px' }}
      title={
        isChainMode ? (
          <Box
            component="span"
            data-tauri-drag-region="true"
            className="inline-flex items-center gap-3"
          >
            {t('proxies.page.title.chainMode')}
            <TooltipIcon
              title={chainWarning}
              icon={AlertTriangle}
              color="warning"
              className="p-1"
            />
          </Box>
        ) : (
          t('proxies.page.title.default')
        )
      }
    >
      <Grid container spacing={3} style={{ height: '100%' }} columns={12}>
        <Grid item xs={12} lg={6} xl={6} style={{ height: '100%', overflow: 'hidden' }}>
          <Box className="flex items-center gap-1 mb-2 pl-3">
            <ProviderButton />

            <ButtonGroup className="uds-toolbar" size="small">
              {PROXY_CHAIN_MODES.map((mode) => (
                <Button
                  key={mode}
                  variant={mode === proxyDisplayMode ? 'primary' : 'outlined'}
                  onClick={() => onChangeMode(mode)}
                >
                  {PROXY_CHAIN_MODE_LABELS[mode]}
                </Button>
              ))}
            </ButtonGroup>

            <Button
              size="small"
              variant={isChainMode ? 'primary' : 'outlined'}
              onClick={onToggleChainMode}
              className="ml-1"
              startIcon={<Network className="h-5 w-5" />}
            >
              {t('proxies.page.actions.toggleChain')}
            </Button>
          </Box>
          <div style={{ height: 'calc(100% - 36px)', overflow: 'hidden' }}>
            <ProxyGroups
              mode={proxyDisplayMode ?? DEFAULT_CLASH_MODE}
              isChainMode={isChainMode}
              chainConfigData={chainConfigData}
              onCloseChainMode={onToggleChainMode}
            />
          </div>
        </Grid>
        <Grid item xs={12} lg={6} xl={6} style={{ height: '100%', overflow: 'hidden' }}>
          <div className="h-full flex flex-col gap-4 overflow-y-auto pr-2 pb-4">
            <Suspense fallback={<Skeleton variant="rectangular" height={200} />}>
              <LazyProxyDetectionCard />
            </Suspense>
            <Suspense fallback={<Skeleton variant="rectangular" height={200} />}>
              <LazyDNSLeakCard />
            </Suspense>
          </div>
        </Grid>
      </Grid>
    </BasePage>
  )
}

export default ProxyPage
