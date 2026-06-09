import { categorizeProxyGroup } from '@/services/proxy-display'

import { ProxyItem } from '../proxy-item'
import { ProxyItemMini } from '../proxy-item-mini'
import type { IRenderItem } from '../render-list/types'

interface ProxyRenderProxyCardProps {
  compact?: boolean
  group: NonNullable<IRenderItem['group']>
  proxy: IProxyItem
  proxyKey?: string
  showType?: boolean
  clickable: boolean
  onChangeProxy: (
    group: NonNullable<IRenderItem['group']>,
    proxy: NonNullable<IRenderItem['proxy']> & { name: string },
  ) => void
  onConfigureStrategyGroup: (
    group: NonNullable<IRenderItem['group']>,
  ) => void
}

const resolveConfigureHandler = (
  proxy: IProxyItem,
  onConfigureStrategyGroup: (
    group: NonNullable<IRenderItem['group']>,
  ) => void,
) => {
  return categorizeProxyGroup(proxy) === 'strategy'
    ? (strategyGroup: NonNullable<IRenderItem['group']>) =>
        onConfigureStrategyGroup(strategyGroup)
    : undefined
}

export const ProxyRenderProxyCard = ({
  compact = false,
  group,
  proxy,
  proxyKey,
  showType,
  clickable,
  onChangeProxy,
  onConfigureStrategyGroup,
}: ProxyRenderProxyCardProps) => {
  const cardProps = {
    group,
    proxy,
    selected: group.now === proxy.name,
    showType,
    clickable,
    onClick: clickable ? () => onChangeProxy(group, proxy) : undefined,
    onConfigure: resolveConfigureHandler(proxy, onConfigureStrategyGroup),
  }

  if (compact) {
    return <ProxyItemMini key={proxyKey} {...cardProps} />
  }

  return <ProxyItem {...cardProps} />
}
