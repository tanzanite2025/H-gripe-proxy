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
import { useRuntimeConfig } from '@/hooks/data/use-clash'
import { useVerge } from '@/hooks/system'
import {
  useAppRefreshers,
  useClashConfigData,
  useCoreDataStatus,
  useProxiesData,
  useRulesData,
} from '@/providers/app-data-context'
import { DEFAULT_CLASH_MODE, resolveClashMode } from '@/services/clash-mode'
import delayManager from '@/services/delay'

import { ProxyInfoDisplay } from './components/proxy-info-display'
import { useCurrentProxyData } from './hooks/use-current-proxy-data'
import { useProxyDelayCheck } from './hooks/use-proxy-delay-check'
import { convertDelayColor, getSignalIcon } from './utils/proxy-helpers'

const getProxyOptionPrefix = (kind: string) => {
  return kind === 'strategy' ? '[Strategy]' : ''
}

export const CurrentProxyCard = () => {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { proxies } = useProxiesData()
  const { clashConfig } = useClashConfigData()
  const { data: runtimeConfig } = useRuntimeConfig()
  const { rules } = useRulesData()
  const { refreshProxy } = useAppRefreshers()
  const { isCoreDataPending } = useCoreDataStatus()
  const { verge } = useVerge()
  const { current: currentProfile } = useProfiles()

  const defaultLatencyTimeout = verge?.default_latency_timeout || 10000
  const currentProfileId = currentProfile?.uid || null

  const mode =
    resolveClashMode(clashConfig?.mode, runtimeConfig?.mode) ??
    DEFAULT_CLASH_MODE
  const proxyChainMode = mode === 'direct' ? DEFAULT_CLASH_MODE : mode
  const isGlobalMode = proxyChainMode === 'global'

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
    defaultLatencyTimeout,
    refreshProxy,
  })

  const { handleCheckAllDelay } = useProxyDelayCheck({
    currentGroup: state.selection.group,
    defaultLatencyTimeout,
    proxyRecords: state.proxyData.records,
    refreshProxy,
    onDelayCheckComplete: () => {
      if (sortType === 1) {
        triggerDelaySortRefresh()
      }
    },
  })

  const goToProxies = useCallback(() => {
    navigate('/proxies')
  }, [navigate])

  const currentProxy = useMemo(() => state.displayProxy, [state.displayProxy])
  const currentPathText = useMemo(
    () => (state.resolvedPath.length > 0 ? state.resolvedPath.join(' -> ') : ''),
    [state.resolvedPath],
  )

  const handleGroupSelectChange = useCallback(
    (event: { target: { value: string } }) => {
      handleGroupChange(event.target.value)
    },
    [handleGroupChange],
  )

  const currentDelay =
    currentProxy && state.selection.group
      ? delayManager.getDelayFix(currentProxy, state.selection.group)
      : -1

  const signalInfo =
    currentProxy && state.selection.group
      ? getSignalIcon(currentDelay)
      : { icon: <SignalNone />, text: '鏈垵濮嬪寲', color: 'text.secondary' }

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
              : '鏃犱唬鐞嗚妭鐐?'
          }
        >
          <div style={{ color: signalInfo.color }}>
            {currentProxy ? (
              signalInfo.icon
            ) : (
              <SignalNone className="h-5 w-5 text-gray-400" />
            )}
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
        <div className="px-3 pt-1.5 pb-3">
          {currentPathText && (
            <div className="mb-3 rounded-2xl border border-sky-500/20 bg-sky-500/5 px-3 py-2 text-xs text-gray-300">
              <span className="mr-2 font-semibold text-sky-400">褰撳墠閾捐矾</span>
              <span className="break-all">{currentPathText}</span>
            </div>
          )}

          <div className="flex items-center gap-4">
            <div className="h-9 min-w-0 flex-1">
              <ProxyInfoDisplay
                proxy={currentProxy}
                delay={currentDelay}
                isGlobalMode={isGlobalMode}
              />
            </div>

            {currentProxy && (
              <div className="min-w-0 flex-1">
                <Select
                  value={state.selection.group}
                  onChange={handleGroupSelectChange}
                  disabled={isGlobalMode}
                  size="small"
                  className="h-[38px] rounded-2xl border border-solid border-gray-200 bg-gray-50/20 dark:border-gray-700 dark:bg-gray-800/20 [&_select]:border-0 [&_select]:bg-transparent"
                >
                  {state.proxyData.groups.map((group) => (
                    <MenuItem key={group.name} value={group.name}>
                      {group.name}
                    </MenuItem>
                  ))}
                </Select>
              </div>
            )}

            {currentProxy && (
              <div className="min-w-0 flex-1">
                <Select
                  value={state.selection.proxy}
                  onChange={handleProxyChange}
                  size="small"
                  renderValue={(selected: SelectPrimitiveValue) => (
                    <div className="truncate">{String(selected)}</div>
                  )}
                  className="h-[38px] rounded-2xl border border-solid border-gray-200 bg-gray-50/20 dark:border-gray-700 dark:bg-gray-800/20 [&_select]:border-0 [&_select]:bg-transparent"
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
                  {proxyOptions.map((proxy) => {
                    const delayValue =
                      state.proxyData.records[proxy.name] && state.selection.group
                        ? delayManager.getDelayFix(
                            state.proxyData.records[proxy.name],
                            state.selection.group,
                          )
                        : -1

                    const prefix = getProxyOptionPrefix(proxy.kind)
                    return (
                      <MenuItem
                        key={proxy.name}
                        value={proxy.name}
                        className="flex w-full items-center justify-between pr-1"
                      >
                        <div className="mr-1 flex-1 truncate">
                          {prefix ? `${prefix} ` : ''}
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
        </div>
      )}
    </EnhancedCard>
  )
}
