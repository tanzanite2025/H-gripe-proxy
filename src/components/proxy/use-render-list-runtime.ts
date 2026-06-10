import { useMemo } from 'react'

import { useRuntimeConfig } from '@/hooks/data/use-clash'
import { useVerge } from '@/hooks/system'
import {
  useAppRefreshers,
  useProxiesData,
} from '@/providers/app-data-context'
import { resolveVergeDelayTimeout } from '@/services/delay-config'

import { calculateColumns } from './render-list/utils'
import type { IRenderItem } from './render-list/types'
import { useRuntimeSummaryItem } from './use-runtime-summary-item'
import { useWindowWidth } from './use-window-width'
import { useChainModeDelaySync } from './render-list-runtime/use-chain-mode-delay-sync'
import { useProxyRefreshRecovery } from './render-list-runtime/use-proxy-refresh-recovery'

export interface RenderListRuntimeContext {
  col: number
  latencyTimeout: number
  onProxies: () => void
  runtimeSummaryItem: IRenderItem | null
}

export const useRenderListRuntime = (
  isChainMode?: boolean,
): RenderListRuntimeContext => {
  const { proxies: proxiesData } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const { verge } = useVerge()
  const { width } = useWindowWidth()
  const latencyTimeout = resolveVergeDelayTimeout(verge)

  const { data: runtimeConfig } = useRuntimeConfig(!!isChainMode)
  const runtimeSummaryItem = useRuntimeSummaryItem()

  const col = useMemo(
    () => calculateColumns(width, verge?.proxy_layout_column || 6),
    [width, verge?.proxy_layout_column],
  )

  useProxyRefreshRecovery({
    onProxies: refreshProxy,
    proxiesData,
  })

  useChainModeDelaySync({
    isChainMode,
    latencyTimeout,
    onProxies: refreshProxy,
    runtimeConfig,
  })

  return {
    col,
    latencyTimeout,
    onProxies: refreshProxy,
    runtimeSummaryItem,
  }
}
