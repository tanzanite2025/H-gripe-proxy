import {
  Clock as AccessTimeRounded,
  ChevronRight,
  Activity as NetworkCheckRounded,
  WifiOff as SignalNone,
  SortAsc as SortByAlphaRounded,
  ArrowUpDown as SortRounded,
} from 'lucide-react'
import React, { useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import { EnhancedCard } from '@/components/home/enhanced-card'
import { Button } from '@/components/tailwind/Button'
import { IconButton } from '@/components/tailwind/IconButton'
import { Tooltip } from '@/components/tailwind/Tooltip'
import type { SelectChangeEvent } from '@/components/tailwind/Select'
import { useProfiles } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import {
  useAppRefreshers,
  useClashConfigData,
  useCoreDataStatus,
  useProxiesData,
  useRulesData,
} from '@/providers/app-data-context'
import delayManager from '@/services/delay'

import { ProxyInfoDisplay } from './components/proxy-info-display'
import { ProxySelectors } from './components/proxy-selectors'
import { useCurrentProxyData } from './hooks/use-current-proxy-data'
import { useProxyDelayCheck } from './hooks/use-proxy-delay-check'
import { getSignalIcon } from './utils/proxy-helpers'

/**
 * 当前代理卡片组件
 * 显示当前选中的代理信息，支持切换代理组和代理节点
 */
export const CurrentProxyCard = () => {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { proxies } = useProxiesData()
  const { clashConfig } = useClashConfigData()
  const { rules } = useRulesData()
  const { refreshProxy } = useAppRefreshers()
  const { isCoreDataPending } = useCoreDataStatus()
  const { verge } = useVerge()
  const { current: currentProfile } = useProfiles()

  // 配置参数
  const autoDelayEnabled = verge?.enable_auto_delay_detection ?? false
  const defaultLatencyTimeout = verge?.default_latency_timeout || 10000
  const autoDelayIntervalMs = useMemo(() => {
    const rawInterval = verge?.auto_delay_detection_interval_minutes
    const intervalMinutes =
      typeof rawInterval === 'number' && rawInterval > 0 ? rawInterval : 5
    return Math.max(1, Math.round(intervalMinutes)) * 60 * 1000
  }, [verge?.auto_delay_detection_interval_minutes])

  const currentProfileId = currentProfile?.uid || null

  // 判断模式
  const mode = clashConfig?.mode?.toLowerCase() || 'rule'
  const isGlobalMode = mode === 'global'
  const isDirectMode = mode === 'direct'

  // 数据管理
  const {
    state,
    sortType,
    proxyOptions,
    handleGroupChange,
    handleProxyChange,
    handleSortTypeChange,
    triggerDelaySortRefresh,
  } = useCurrentProxyData({
    proxies,
    rules,
    clashConfig,
    currentProfileId,
    isGlobalMode,
    isDirectMode,
    defaultLatencyTimeout,
    refreshProxy,
  })

  // 延迟检测
  const { handleCheckAllDelay } = useProxyDelayCheck({
    currentGroup: state.selection.group,
    currentProxy: state.selection.proxy,
    currentProxyRecord: state.displayProxy,
    isDirectMode,
    autoDelayEnabled,
    autoDelayIntervalMs,
    defaultLatencyTimeout,
    proxyRecords: state.proxyData.records,
    refreshProxy,
    onDelayCheckComplete: () => {
      if (sortType === 1) {
        triggerDelaySortRefresh()
      }
    },
  })

  // 导航到代理页面
  const goToProxies = useCallback(() => {
    navigate('/proxies')
  }, [navigate])

  // 获取要显示的代理节点
  const currentProxy = useMemo(() => {
    return state.displayProxy
  }, [state.displayProxy])

  const handleGroupSelectChange = useCallback(
    (event: SelectChangeEvent<string>) => {
      handleGroupChange(event.target.value)
    },
    [handleGroupChange],
  )

  // 获取当前节点的延迟
  const currentDelay =
    currentProxy && state.selection.group
      ? delayManager.getDelayFix(currentProxy, state.selection.group)
      : -1

  // 信号图标
  const signalInfo =
    currentProxy && state.selection.group
      ? getSignalIcon(currentDelay)
      : { icon: <SignalNone />, text: '未初始化', color: 'text.secondary' }

  // 获取排序图标
  const getSortIcon = (): React.ReactElement => {
    switch (sortType) {
      case 1:
        return <AccessTimeRounded className="h-4 w-4" />
      case 2:
        return <SortByAlphaRounded className="h-4 w-4" />
      default:
        return <SortRounded className="h-4 w-4" />
    }
  }

  // 获取排序提示文本
  const getSortTooltip = (): string => {
    switch (sortType) {
      case 0:
        return t('proxies.page.tooltips.sortDefault')
      case 1:
        return t('proxies.page.tooltips.sortDelay')
      case 2:
        return t('proxies.page.tooltips.sortName')
      default:
        return ''
    }
  }

  return (
    <EnhancedCard
      title={t('home.components.currentProxy.title')}
      icon={
        <Tooltip
          title={
            currentProxy
              ? `${signalInfo.text}: ${delayManager.formatDelay(currentDelay)}`
              : '无代理节点'
          }
        >
          <div style={{ color: signalInfo.color }}>
            {currentProxy ? signalInfo.icon : <SignalNone className="h-5 w-5 text-gray-400" />}
          </div>
        </Tooltip>
      }
      iconColor={currentProxy ? 'primary' : undefined}
      action={
        <div className="flex items-center gap-1">
          <Tooltip
            title={t('home.components.currentProxy.actions.refreshDelay')}
          >
            <span>
              <IconButton
                size="small"
                color="inherit"
                onClick={() => handleCheckAllDelay(isGlobalMode)}
                disabled={isDirectMode}
              >
                <NetworkCheckRounded className="h-5 w-5" />
              </IconButton>
            </span>
          </Tooltip>
          <Tooltip title={getSortTooltip()}>
            <IconButton
              size="small"
              color="inherit"
              onClick={handleSortTypeChange}
            >
              {getSortIcon()}
            </IconButton>
          </Tooltip>
          <Button
            variant="outlined"
            size="small"
            onClick={goToProxies}
            className="rounded-xl"
            endIcon={<ChevronRight className="h-4 w-4" />}
          >
            {t('layout.components.navigation.tabs.proxies')}
          </Button>
        </div>
      }
    >
      {isCoreDataPending ? (
        <div className="py-4" />
      ) : (
        <div>
          {/* 代理节点信息显示 */}
          <ProxyInfoDisplay
            proxy={currentProxy}
            delay={currentDelay}
            isGlobalMode={isGlobalMode}
            isDirectMode={isDirectMode}
          />

          {/* 代理选择器 */}
          {currentProxy && (
            <ProxySelectors
              state={state}
              proxyOptions={proxyOptions}
              isGlobalMode={isGlobalMode}
              isDirectMode={isDirectMode}
              onGroupChange={handleGroupSelectChange}
              onProxyChange={handleProxyChange}
            />
          )}
        </div>
      )}
    </EnhancedCard>
  )
}
