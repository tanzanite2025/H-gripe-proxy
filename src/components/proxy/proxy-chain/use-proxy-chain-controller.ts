import { useCallback, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { useAppRefreshers, useProxiesData } from '@/providers/app-data-context'

import { isProxyChainConnected } from '../proxy-chain-types'
import { buildProxyChainViewModel } from './build-proxy-chain-view-model'
import { buildProxyChainCopy } from './proxy-chain-copy'
import { useProxyChainConnection } from './use-proxy-chain-connection'
import { useProxyChainConfigLoader } from './use-proxy-chain-config-loader'
import { useProxyChainDelayUpdater } from './use-proxy-chain-delay-updater'
import { useProxyChainDnd } from './use-proxy-chain-dnd'
import { useProxyChainLengthDirtyMarker } from './use-proxy-chain-length-dirty-marker'
import { useResidentialPoolState } from './use-residential-pool-state'
import type { ProxyChainProps } from './types'

export const useProxyChainController = ({
  proxyChain,
  onUpdateChain,
  chainConfigData,
  onMarkUnsavedChanges,
  selectedGroup,
  bare = false,
  onClose,
}: ProxyChainProps) => {
  const { t } = useTranslation()
  const { proxies } = useProxiesData()
  const { refreshProxy } = useAppRefreshers()
  const [helpDialogOpen, setHelpDialogOpen] = useState(false)

  const markUnsavedChanges = useCallback(() => {
    onMarkUnsavedChanges?.()
  }, [onMarkUnsavedChanges])

  const {
    residentialPool,
    enabledResidentialProxies,
    localResidentialPool,
    residentialConfigOpen,
    setLocalResidentialPool,
    setResidentialConfigOpen,
    addResidentialExit,
    openResidentialConfig,
    saveResidentialPool,
  } = useResidentialPoolState(proxyChain, onUpdateChain, markUnsavedChanges)

  useProxyChainLengthDirtyMarker(proxyChain.length, markUnsavedChanges)
  useProxyChainConfigLoader(chainConfigData, onUpdateChain)
  useProxyChainDelayUpdater(proxies?.records, proxyChain, onUpdateChain)

  const copy = useMemo(
    () => buildProxyChainCopy(t, proxyChain.length),
    [t, proxyChain.length],
  )

  const isConnected = useMemo(
    () => isProxyChainConnected(proxies, proxyChain, selectedGroup),
    [proxies, proxyChain, selectedGroup],
  )

  const { sensors, handleDragEnd, handleRemoveProxy } = useProxyChainDnd({
    proxyChain,
    onUpdateChain,
    onDirty: markUnsavedChanges,
  })

  const { isConnecting, handleClearChain, handleConnect } =
    useProxyChainConnection({
      isConnected,
      proxyChain,
      selectedGroup,
      onUpdateChain,
      refreshProxy,
      copy,
    })

  return buildProxyChainViewModel({
    bare,
    copy,
    enabledResidentialProxies,
    helpDialogOpen,
    isConnected,
    isConnecting,
    localResidentialPool,
    onAddResidentialExit: addResidentialExit,
    onChangeResidentialPool: setLocalResidentialPool,
    onClearChain: handleClearChain,
    onClose,
    onCloseHelpDialog: () => setHelpDialogOpen(false),
    onCloseResidentialConfig: () => setResidentialConfigOpen(false),
    onConnect: handleConnect,
    onDragEnd: handleDragEnd,
    onOpenHelp: () => setHelpDialogOpen(true),
    onOpenResidentialConfig: openResidentialConfig,
    onRemove: handleRemoveProxy,
    onSaveResidentialPool: saveResidentialPool,
    proxyChain,
    residentialConfigOpen,
    residentialPool,
    selectedGroup,
    sensors,
  })
}
