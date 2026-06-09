import { useMemo } from 'react'

import type { IRenderItem } from '../render-list/types'
import { ProxyRenderProxyCard } from './proxy-render-proxy-card'

interface ProxyRenderProxyGridProps {
  item: IRenderItem
  group: NonNullable<IRenderItem['group']>
  proxyCol: NonNullable<IRenderItem['proxyCol']>
  showType?: boolean
  clickable: boolean
  onChangeProxy: (
    group: NonNullable<IRenderItem['group']>,
    proxy: NonNullable<IRenderItem['proxy']> & { name: string },
  ) => void
}

export const ProxyRenderProxyGrid = ({
  item,
  group,
  proxyCol,
  showType,
  clickable,
  onChangeProxy,
}: ProxyRenderProxyGridProps) => {
  const proxyItems = useMemo(() => {
    return proxyCol.map((proxy) => (
      <ProxyRenderProxyCard
        key={`${item.key}-${proxy?.name ?? 'unknown'}`}
        compact
        group={group}
        proxy={proxy}
        showType={showType}
        clickable={clickable}
        onChangeProxy={onChangeProxy}
      />
    ))
  }, [clickable, group, item.key, onChangeProxy, proxyCol, showType])

  return (
    <div
      className="grid gap-2 px-4 py-1"
      style={{
        gridTemplateColumns: `repeat(${item.col || 2}, minmax(0, 1fr))`,
      }}
    >
      {proxyItems}
    </div>
  )
}
