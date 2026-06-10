import { useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'

import { useProfiles } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import {
  useAppRefreshers,
  useCoreDataStatus,
  useProxiesData,
} from '@/providers/app-data-context'
import { resolveVergeDelayTimeout } from '@/services/delay-config'
import delayManager from '@/services/delay'

import { useCurrentProxyData } from './use-current-proxy-data'
import { useProxyDelayCheck } from './use-proxy-delay-check'
import { getDelaySignalVisual } from '../utils/delay-visuals'

const SORT_TOOLTIP_KEYS = {
  0: 'proxies.page.tooltips.sortDefault',
  1: 'proxies.page.tooltips.sortDelay',
  2: 'proxies.page.tooltips.sortName',
} as const

export function useCurrentProxyCardController() {
  const { t } = useTranslation()
  const navigate = useNavigate()
  const { proxies } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const { isCoreDataPending } = useCoreDataStatus()
  const { verge } = useVerge()
  const { current: currentProfile } = useProfiles()

  const defaultLatencyTimeout = resolveVergeDelayTimeout(verge)
  const currentProfileId = currentProfile?.uid || null

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
    currentProfileId,
    defaultLatencyTimeout,
    refreshProxy,
  })

  const { handleCheckAllDelay } = useProxyDelayCheck({
    currentGroup: state.selection.group,
    defaultLatencyTimeout,
    groupMap: state.proxyData.groupMap,
    proxyRecords: state.proxyData.records,
    refreshProxy,
    onDelayCheckComplete: () => {
      if (sortType === 1) {
        triggerDelaySortRefresh()
      }
    },
  })

  const onOpenProxies = useCallback(() => {
    navigate('/proxies')
  }, [navigate])

  const currentProxy = state.displayProxy
  const currentPathText =
    state.resolvedPath.length > 0 ? state.resolvedPath.join(' -> ') : ''

  const onGroupSelectChange = useCallback(
    (event: { target: { value: string } }) => {
      handleGroupChange(event.target.value)
    },
    [handleGroupChange],
  )

  const currentDelay =
    currentProxy && state.selection.group
      ? delayManager.getDelayFix(currentProxy, state.selection.group)
      : -1

  const signalVisual =
    currentProxy && state.selection.group
      ? getDelaySignalVisual(currentDelay, defaultLatencyTimeout)
      : null

  const refreshDelayLabel = t('home.components.currentProxy.actions.refreshDelay')
  const noActiveNodeLabel = t('home.components.currentProxy.labels.noActiveNode')

  return {
    currentDelay,
    currentPathText,
    currentProxy,
    defaultLatencyTimeout,
    isCoreDataPending,
    noActiveNodeLabel,
    onCheckAllDelay: handleCheckAllDelay,
    onGroupSelectChange,
    onOpenProxies,
    onProxyChange: handleProxyChange,
    onSortTypeChange: handleSortTypeChange,
    pageTitle: t('home.components.currentProxy.title'),
    proxiesLabel: t('layout.components.navigation.tabs.proxies'),
    proxyOptions,
    refreshDelayLabel,
    records: state.proxyData.records,
    selectedGroup: state.selection.group,
    selectedProxy: state.selection.proxy,
    signalVisual,
    sortTooltip: t(SORT_TOOLTIP_KEYS[sortType]),
    sortType,
    groups: state.proxyData.groups,
  }
}
