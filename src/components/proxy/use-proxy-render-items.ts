import { useMemo } from 'react'

import {
  useProxiesData,
  useRulesData,
} from '@/providers/app-data-context'

import {
  buildProxyRenderList,
  type ProxyRenderListBuilderOptions,
} from './proxy-render-items-builder'
import type { IRenderItem } from './render-list/types'

export type { ProxyRenderListBuilderOptions as UseProxyRenderItemsOptions }

export const useProxyRenderItems = ({
  col,
  headStates,
  latencyTimeout,
  mode,
  runtimeSummaryItem,
}: ProxyRenderListBuilderOptions): IRenderItem[] => {
  const { proxies: proxiesData } = useProxiesData()
  const { rules } = useRulesData()

  return useMemo(
    () =>
      buildProxyRenderList({
        col,
        headStates,
        latencyTimeout,
        mode,
        proxiesData,
        rules,
        runtimeSummaryItem,
      }),
    [
      col,
      headStates,
      latencyTimeout,
      mode,
      proxiesData,
      rules,
      runtimeSummaryItem,
    ],
  )
}
