import { categorizeProxyGroup } from '@/services/proxy-display'
import { cn } from '@/utils/cn'

import { ProxyHead } from './proxy-head'
import { ProxyGroupCard } from './proxy-render/proxy-group-card'
import { ProxyRenderEmptyState } from './proxy-render/proxy-render-empty-state'
import { ProxyRenderProxyCard } from './proxy-render/proxy-render-proxy-card'
import { ProxyRenderProxyGrid } from './proxy-render/proxy-render-proxy-grid'
import { ProxyRuntimeSection } from './proxy-render/proxy-runtime-section'
import type { IRenderItem } from './render-list/types'
import { type HeadState } from './use-head-state'

interface RenderProps {
  item: IRenderItem
  indent: boolean
  onLocation: (group: NonNullable<IRenderItem['group']>) => void
  onCheckAll: (groupName: string) => void
  onHeadState: (groupName: string, patch: Partial<HeadState>) => void
  onChangeProxy: (
    group: NonNullable<IRenderItem['group']>,
    proxy: NonNullable<IRenderItem['proxy']> & { name: string },
  ) => void
  onConfigureStrategyGroup: (
    group: NonNullable<IRenderItem['group']>,
  ) => void
}

export const ProxyRender = ({
  indent,
  item,
  onLocation,
  onCheckAll,
  onHeadState,
  onChangeProxy,
  onConfigureStrategyGroup,
}: RenderProps) => {
  const { type, group, headState, proxy, proxyCol } = item
  const showType = headState?.showType
  const allowMemberSelection = group
    ? categorizeProxyGroup(group) !== 'strategy'
    : true

  if (type === 5) {
    return <ProxyRuntimeSection item={item} />
  }

  if (type === 0 && group) {
    return (
      <ProxyGroupCard
        group={group}
        headState={headState}
        item={item}
        onHeadState={onHeadState}
        onConfigureStrategyGroup={onConfigureStrategyGroup}
      />
    )
  }

  if (type === 1 && group) {
    return (
      <ProxyHead
        className={cn('mb-2 pl-4 pr-6', indent ? 'mt-2' : 'mt-1')}
        url={group.testUrl}
        groupName={group.name}
        headState={headState!}
        onLocation={() => onLocation(group)}
        onCheckDelay={() => onCheckAll(group.name)}
        onHeadState={(patch) => onHeadState(group.name, patch)}
      />
    )
  }

  if (type === 2 && group && proxy) {
    return (
      <ProxyRenderProxyCard
        group={group}
        proxy={proxy}
        showType={showType}
        clickable={allowMemberSelection}
        onChangeProxy={onChangeProxy}
        onConfigureStrategyGroup={onConfigureStrategyGroup}
      />
    )
  }

  if (type === 3) {
    return <ProxyRenderEmptyState />
  }

  if (type === 4 && group && proxyCol) {
    return (
      <ProxyRenderProxyGrid
        item={item}
        group={group}
        proxyCol={proxyCol}
        showType={showType}
        clickable={allowMemberSelection}
        onChangeProxy={onChangeProxy}
        onConfigureStrategyGroup={onConfigureStrategyGroup}
      />
    )
  }

  return null
}
