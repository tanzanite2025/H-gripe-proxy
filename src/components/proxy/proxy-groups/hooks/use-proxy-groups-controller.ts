import { useCallback } from 'react'

import type { ProxyGroupsProps } from '../types'

import { useChainMode } from './use-chain-mode'
import { useDelayCheck } from './use-delay-check'
import { useProxyGroups } from './use-proxy-groups'
import { useProxyGroupsListView } from './use-proxy-groups-list-view'

export function useProxyGroupsController(props: ProxyGroupsProps) {
  const { mode, isChainMode = false, chainConfigData, onCloseChainMode } = props
  const displayMode = mode === 'direct' ? 'rule' : mode

  const chainMode = useChainMode({
    isChainMode,
    mode: displayMode,
  })

  const proxyGroups = useProxyGroups({
    mode: displayMode,
    isChainMode,
  })

  const { handleCheckAll } = useDelayCheck({
    renderList: proxyGroups.renderList,
    timeout: proxyGroups.timeout,
    getGroupHeadState: proxyGroups.getGroupHeadState,
    onProxies: proxyGroups.onProxies,
    onHeadState: proxyGroups.onHeadState,
  })

  const handleChangeProxy = useCallback(
    (group: IProxyGroupItem, proxy: IProxyItem) => {
      if (isChainMode) {
        chainMode.addProxyToChain(proxy)
        return
      }

      if (group.type !== 'Selector') return
      proxyGroups.handleProxyGroupChange(group, proxy)
    },
    [chainMode, isChainMode, proxyGroups],
  )

  const handleCloseChainMode = useCallback(() => {
    onCloseChainMode?.()
  }, [onCloseChainMode])

  const listView = useProxyGroupsListView({
    activeSelectedGroup: chainMode.activeSelectedGroup,
    displayMode,
    handleChangeProxy,
    handleCheckAll,
    handleGroupLocationByName: proxyGroups.handleGroupLocationByName,
    handleLocation: proxyGroups.handleLocation,
    isChainMode,
    onHeadState: proxyGroups.onHeadState,
    renderList: proxyGroups.renderList,
  })

  return {
    chainConfigData,
    createProxyListProps: listView.createProxyListProps,
    currentGroup: chainMode.currentGroup,
    availableGroups: chainMode.availableGroups,
    activeSelectedGroup: chainMode.activeSelectedGroup,
    displayMode,
    duplicateWarning: chainMode.duplicateWarning,
    handleCloseChainMode,
    handleCloseDuplicateWarning: chainMode.handleCloseDuplicateWarning,
    handleGroupLocationByNameWithScroll:
      listView.handleGroupLocationByNameWithScroll,
    handleGroupMenuClose: chainMode.handleGroupMenuClose,
    handleGroupMenuOpen: chainMode.handleGroupMenuOpen,
    handleGroupSelect: chainMode.handleGroupSelect,
    isChainMode,
    isRuleMode: displayMode === 'rule',
    onProxies: proxyGroups.onProxies,
    proxyChain: chainMode.proxyChain,
    proxyGroupNames: proxyGroups.proxyGroupNames,
    proxiesData: proxyGroups.proxiesData,
    ruleMenuAnchor: chainMode.ruleMenuAnchor,
    scrollToTop: listView.scrollToTop,
    setProxyChain: chainMode.setProxyChain,
    showScrollTop: listView.showScrollTop,
  }
}

export type ProxyGroupsController = ReturnType<typeof useProxyGroupsController>
