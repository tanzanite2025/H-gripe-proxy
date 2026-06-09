import {
  categorizeProxyGroup,
  getPreferredProxyGroupName,
} from '@/services/proxy-display'

import { normalizePolicyName } from '../../utils/proxy-selection'
import type { ProxyState } from './shared'
import {
  buildSelectionSnapshot,
  extractProxyNames,
  pickVisibleProxyName,
  type CurrentProxySource,
  type ProxyGroupOption,
} from './shared'

interface BuildVisibleGroupsResult {
  allGroups: ProxyGroupOption[]
  groupMap: Record<string, ProxyGroupOption>
  preferredGroupName: string
  visibleGroups: ProxyGroupOption[]
}

interface BuildProxyStateOptions {
  isGlobalMode: boolean
  prevState: ProxyState
  proxies: CurrentProxySource
  rules: any[]
  savedGroup: string | null
  savedProxy: string | null
}

interface BuildProxyStateResult {
  persistedGroup?: string
  persistedProxy?: string
  state: ProxyState
}

function registerGroup(
  groupsMap: Map<string, ProxyGroupOption>,
  group: any,
  aliasName?: string,
) {
  if (!group && !aliasName) return

  const rawName =
    typeof group?.name === 'string' && group.name.length > 0
      ? group.name
      : aliasName
  const name = normalizePolicyName(rawName)
  if (!name || groupsMap.has(name)) return

  const uniqueAll = Array.from(new Set(extractProxyNames(group?.all)))
  if (uniqueAll.length === 0) return

  const displayKind = categorizeProxyGroup(group)
  if (displayKind === 'auxiliary') return

  groupsMap.set(name, {
    name,
    now: normalizePolicyName(group?.now),
    all: uniqueAll,
    type: group?.type,
    displayKind,
  })
}

function buildVisibleGroups(
  proxies: CurrentProxySource,
  rules: any[],
  isGlobalMode: boolean,
): BuildVisibleGroupsResult {
  const groupsMap = new Map<string, ProxyGroupOption>()
  const preferredGroupName =
    getPreferredProxyGroupName({
      proxies,
      rules,
      isGlobalMode,
    }) || ''

  if (preferredGroupName) {
    const preferredGroup =
      proxies.groups?.find(
        (group: { name?: string }) => group?.name === preferredGroupName,
      ) ||
      (proxies.global?.name === preferredGroupName ? proxies.global : null) ||
      proxies.records?.[preferredGroupName]

    registerGroup(groupsMap, preferredGroup, preferredGroupName)
  }

  ;(proxies.groups || []).forEach((group: any) => registerGroup(groupsMap, group))

  const allGroups = Array.from(groupsMap.values())
  const groupMap = Object.fromEntries(
    allGroups.map((group) => [group.name, group]),
  )
  const manualGroups = allGroups.filter((group) => group.displayKind === 'manual')
  const strategyGroups = allGroups.filter(
    (group) => group.displayKind === 'strategy',
  )

  return {
    allGroups,
    groupMap,
    preferredGroupName,
    visibleGroups: manualGroups.concat(strategyGroups),
  }
}

export function buildProxyState({
  isGlobalMode,
  prevState,
  proxies,
  rules,
  savedGroup,
  savedProxy,
}: BuildProxyStateOptions): BuildProxyStateResult {
  const records = proxies.records || {}
  const { allGroups, groupMap, preferredGroupName, visibleGroups } =
    buildVisibleGroups(proxies, rules, isGlobalMode)

  if (isGlobalMode && proxies.global) {
    const selectedProxy = pickVisibleProxyName(
      extractProxyNames(proxies.global.all),
      records,
      proxies.global.now,
      prevState.selection.proxy,
      savedProxy,
    )
    const snapshot = buildSelectionSnapshot(records, null, selectedProxy)

    return {
      state: {
        proxyData: {
          groups: visibleGroups,
          groupMap,
          records,
        },
        selection: {
          group: 'GLOBAL',
          proxy: selectedProxy,
        },
        displayProxy: snapshot.displayProxy,
        resolvedPath: snapshot.resolvedPath,
      },
    }
  }

  const activeGroup =
    groupMap[prevState.selection.group] ||
    groupMap[savedGroup || ''] ||
    groupMap[preferredGroupName] ||
    visibleGroups[0] ||
    allGroups[0]

  if (!activeGroup) {
    return {
      state: {
        proxyData: {
          groups: visibleGroups,
          groupMap,
          records,
        },
        selection: {
          group: prevState.selection.group || savedGroup || preferredGroupName,
          proxy: '',
        },
        displayProxy: null,
        resolvedPath: [],
      },
    }
  }

  const selectedProxy = pickVisibleProxyName(
    activeGroup.all,
    records,
    activeGroup.now,
    prevState.selection.proxy,
    savedProxy,
  )
  const snapshot = buildSelectionSnapshot(records, activeGroup.name, selectedProxy)

  return {
    persistedGroup: activeGroup.name,
    persistedProxy: selectedProxy || undefined,
    state: {
      proxyData: {
        groups: visibleGroups,
        groupMap,
        records,
      },
      selection: {
        group: activeGroup.name,
        proxy: selectedProxy,
      },
      displayProxy: snapshot.displayProxy,
      resolvedPath: snapshot.resolvedPath,
    },
  }
}
