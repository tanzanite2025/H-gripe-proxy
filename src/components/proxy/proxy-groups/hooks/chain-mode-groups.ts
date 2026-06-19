import type { IProxyGroupItem } from '@/types/proxy'
export const getAvailableChainModeGroups = (
  groups: IProxyGroupItem[] | undefined,
  isChainMode: boolean,
) => {
  if (!groups) return []

  return isChainMode
    ? groups.filter((group) => group.type === 'Selector')
    : groups
}

export const getDefaultChainModeGroup = (
  availableGroups: IProxyGroupItem[],
  isChainMode: boolean,
) => {
  if (isChainMode && availableGroups.length > 0) {
    return availableGroups[0].name
  }

  return null
}

export const findCurrentChainModeGroup = (
  availableGroups: IProxyGroupItem[],
  activeSelectedGroup: string | null,
) => {
  if (!activeSelectedGroup) return null

  return (
    availableGroups.find((group) => group.name === activeSelectedGroup) ?? null
  )
}
