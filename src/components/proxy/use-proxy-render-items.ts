import { useMemo } from 'react'

import { useProxiesData } from '@/providers/app-data-context'

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
  runtimeSummaryItem,
}: ProxyRenderListBuilderOptions): IRenderItem[] => {
  const { proxies: proxiesData } = useProxiesData()

  return useMemo(
    () =>
      buildProxyRenderList({
        col,
        headStates,
        latencyTimeout,
        proxiesData,
        runtimeSummaryItem,
      }),
    [
      col,
      headStates,
      latencyTimeout,
      proxiesData,
      runtimeSummaryItem,
    ],
  )
}
