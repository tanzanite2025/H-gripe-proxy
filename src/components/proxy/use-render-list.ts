import { useQuery } from '@tanstack/react-query'
import { useEffect, useMemo } from 'react'

import { useProfiles } from '@/hooks/data'
import { useRuntimeConfig } from '@/hooks/data/use-clash'
import { useVerge } from '@/hooks/system'
import {
  useAppRefreshers,
  useProxiesData,
} from '@/providers/app-data-context'
import { readProfileFile } from '@/services/cmds'
import delayManager from '@/services/delay'
import { debugLog } from '@/utils/misc'

import { parseGroupsYaml } from '../profile/groups-editor-viewer/utils/group-helpers'
import {
  calculateColumns,
  type ProxyItem,
} from './render-list-shared'
import {
  useHeadStateNew,
} from './use-head-state'
import { useProxyRenderItems } from './use-proxy-render-items'
import { useRuntimeSummaryItem } from './use-runtime-summary-item'
import { useWindowWidth } from './use-window-width'

export type { IRenderItem } from './render-list-shared'

const normalizeNames = (names: Array<string | null | undefined>) =>
  Array.from(
    new Set(
      names
        .map((name) => name?.trim() || '')
        .filter((name) => name.length > 0),
    ),
  )

export const useRenderList = (mode: string, isChainMode?: boolean) => {
  const { proxies: proxiesData } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const { verge } = useVerge()
  const { current } = useProfiles()
  const { width } = useWindowWidth()
  const [headStates, setHeadState] = useHeadStateNew()
  const latencyTimeout = verge?.default_latency_timeout

  const { data: runtimeConfig } = useRuntimeConfig(!!isChainMode)
  const runtimeSummaryItem = useRuntimeSummaryItem(mode)
  const groupsOverridePath = current?.option?.groups?.trim() || ''
  const { data: strategyGroupOverrides = {} } = useQuery({
    queryKey: ['proxy-strategy-group-overrides', groupsOverridePath],
    enabled: !!groupsOverridePath,
    staleTime: 3_000,
    refetchOnWindowFocus: false,
    queryFn: async () => {
      const groupsData = await readProfileFile(groupsOverridePath)
      const sequence = parseGroupsYaml(groupsData)
      const overrides: Record<string, string[]> = {}

      ;([...sequence.prepend, ...sequence.append] as IProxyGroupConfig[]).forEach(
        (group) => {
          const name = group?.name?.trim()
          if (!name) return

          overrides[name] = Array.isArray(group.proxies)
            ? normalizeNames(group.proxies)
            : []
        },
      )

      return overrides
    },
  })

  const col = useMemo(
    () => calculateColumns(width, verge?.proxy_layout_column || 6),
    [width, verge?.proxy_layout_column],
  )

  useEffect(() => {
    if (!proxiesData) return
    const { groups, proxies } = proxiesData

    if (
      (mode === 'rule' && !groups.length) ||
      (mode === 'global' && proxies.length < 2)
    ) {
      const handle = setTimeout(() => refreshProxy(), 500)
      return () => clearTimeout(handle)
    }
  }, [proxiesData, mode, refreshProxy])

  useEffect(() => {
    if (!isChainMode || !runtimeConfig) return

    const allProxies: ProxyItem[] = Object.values(
      (runtimeConfig as any).proxies || {},
    )
    if (allProxies.length === 0) return

    const groupListener = () => {
      debugLog('[ChainMode] delay updated, refreshing proxy view')
      refreshProxy()
    }

    delayManager.setGroupListener('chain-mode', groupListener)

    const calculateDelays = async () => {
      try {
        const timeout = verge?.default_latency_timeout || 10000
        const proxyNames = allProxies.map((proxy) => proxy.name)

        debugLog(
          `[ChainMode] calculating delay for ${proxyNames.length} proxies`,
        )
        delayManager.checkListDelay(proxyNames, 'chain-mode', timeout)
      } catch (error) {
        console.error('Failed to calculate delays for chain mode:', error)
      }
    }

    const handle = setTimeout(calculateDelays, 100)

    return () => {
      clearTimeout(handle)
      delayManager.removeGroupListener('chain-mode')
    }
  }, [isChainMode, runtimeConfig, verge?.default_latency_timeout, refreshProxy])

  const renderList = useProxyRenderItems({
    mode,
    headStates,
    col,
    latencyTimeout,
    runtimeSummaryItem,
    strategyGroupOverrides,
  })

  return {
    renderList,
    onProxies: refreshProxy,
    onHeadState: setHeadState,
  }
}
