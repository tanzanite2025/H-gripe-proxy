import { useMemo } from 'react'

import { useProfiles } from '@/hooks/data'
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
import { useManagedStrategyGroupNames } from './render-list-runtime/use-managed-strategy-group-names'
import { useProxyRefreshRecovery } from './render-list-runtime/use-proxy-refresh-recovery'
import { useStrategyGroupOverrides } from './render-list-runtime/use-strategy-group-overrides'

export interface RenderListRuntimeContext {
  col: number
  latencyTimeout: number
  managedStrategyGroupNames: string[]
  onProxies: () => void
  runtimeSummaryItem: IRenderItem | null
  strategyGroupOverrides: Record<string, string[]>
}

export const useRenderListRuntime = (
  mode: string,
  isChainMode?: boolean,
): RenderListRuntimeContext => {
  const { proxies: proxiesData } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const { verge } = useVerge()
  const { current } = useProfiles()
  const { width } = useWindowWidth()
  const latencyTimeout = resolveVergeDelayTimeout(verge)

  const { data: runtimeConfig } = useRuntimeConfig(!!isChainMode)
  const runtimeSummaryItem = useRuntimeSummaryItem(mode)
  const groupsOverridePath = current?.option?.groups?.trim() || ''
  const profileUid = current?.uid?.trim() || ''

  const { data: strategyGroupOverrides = {} } =
    useStrategyGroupOverrides(groupsOverridePath)
  const { data: managedStrategyGroupNames = [] } =
    useManagedStrategyGroupNames(profileUid, groupsOverridePath)

  const col = useMemo(
    () => calculateColumns(width, verge?.proxy_layout_column || 6),
    [width, verge?.proxy_layout_column],
  )

  useProxyRefreshRecovery({
    mode,
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
    managedStrategyGroupNames,
    onProxies: refreshProxy,
    runtimeSummaryItem,
    strategyGroupOverrides,
  }
}
