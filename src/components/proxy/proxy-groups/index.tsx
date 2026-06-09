import { ProxyGroupsChainView } from './components/proxy-groups-chain-view'
import { ProxyGroupsDefaultView } from './components/proxy-groups-default-view'
import { useProxyGroupsController } from './hooks/use-proxy-groups-controller'
import type { ProxyGroupsProps } from './types'

export const ProxyGroups = (props: ProxyGroupsProps) => {
  const controller = useProxyGroupsController(props)

  if (controller.isChainMode) {
    return <ProxyGroupsChainView controller={controller} />
  }

  return <ProxyGroupsDefaultView controller={controller} />
}
