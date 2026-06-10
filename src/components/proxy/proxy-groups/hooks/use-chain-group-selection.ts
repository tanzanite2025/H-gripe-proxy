import { useCallback, useMemo, useState } from 'react'

import {
  findCurrentChainModeGroup,
  getAvailableChainModeGroups,
  getDefaultChainModeGroup,
} from './chain-mode-groups'

interface UseChainGroupSelectionOptions {
  groups: IProxyGroupItem[] | undefined
  isChainMode: boolean
  onSelectGroup?: () => void
}

export function useChainGroupSelection({
  groups,
  isChainMode,
  onSelectGroup,
}: UseChainGroupSelectionOptions) {
  const [selectedGroup, setSelectedGroup] = useState<string | null>(null)
  const [ruleMenuAnchor, setRuleMenuAnchor] = useState<null | HTMLElement>(null)

  const availableGroups = useMemo(
    () => getAvailableChainModeGroups(groups, isChainMode),
    [groups, isChainMode],
  )

  const defaultRuleGroup = useMemo(
    () => getDefaultChainModeGroup(availableGroups, isChainMode),
    [availableGroups, isChainMode],
  )

  const activeSelectedGroup = useMemo(
    () => selectedGroup ?? defaultRuleGroup,
    [defaultRuleGroup, selectedGroup],
  )

  const currentGroup = useMemo(
    () => findCurrentChainModeGroup(availableGroups, activeSelectedGroup),
    [activeSelectedGroup, availableGroups],
  )

  const handleGroupMenuOpen = useCallback(
    (event: React.MouseEvent<HTMLElement>) => {
      setRuleMenuAnchor(event.currentTarget)
    },
    [],
  )

  const handleGroupMenuClose = useCallback(() => {
    setRuleMenuAnchor(null)
  }, [])

  const handleGroupSelect = useCallback(
    (groupName: string) => {
      setSelectedGroup(groupName)
      handleGroupMenuClose()
      onSelectGroup?.()
    },
    [handleGroupMenuClose, onSelectGroup],
  )

  return {
    activeSelectedGroup,
    availableGroups,
    currentGroup,
    handleGroupMenuClose,
    handleGroupMenuOpen,
    handleGroupSelect,
    ruleMenuAnchor,
  }
}
