import { useHeadStateNew } from './use-head-state'
import { useProxyRenderItems } from './use-proxy-render-items'
import { useRenderListRuntime } from './use-render-list-runtime'

export type { IRenderItem } from './render-list/types'

export const useRenderList = (mode: string, isChainMode?: boolean) => {
  const [headStates, setHeadState] = useHeadStateNew()
  const runtimeContext = useRenderListRuntime(mode, isChainMode)

  const renderList = useProxyRenderItems({
    mode,
    headStates,
    col: runtimeContext.col,
    latencyTimeout: runtimeContext.latencyTimeout,
    runtimeSummaryItem: runtimeContext.runtimeSummaryItem,
    strategyGroupOverrides: runtimeContext.strategyGroupOverrides,
    managedStrategyGroupNames: runtimeContext.managedStrategyGroupNames,
  })

  return {
    renderList,
    onProxies: runtimeContext.onProxies,
    onHeadState: setHeadState,
  }
}
