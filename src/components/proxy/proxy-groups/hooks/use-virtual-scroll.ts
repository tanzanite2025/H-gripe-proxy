import { defaultRangeExtractor, useVirtualizer } from '@tanstack/react-virtual'
import { useCallback, useMemo, useRef } from 'react'

import type { IRenderItem } from '../../use-render-list'

interface UseVirtualScrollOptions {
  renderList: IRenderItem[]
  parentRef: React.RefObject<HTMLDivElement | null>
}

/**
 * 管理虚拟滚动逻辑
 */
export function useVirtualScroll(options: UseVirtualScrollOptions) {
  const { renderList, parentRef } = options

  const activeStickyIndexRef = useRef<number | null>(null)

  // 获取所有粘性组索引（用于组头部固定）
  const stickyGroupIndexes = useMemo(
    () =>
      renderList.flatMap((item, index) =>
        item.type === 0 && !item.group?.hidden ? [index] : [],
      ),
    [renderList],
  )

  // 自定义范围提取器 - 确保粘性组头部始终渲染
  const rangeExtractor = useCallback(
    (range: Parameters<typeof defaultRangeExtractor>[0]) => {
      const activeStickyIndex = [...stickyGroupIndexes]
        .reverse()
        .find((index) => index <= range.startIndex)
      activeStickyIndexRef.current = activeStickyIndex ?? null

      const indexes = defaultRangeExtractor(range)
      return activeStickyIndex == null || indexes.includes(activeStickyIndex)
        ? indexes
        : [activeStickyIndex, ...indexes]
    },
    [stickyGroupIndexes],
  )

  // 创建虚拟滚动器
  const virtualizer = useVirtualizer({
    count: renderList.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 56,
    overscan: 15,
    getItemKey: (index) => renderList[index]?.key ?? index,
    rangeExtractor,
  })

  const virtualItems = virtualizer.getVirtualItems()
  const activeStickyIndex = activeStickyIndexRef.current

  // 滚动到指定索引
  const scrollToIndex = useCallback(
    (index: number, options?: { align?: 'start' | 'center' | 'end'; behavior?: ScrollBehavior }) => {
      virtualizer.scrollToIndex(index, options)
    },
    [virtualizer],
  )

  return {
    virtualizer,
    virtualItems,
    activeStickyIndex,
    scrollToIndex,
    totalSize: virtualizer.getTotalSize(),
    measureElement: virtualizer.measureElement,
  }
}
