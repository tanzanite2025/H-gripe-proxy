import { useLockFn } from 'ahooks'
import { Network, AlertTriangle } from 'lucide-react'
import { lazy, useCallback, useEffect, useReducer, useState, Suspense } from 'react'
import { useTranslation } from 'react-i18next'
import { closeAllConnections } from 'tauri-plugin-mihomo-api'

import { BasePage, TooltipIcon } from '@/components/base'
import { IpInfoCard } from '@/components/home/ip-info-card'
import { ProviderButton } from '@/components/proxy/provider-button'
import { ProxyGroups } from '@/components/proxy/proxy-groups'
import { Box, Button, ButtonGroup, Grid, Skeleton } from '@/components/tailwind'
import { useVerge } from '@/hooks/system'
import {
  useAppRefreshers,
  useClashConfigData,
} from '@/providers/app-data-context'
import {
  getRuntimeProxyChainConfig,
  patchClashMode,
  updateProxyChainConfigInRuntime,
} from '@/services/cmds'
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

const MODES = ['rule', 'global', 'direct'] as const
type Mode = (typeof MODES)[number]
const MODE_SET = new Set<string>(MODES)
const isMode = (value: unknown): value is Mode =>
  typeof value === 'string' && MODE_SET.has(value)

const ProxyPage = () => {
  const { t } = useTranslation()

  // 从 localStorage 恢复链式代理按钮状态
  const [isChainMode, setIsChainMode] = useState(() => {
    try {
      const saved = localStorage.getItem('proxy-chain-mode-enabled')
      return saved === 'true'
    } catch {
      return false
    }
  })

  const [chainConfigData, dispatchChainConfigData] = useReducer(
    (_: string | null, action: string | null) => action,
    null as string | null,
  )

  const { clashConfig } = useClashConfigData()
  const { refreshClashConfig } = useAppRefreshers()

  const updateChainConfigData = useCallback((value: string | null) => {
    dispatchChainConfigData(value)
  }, [])
  const { verge } = useVerge()

  const normalizedMode = clashConfig?.mode?.toLowerCase()
  const curMode = isMode(normalizedMode) ? normalizedMode : undefined
  const chainWarning = t('proxies.page.chain.warning')

  const onChangeMode = useLockFn(async (mode: Mode) => {
    // 断开连接
    if (mode !== curMode && verge?.auto_close_connection) {
      closeAllConnections()
    }
    await patchClashMode(mode)
    refreshClashConfig()
  })

  const onToggleChainMode = useLockFn(async () => {
    const newChainMode = !isChainMode

    setIsChainMode(newChainMode)
    // 保存链式代理按钮状态到 localStorage
    localStorage.setItem('proxy-chain-mode-enabled', newChainMode.toString())

    if (!newChainMode) {
      // 退出链式代理模式时，清除链式代理配置
      try {
        debugLog('Exiting chain mode, clearing chain configuration')
        await updateProxyChainConfigInRuntime(null)
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
        const exitNode = localStorage.getItem('proxy-chain-exit-node')

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
    if (normalizedMode && !isMode(normalizedMode)) {
      onChangeMode('rule')
    }
  }, [normalizedMode, onChangeMode])

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
          <Box className="flex items-center gap-1 mb-2">
            <ProviderButton />

            <ButtonGroup className="uds-toolbar" size="small">
              {MODES.map((mode) => (
                <Button
                  key={mode}
                  variant={mode === curMode ? 'primary' : 'outlined'}
                  onClick={() => onChangeMode(mode)}
                  className="capitalize"
                >
                  {t(`proxies.page.modes.${mode}`)}
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
              mode={curMode ?? 'rule'}
              isChainMode={isChainMode}
              chainConfigData={chainConfigData}
            />
          </div>
        </Grid>
        <Grid item xs={12} lg={6} xl={6} style={{ height: '100%', overflow: 'hidden' }}>
          <div className="h-full flex flex-col gap-4 overflow-y-auto pr-2 pb-4">
            <Suspense fallback={<Skeleton variant="rectangular" height={250} />}>
              <IpInfoCard />
            </Suspense>
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
