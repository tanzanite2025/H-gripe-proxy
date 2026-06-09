import { useCallback, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useLocation } from 'react-router'

import { Alert, Snackbar } from '@/components/tailwind'

import { ScrollTopButton } from '../../layout/scroll-top-button'
import { ProxyChainDrawer } from '../proxy-chain-drawer'
import {
  DEFAULT_HOVER_DELAY,
  ProxyGroupNavigator,
} from '../proxy-group-navigator'
import { StrategyPoolEditorDialog } from '../strategy-pool-editor-dialog'

import { ChainRuleHeader } from './components/chain-rule-header'
import { GroupSelectMenu } from './components/group-select-menu'
import { ProxyVirtualList } from './components/proxy-virtual-list'
import { useChainMode } from './hooks/use-chain-mode'
import { useDelayCheck } from './hooks/use-delay-check'
import { useProxyGroups } from './hooks/use-proxy-groups'
import {
  useRestoreScrollPosition,
  useScrollListener,
  useScrollPosition,
} from './hooks/use-scroll-position'
import { useVirtualScroll } from './hooks/use-virtual-scroll'

interface Props {
  mode: string
  isChainMode?: boolean
  chainConfigData?: string | null
  onCloseChainMode?: () => void
}

/**
 * 代理组主组件 - 重构后的版本
 * 
 * 职责：
 * 1. 协调各个子模块
 * 2. 处理模式切换（normal/chain）
 * 3. 管理布局和渲染
 */
