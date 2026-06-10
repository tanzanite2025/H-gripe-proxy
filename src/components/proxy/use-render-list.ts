import { useHeadStateNew } from './use-head-state'
import { useProxyRenderItems } from './use-proxy-render-items'
import { useRenderListRuntime } from './use-render-list-runtime'

export type { IRenderItem } from './render-list/types'

export const useRenderList = (isChainMode?: boolean) => {
  const [headStates, setHeadState] = useHeadStateNew()
  const runtimeContext = useRenderListRuntime(isChainMode)

  const renderList = useProxyRenderItems({
    headStates,
    col: runtimeContext.col,
    latencyTimeout: runtimeContext.latencyTimeout,
    runtimeSummaryItem: runtimeContext.runtimeSummaryItem,
  })

  return {
    renderList,
    onProxies: runtimeContext.onProxies,
    onHeadState: setHeadState,
  }
}
