import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'

import { useProxiesData } from '@/providers/app-data-context'

import { useChainGroupSelection } from './use-chain-group-selection'
import { usePersistedProxyChain } from './use-persisted-proxy-chain'

interface UseChainModeOptions {
  isChainMode: boolean
}

export function useChainMode(options: UseChainModeOptions) {
  const { isChainMode } = options
  const { t } = useTranslation()
  const { proxies: proxiesData } = useProxiesData()

  const chainState = usePersistedProxyChain({
    duplicateWarningMessage: t('proxies.page.chain.duplicateNode'),
  })

  const handleSelectRuleGroup = useCallback(() => {
    if (isChainMode) {
      chainState.resetProxyChain()
    }
  }, [chainState, isChainMode])

  const groupSelection = useChainGroupSelection({
    groups: proxiesData?.groups,
    isChainMode,
    onSelectGroup: handleSelectRuleGroup,
  })

  return {
    activeSelectedGroup: groupSelection.activeSelectedGroup,
    addProxyToChain: chainState.addProxyToChain,
    availableGroups: groupSelection.availableGroups,
    currentGroup: groupSelection.currentGroup,
    duplicateWarning: chainState.duplicateWarning,
    handleCloseDuplicateWarning: chainState.handleCloseDuplicateWarning,
    handleGroupMenuClose: groupSelection.handleGroupMenuClose,
    handleGroupMenuOpen: groupSelection.handleGroupMenuOpen,
    handleGroupSelect: groupSelection.handleGroupSelect,
    proxyChain: chainState.proxyChain,
    ruleMenuAnchor: groupSelection.ruleMenuAnchor,
    setProxyChain: chainState.setProxyChain,
  }
}
