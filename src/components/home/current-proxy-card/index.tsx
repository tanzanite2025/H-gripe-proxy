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
import { Chip } from '@/components/tailwind/Chip'
import { IconButton } from '@/components/tailwind/IconButton'
import {
  MenuItem,
  Select,
  type SelectPrimitiveValue,
} from '@/components/tailwind/Select'
import { Tooltip } from '@/components/tailwind/Tooltip'
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
import { useCurrentProxyData } from './hooks/use-current-proxy-data'
import { useProxyDelayCheck } from './hooks/use-proxy-delay-check'
import { convertDelayColor, getSignalIcon } from './utils/proxy-helpers'

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
    (event: { target: { value: string } }) => {
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
      noContentPadding
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
        <div className="py-2" />
      ) : (
        <div className="flex items-center gap-4 px-3 pt-1.5 pb-3">
          {/* 1. 节点信息 */}
          <div className="flex-1 min-w-0 h-9">
            <ProxyInfoDisplay
              proxy={currentProxy}
              delay={currentDelay}
              isGlobalMode={isGlobalMode}
              isDirectMode={isDirectMode}
            />
          </div>

          {/* 2. 代理组 */}
          {currentProxy && (
            <div className="flex-1 min-w-0">
              <Select
                value={state.selection.group}
                onChange={handleGroupSelectChange}
                disabled={isGlobalMode || isDirectMode}
                size="small"
                className="h-[38px] rounded-2xl border border-dashed border-gray-200 bg-gray-50/20 dark:border-gray-700 dark:bg-gray-800/20 [&_select]:border-0 [&_select]:bg-transparent"
              >
                {state.proxyData.groups.map((group) => (
                  <MenuItem key={group.name} value={group.name}>
                    {group.name}
                  </MenuItem>
                ))}
              </Select>
            </div>
          )}

          {/* 3. 代理节点 */}
          {currentProxy && (
            <div className="flex-1 min-w-0">
              <Select
                value={state.selection.proxy}
                onChange={handleProxyChange}
                disabled={isDirectMode}
                size="small"
                renderValue={(selected: SelectPrimitiveValue) => <div className="truncate">{String(selected)}</div>}
                className="h-[38px] rounded-2xl border border-dashed border-gray-200 bg-gray-50/20 dark:border-gray-700 dark:bg-gray-800/20 [&_select]:border-0 [&_select]:bg-transparent"
                MenuProps={{
                  slotProps: {
                    paper: {
                      style: {
                        maxHeight: 500,
                      },
                    },
                  },
                }}
              >
                {isDirectMode
                  ? null
                  : proxyOptions.map((proxy) => {
                      const delayValue =
                        state.proxyData.records[proxy.name] && state.selection.group
                          ? delayManager.getDelayFix(
                              state.proxyData.records[proxy.name],
                              state.selection.group,
                            )
                          : -1
                      return (
                        <MenuItem
                          key={proxy.name}
                          value={proxy.name}
                          className="flex w-full items-center justify-between pr-1"
                        >
                          <div className="mr-1 flex-1 truncate">
                            {proxy.name}
                          </div>
                          <Chip
                            size="small"
                            label={delayManager.formatDelay(delayValue)}
                            color={convertDelayColor(delayValue)}
                            className="h-[22px] min-w-[60px] shrink-0"
                          />
                        </MenuItem>
                      )
                    })}
              </Select>
            </div>
          )}
        </div>
      )}
    </EnhancedCard>
  )
}
