import { useCallback } from 'react'

import type { HeadState } from '../../use-head-state'
import type { IRenderItem } from '../../use-render-list'
import type { ProxyVirtualListProps } from '../components/proxy-virtual-list'

import { useScrollPosition } from './use-scroll-position'
import { useVirtualScroll } from './use-virtual-scroll'

interface UseProxyGroupsListViewOptions {
  activeSelectedGroup: string | null
  handleChangeProxy: (group: IProxyGroupItem, proxy: IProxyItem) => void
  handleCheckAll: (groupName: string) => void
  handleGroupLocationByName: (
    groupName: string,
    scrollToIndex: (index: number, options?: any) => void,
  ) => void
  handleLocation: (
    group: IProxyGroupItem,
    scrollToIndex: (index: number, options?: any) => void,
  ) => void
  isChainMode: boolean
  onHeadState: (groupName: string, patch: Partial<HeadState>) => void
  renderList: IRenderItem[]
}

export function useProxyGroupsListView({
  activeSelectedGroup,
  handleChangeProxy,
  handleCheckAll,
  handleGroupLocationByName,
  handleLocation,
  isChainMode,
  onHeadState,
  renderList,
}: UseProxyGroupsListViewOptions) {
  const scrollPosition = useScrollPosition({
    isChainMode,
    activeSelectedGroup,
    renderListLength: renderList.length,
  })

  const virtualScroll = useVirtualScroll({
    renderList,
    parentRef: scrollPosition.parentRef,
  })

  const handleLocationWithScroll = useCallback(
    (group: IProxyGroupItem) => {
      handleLocation(group, virtualScroll.scrollToIndex)
    },
    [handleLocation, virtualScroll.scrollToIndex],
  )

  const handleGroupLocationByNameWithScroll = useCallback(
    (groupName: string) => {
      handleGroupLocationByName(groupName, virtualScroll.scrollToIndex)
    },
    [handleGroupLocationByName, virtualScroll.scrollToIndex],
  )

  const createProxyListProps = useCallback(
    (height: string): ProxyVirtualListProps => ({
      parentRef: scrollPosition.parentRef,
      height,
      totalSize: virtualScroll.totalSize,
      virtualItems: virtualScroll.virtualItems,
      renderList,
      activeStickyIndex: virtualScroll.activeStickyIndex,
      indent: false,
      measureElement: virtualScroll.measureElement,
      onLocation: handleLocationWithScroll,
      onCheckAll: handleCheckAll,
      onHeadState,
      onChangeProxy: handleChangeProxy,
    }),
    [
      handleChangeProxy,
      handleCheckAll,
      handleLocationWithScroll,
      onHeadState,
      renderList,
      scrollPosition.parentRef,
      virtualScroll.activeStickyIndex,
      virtualScroll.measureElement,
      virtualScroll.totalSize,
      virtualScroll.virtualItems,
    ],
  )

  return {
    createProxyListProps,
    handleGroupLocationByNameWithScroll,
    scrollToTop: scrollPosition.scrollToTop,
    showScrollTop: scrollPosition.showScrollTop,
  }
}
