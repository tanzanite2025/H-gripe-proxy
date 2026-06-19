import type { IProxyItem } from '@/types/proxy'

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
}

export const ProxyRenderProxyCard = ({
  compact = false,
  group,
  proxy,
  proxyKey,
  showType,
  clickable,
  onChangeProxy,
}: ProxyRenderProxyCardProps) => {
  const cardProps = {
    group,
    proxy,
    selected: group.now === proxy.name,
    showType,
    clickable,
    onClick: clickable ? () => onChangeProxy(group, proxy) : undefined,
  }

  if (compact) {
    return <ProxyItemMini key={proxyKey} {...cardProps} />
  }

  return <ProxyItem {...cardProps} />
}