export const ProxyGroups = (props: Props) => {
  const { mode, isChainMode = false, chainConfigData, onCloseChainMode } = props
  const displayMode = mode === 'direct' ? 'rule' : mode
  const { t } = useTranslation()
  const { pathname } = useLocation()
  const [editingStrategyGroup, setEditingStrategyGroup] =
    useState<IProxyGroupItem | null>(null)

  const parentRef = useRef<HTMLDivElement>(null)
  const scrollTopRef = useRef(0)
  const restoredScrollKeyRef = useRef<string | null>(null)

  // 链式代理模式管理
  const {
    proxyChain,
    ruleMenuAnchor,
    duplicateWarning,
    availableGroups,
    activeSelectedGroup,
    currentGroup,
    setProxyChain,
    handleGroupMenuOpen,
    handleGroupMenuClose,
    handleGroupSelect,
    addProxyToChain,
    handleCloseDuplicateWarning,
  } = useChainMode({
    isChainMode,
    mode: displayMode,
  })

  // 代理组数据和业务逻辑
  const {
    proxiesData,
    renderList,
    timeout,
    proxyGroupNames,
    onProxies,
    onHeadState,
    getGroupHeadState,
    handleProxyGroupChange,
    handleLocation,
    handleGroupLocationByName,
  } = useProxyGroups({
    mode: displayMode,
    isChainMode,
  })

  // 虚拟滚动
  const { virtualItems, activeStickyIndex, scrollToIndex, totalSize, measureElement } =
    useVirtualScroll({
      renderList,
      parentRef,
    })

  // 滚动位置管理
  const {
    showScrollTop,
    scrollPositionKey,
    handleScroll,
    restoreScrollPosition,
    scrollToTop: scrollToTopFn,
    saveScrollPosition,
  } = useScrollPosition({
    mode: displayMode,
    isChainMode,
    activeSelectedGroup,
    renderListLength: renderList.length,
  })

  // 恢复滚动位置
  useRestoreScrollPosition(parentRef, restoreScrollPosition, pathname)

  // 监听滚动事件
  useScrollListener(
    parentRef,
    handleScroll,
    saveScrollPosition,
    scrollPositionKey,
    scrollTopRef,
    restoredScrollKeyRef,
  )

  // 延迟测试
  const { handleCheckAll } = useDelayCheck({
    renderList,
    timeout,
    getGroupHeadState,
    onProxies,
    onHeadState,
  })

  // 滚动到顶部
  const scrollToTop = useCallback(() => {
    scrollToTopFn(parentRef.current)
  }, [scrollToTopFn])

  // 处理代理变更
  const handleChangeProxy = useCallback(
    (group: IProxyGroupItem, proxy: IProxyItem) => {
      if (isChainMode) {
        addProxyToChain(proxy)
        return
      }

      if (group.type !== 'Selector') return

      handleProxyGroupChange(group, proxy)
    },
    [addProxyToChain, handleProxyGroupChange, isChainMode],
  )

  // 定位到代理节点（包装 scrollToIndex）
  const handleLocationWithScroll = useCallback(
    (group: IProxyGroupItem) => {
      handleLocation(group, scrollToIndex)
    },
    [handleLocation, scrollToIndex],
  )

  // 定位到代理组（包装 scrollToIndex）
  const handleGroupLocationByNameWithScroll = useCallback(
    (groupName: string) => {
      handleGroupLocationByName(groupName, scrollToIndex)
    },
    [handleGroupLocationByName, scrollToIndex],
  )

  const handleConfigureStrategyGroup = useCallback((group: IProxyGroupItem) => {
    setEditingStrategyGroup(group)
  }, [])

  // 渲染代理列表
  const renderProxyList = useCallback(
    (height: string) => {
      return (
        <ProxyVirtualList
          parentRef={parentRef}
          height={height}
          totalSize={totalSize}
          virtualItems={virtualItems}
          renderList={renderList}
          activeStickyIndex={activeStickyIndex}
          indent={displayMode === 'rule' || displayMode === 'script'}
          measureElement={measureElement}
          onLocation={handleLocationWithScroll}
          onCheckAll={handleCheckAll}
          onHeadState={onHeadState}
          onChangeProxy={handleChangeProxy}
          onConfigureStrategyGroup={handleConfigureStrategyGroup}
        />
      )
    },
    [
      activeStickyIndex,
      handleChangeProxy,
      handleCheckAll,
      handleConfigureStrategyGroup,
      handleLocationWithScroll,
      measureElement,
      displayMode,
      onHeadState,
      renderList,
      totalSize,
      virtualItems,
    ],
  )

  // 链式代理模式
  if (isChainMode) {
    const proxyGroups = proxiesData?.groups || []
    const showRuleHeader = displayMode === 'rule' && proxyGroups.length > 0

    return (
      <>
        <div className="h-full">
          {showRuleHeader && (
            <ChainRuleHeader
              title={t('proxies.page.rules.title')}
              selectLabel={t('proxies.page.rules.select')}
              currentGroup={currentGroup}
              canSelectGroup={availableGroups.length > 0}
              onMenuOpen={handleGroupMenuOpen}
            />
          )}

          {renderProxyList(
            showRuleHeader ? 'calc(100% - 80px)' : 'calc(100% - 14px)',
          )}
          <ScrollTopButton show={showScrollTop} onClick={scrollToTop} />
        </div>

        <Snackbar
          open={duplicateWarning.open}
          autoHideDuration={3000}
          onClose={handleCloseDuplicateWarning}
          anchorOrigin={{ vertical: 'top', horizontal: 'center' }}
        >
          <Alert
            onClose={handleCloseDuplicateWarning}
            severity="warning"
            variant="filled"
          >
            {duplicateWarning.message}
          </Alert>
        </Snackbar>

        <GroupSelectMenu
          anchorEl={ruleMenuAnchor}
          groups={availableGroups}
          selectedGroup={activeSelectedGroup}
          emptyText="暂无可用代理组"
          onClose={handleGroupMenuClose}
          onSelect={handleGroupSelect}
        />

        <ProxyChainDrawer
          open={isChainMode}
          proxyChain={proxyChain}
          onUpdateChain={setProxyChain}
          chainConfigData={chainConfigData}
          mode={displayMode}
          selectedGroup={activeSelectedGroup}
          onClose={onCloseChainMode ?? (() => {})}
        />
        <StrategyPoolEditorDialog
          open={Boolean(editingStrategyGroup)}
          group={editingStrategyGroup}
          onClose={() => setEditingStrategyGroup(null)}
          onSaved={onProxies}
        />
      </>
    )
  }

  // 普通模式
  return (
    <div
      style={{ position: 'relative', height: '100%', willChange: 'transform' }}
    >
      {/* 代理组导航栏 */}
      {displayMode === 'rule' && (
        <ProxyGroupNavigator
          proxyGroupNames={proxyGroupNames}
          onGroupLocation={handleGroupLocationByNameWithScroll}
          enableHoverJump={true}
          hoverDelay={DEFAULT_HOVER_DELAY}
        />
      )}

      {renderProxyList('calc(100% - 14px)')}
      <ScrollTopButton show={showScrollTop} onClick={scrollToTop} />
      <StrategyPoolEditorDialog
        open={Boolean(editingStrategyGroup)}
        group={editingStrategyGroup}
        onClose={() => setEditingStrategyGroup(null)}
        onSaved={onProxies}
      />
    </div>
  )
}
