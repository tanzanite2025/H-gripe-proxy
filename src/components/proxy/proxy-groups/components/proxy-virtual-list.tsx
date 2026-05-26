import { useTheme } from '@mui/material/styles'
import type { Key, RefObject } from 'react'

import { ProxyRender } from '../../proxy-render'
import type { HeadState } from '../../use-head-state'
import type { IRenderItem } from '../../use-render-list'

type VirtualListItem = {
  key: Key
  index: number
  start: number
  end: number
}

interface ProxyVirtualListProps {
  parentRef: RefObject<HTMLDivElement | null>
  height: string
  totalSize: number
  virtualItems: VirtualListItem[]
  renderList: IRenderItem[]
  activeStickyIndex: number | null
  indent: boolean
  isChainMode?: boolean
  measureElement: (node: Element | null) => void
  onLocation: (group: IRenderItem['group']) => void
  onCheckAll: (groupName: string) => void
  onHeadState: (groupName: string, patch: Partial<HeadState>) => void
  onChangeProxy: (
    group: IRenderItem['group'],
    proxy: IRenderItem['proxy'] & { name: string },
  ) => void
}

/**
 * 代理虚拟列表组件 - 使用虚拟滚动优化性能
 */
export function ProxyVirtualList({
  parentRef,
  height,
  totalSize,
  virtualItems,
  renderList,
  activeStickyIndex,
  indent,
  isChainMode,
  measureElement,
  onLocation,
  onCheckAll,
  onHeadState,
  onChangeProxy,
}: ProxyVirtualListProps) {
  const theme = useTheme()
  const stickyBackground =
    theme.palette.mode === 'dark' ? '#1e1f27' : 'var(--background-color)'

  return (
    <div ref={parentRef} style={{ height, overflow: 'auto' }}>
      <div style={{ height: totalSize, position: 'relative' }}>
        {virtualItems.map((virtualItem) => (
          <div
            key={virtualItem.key}
            data-index={virtualItem.index}
            ref={measureElement}
            style={{
              position:
                virtualItem.index === activeStickyIndex ? 'sticky' : 'absolute',
              top: 0,
              left: 0,
              zIndex: virtualItem.index === activeStickyIndex ? 5 : undefined,
              display:
                virtualItem.index === activeStickyIndex
                  ? 'flow-root'
                  : undefined,
              backgroundColor:
                virtualItem.index === activeStickyIndex
                  ? stickyBackground
                  : undefined,
              width: '100%',
              transform:
                virtualItem.index === activeStickyIndex
                  ? undefined
                  : `translateY(${virtualItem.start}px)`,
            }}
          >
            <ProxyRender
              item={renderList[virtualItem.index]}
              indent={indent}
              onLocation={onLocation}
              onCheckAll={onCheckAll}
              onHeadState={onHeadState}
              onChangeProxy={onChangeProxy}
              isChainMode={isChainMode}
            />
          </div>
        ))}
        <div style={{ height: 8 }} />
      </div>
    </div>
  )
}
