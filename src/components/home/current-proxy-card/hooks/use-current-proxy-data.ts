import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { SelectChangeEvent } from '@/components/tailwind/Select'
import { useProxySelection } from '@/hooks/data'
import delayManager from '@/services/delay'
import {
  buildProxyDisplayOptionsFromNames,
  categorizeProxyGroup,
  getPreferredProxyGroupName,
  isAuxiliarySelectionName,
  pickPreferredProxyNameFromNames,
  resolveProxyPath,
} from '@/services/proxy-display'

import { categorizeDelay, normalizePolicyName } from '../utils/proxy-helpers'

import {
  STORAGE_KEY_GROUP,
  STORAGE_KEY_PROXY,
  useProxyStorage,
} from './use-proxy-storage'

type ProxyGroupOption = {
  name: string
  now: string
  all: string[]
  type?: string
  displayKind: 'manual' | 'strategy'
}

interface ProxyOption {
  name: string
  kind: 'manual' | 'strategy'
}

export type ProxySortType = 0 | 1 | 2

export type ProxyState = {
  proxyData: {
    groups: ProxyGroupOption[]
    groupMap: Record<string, ProxyGroupOption>
    records: Record<string, any>
  }
  selection: {
    group: string
    proxy: string
  }
  displayProxy: any
  resolvedPath: string[]
}

interface UseCurrentProxyDataProps {
  proxies: any
  rules: any[]
  clashConfig: any
  currentProfileId: string | null
  isGlobalMode: boolean
  defaultLatencyTimeout: number
  refreshProxy: () => void
}

const KIND_WEIGHT: Record<ProxyOption['kind'], number> = {
  manual: 0,
  strategy: 1,
}

const resolveLeafProxy = (records: Record<string, any>, name: string) => {
  const resolved = resolveProxyPath(records, name)
  const leafName = resolved.leafName || name
  return {
    displayProxy: records?.[leafName] || records?.[name] || null,
    resolvedPath: resolved.path,
  }
}

const pickVisibleProxyName = (
  names: string[],
  records: Record<string, any>,
  ...candidates: Array<string | null | undefined>
) => {
  for (const candidate of candidates) {
    const normalizedCandidate = normalizePolicyName(candidate)
    if (!normalizedCandidate) continue

    const pickedCandidate = pickPreferredProxyNameFromNames({
      names,
      records,
      candidateName: normalizedCandidate,
    })

    if (pickedCandidate === normalizedCandidate) {
      return pickedCandidate
    }
  }

  return pickPreferredProxyNameFromNames({
    names,
    records,
  })
}

export const useCurrentProxyData = ({
  proxies,
  rules,
  clashConfig: _clashConfig,
  currentProfileId,
  isGlobalMode,
  defaultLatencyTimeout,
  refreshProxy,
}: UseCurrentProxyDataProps) => {
  const { readProfileScopedItem, writeProfileScopedItem } =
    useProxyStorage(currentProfileId)

  const { changeProxy, handleSelectChange } = useProxySelection({
    onSuccess: () => {
      refreshProxy()
    },
    onError: (error) => {
      console.error('Proxy switch failed', error)
      refreshProxy()
    },
  })

  const [sortType, setSortType] = useState<ProxySortType>(() => {
    const savedSortType = localStorage.getItem('clash-verge-proxy-sort-type')
    return savedSortType ? (Number(savedSortType) as ProxySortType) : 0
  })
  const [delaySortRefresh, setDelaySortRefresh] = useState(0)

  const [state, setState] = useState<ProxyState>({
    proxyData: {
      groups: [],
      groupMap: {},
      records: {},
    },
    selection: {
      group: '',
      proxy: '',
    },
    displayProxy: null,
    resolvedPath: [],
  })

  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  const debouncedSetState = useCallback(
    (updateFn: (prev: ProxyState) => ProxyState) => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }

      timeoutRef.current = setTimeout(() => {
        setState(updateFn)
      }, 300)
    },
    [],
  )

  useEffect(() => {
    return () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current)
    }
  }, [])

  useEffect(() => {
    if (!proxies) return

    const preferredGroupName = getPreferredProxyGroupName({
      proxies,
      rules,
      isGlobalMode,
    })

    if (isGlobalMode) {
      setState((prev) => ({
        ...prev,
        selection: {
          ...prev.selection,
          group: 'GLOBAL',
        },
      }))
      return
    }

    const savedGroup = readProfileScopedItem(STORAGE_KEY_GROUP)
    setState((prev) => ({
      ...prev,
      selection: {
        ...prev.selection,
        group: savedGroup || preferredGroupName || '',
      },
    }))
  }, [isGlobalMode, proxies, readProfileScopedItem, rules])

  useEffect(() => {
    if (!proxies) return

    const savedProxy = readProfileScopedItem(STORAGE_KEY_PROXY)

    setState((prev) => {
      const groupsMap = new Map<string, ProxyGroupOption>()

      const registerGroup = (group: any, aliasName?: string) => {
        if (!group && !aliasName) return

        const rawName =
          typeof group?.name === 'string' && group.name.length > 0
            ? group.name
            : aliasName
        const name = normalizePolicyName(rawName)
        if (!name || groupsMap.has(name)) return

        const rawAll = (
          Array.isArray(group?.all)
            ? (group.all as Array<string | { name?: string }>)
            : []
        ) as Array<string | { name?: string }>

        const allNames = rawAll
          .map((item) =>
            typeof item === 'string'
              ? normalizePolicyName(item)
              : normalizePolicyName(item?.name),
          )
          .filter((value): value is string => value.length > 0)

        const uniqueAll = Array.from(new Set(allNames))
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

      const preferredGroupName = getPreferredProxyGroupName({
        proxies,
        rules,
        isGlobalMode,
      })

      if (preferredGroupName) {
        const preferredGroup =
          proxies.groups?.find(
            (group: { name?: string }) => group?.name === preferredGroupName,
          ) ||
          (proxies.global?.name === preferredGroupName ? proxies.global : null) ||
          proxies.records?.[preferredGroupName]

        registerGroup(preferredGroup, preferredGroupName)
      }

      ;(proxies.groups || []).forEach((group: any) => registerGroup(group))

      const allGroups = Array.from(groupsMap.values())
      const groupMap = Object.fromEntries(
        allGroups.map((group) => [group.name, group]),
      )

      const manualGroups = allGroups.filter(
        (group) => group.displayKind === 'manual',
      )
      const strategyGroups = allGroups.filter(
        (group) => group.displayKind === 'strategy',
      )
      const visibleGroups = manualGroups.concat(strategyGroups)

      let newGroup = prev.selection.group
      let newProxy = ''
      let newDisplayProxy = null
      let resolvedPath: string[] = []

      if (isGlobalMode && proxies.global) {
        newGroup = 'GLOBAL'
        const globalNames = (proxies.global.all || [])
          .map((item: any) =>
            typeof item === 'string'
              ? normalizePolicyName(item)
              : normalizePolicyName(item?.name),
          )
          .filter((name: string) => Boolean(name))
        newProxy = pickVisibleProxyName(
          globalNames,
          proxies.records || {},
          proxies.global.now,
          prev.selection.proxy,
          savedProxy,
        )
        const resolved = resolveLeafProxy(proxies.records || {}, newProxy)
        newDisplayProxy = resolved.displayProxy
        resolvedPath = resolved.resolvedPath
      } else {
        const activeGroup =
          groupMap[newGroup] ||
          groupMap[preferredGroupName] ||
          visibleGroups[0] ||
          allGroups[0]

        if (activeGroup) {
          newGroup = activeGroup.name
          newProxy = pickVisibleProxyName(
            activeGroup.all,
            proxies.records || {},
            activeGroup.now,
            prev.selection.proxy,
            savedProxy,
          )
          const resolved = resolveLeafProxy(proxies.records || {}, newProxy)
          newDisplayProxy = resolved.displayProxy
          resolvedPath = [
            activeGroup.name,
            ...resolved.resolvedPath.filter((name) => name !== activeGroup.name),
          ]

          writeProfileScopedItem(STORAGE_KEY_GROUP, newGroup)
          if (newProxy) {
            writeProfileScopedItem(STORAGE_KEY_PROXY, newProxy)
          }
        }
      }

      return {
        proxyData: {
          groups: visibleGroups,
          groupMap,
          records: proxies.records || {},
        },
        selection: {
          group: newGroup,
          proxy: newProxy,
        },
        displayProxy: newDisplayProxy,
        resolvedPath,
      }
    })
  }, [
    isGlobalMode,
    proxies,
    readProfileScopedItem,
    rules,
    writeProfileScopedItem,
  ])

  const correctionAttemptRef = useRef('')

  useEffect(() => {
    if (!proxies?.records || !state.selection.group) {
      correctionAttemptRef.current = ''
      return
    }

    const currentGroup = isGlobalMode
      ? proxies.global
      : state.proxyData.groupMap[state.selection.group]

    const currentNow = normalizePolicyName(currentGroup?.now)
    if (
      !currentNow ||
      !isAuxiliarySelectionName(currentNow, state.proxyData.records)
    ) {
      correctionAttemptRef.current = ''
      return
    }

    const groupNames = (
      Array.isArray(currentGroup?.all)
        ? currentGroup.all.map((item: any) =>
            typeof item === 'string'
              ? normalizePolicyName(item)
              : normalizePolicyName(item?.name),
          )
        : []
    ).filter((name: string) => Boolean(name))

    const targetProxy = pickVisibleProxyName(
      groupNames,
      state.proxyData.records,
      state.selection.proxy,
      currentNow,
    )

    if (!targetProxy || targetProxy === currentNow) {
      correctionAttemptRef.current = ''
      return
    }

    const signature = `${state.selection.group}:${currentNow}->${targetProxy}`
    if (correctionAttemptRef.current === signature) {
      return
    }

    correctionAttemptRef.current = signature
    changeProxy(state.selection.group, targetProxy, currentNow, isGlobalMode)
  }, [
    changeProxy,
    isGlobalMode,
    proxies?.global,
    proxies?.records,
    state.proxyData.groupMap,
    state.proxyData.records,
    state.selection.group,
    state.selection.proxy,
  ])

  const handleGroupChange = useCallback(
    (value: string | number) => {
      if (isGlobalMode) return

      const newGroup = String(value)
      writeProfileScopedItem(STORAGE_KEY_GROUP, newGroup)

      setState((prev) => {
        const group = prev.proxyData.groupMap[newGroup]
        if (!group) {
          return {
            ...prev,
            selection: {
              ...prev.selection,
              group: newGroup,
            },
          }
        }

        const newProxy = pickVisibleProxyName(
          group.all,
          prev.proxyData.records,
          group.now,
          prev.selection.proxy,
        )
        const resolved = resolveLeafProxy(prev.proxyData.records, newProxy)

        return {
          ...prev,
          selection: {
            group: newGroup,
            proxy: newProxy,
          },
          displayProxy: resolved.displayProxy,
          resolvedPath: [
            newGroup,
            ...resolved.resolvedPath.filter((name) => name !== newGroup),
          ],
        }
      })
    },
    [isGlobalMode, writeProfileScopedItem],
  )

  const handleProxyChange = useCallback(
    (event: SelectChangeEvent<string>) => {
      const requestedProxy = event.target.value
      const currentGroup = state.selection.group
      const previousProxy = state.selection.proxy
      const currentGroupData = currentGroup
        ? state.proxyData.groupMap[currentGroup]
        : null
      const newProxy = currentGroupData
        ? pickVisibleProxyName(
            currentGroupData.all,
            state.proxyData.records,
            requestedProxy,
            previousProxy,
          )
        : requestedProxy

      debouncedSetState((prev: ProxyState) => {
        const resolved = resolveLeafProxy(prev.proxyData.records, newProxy)

        return {
          ...prev,
          selection: {
            ...prev.selection,
            proxy: newProxy,
          },
          displayProxy: resolved.displayProxy,
          resolvedPath: currentGroup
            ? [
                currentGroup,
                ...resolved.resolvedPath.filter((name) => name !== currentGroup),
              ]
            : resolved.resolvedPath,
        }
      })

      if (!isGlobalMode) {
        writeProfileScopedItem(STORAGE_KEY_PROXY, newProxy)
      }

      const skipConfigSave = isGlobalMode
      handleSelectChange(currentGroup, previousProxy, skipConfigSave)({
        target: { value: newProxy },
      })
    },
    [
      debouncedSetState,
      handleSelectChange,
      isGlobalMode,
      state.proxyData.groupMap,
      state.proxyData.records,
      state.selection.group,
      state.selection.proxy,
      writeProfileScopedItem,
    ],
  )

  const handleSortTypeChange = useCallback(() => {
    const newSortType = ((sortType + 1) % 3) as ProxySortType
    setSortType(newSortType)
    localStorage.setItem('clash-verge-proxy-sort-type', newSortType.toString())
  }, [sortType])

  const proxyOptions = useMemo(() => {
    const sortWithLatency = (options: ProxyOption[]) => {
      if (!options || sortType === 0) {
        return [...options].sort(
          (a, b) => KIND_WEIGHT[a.kind] - KIND_WEIGHT[b.kind],
        )
      }

      const list = [...options]

      if (sortType === 1) {
        const effectiveTimeout =
          typeof defaultLatencyTimeout === 'number' && defaultLatencyTimeout > 0
            ? defaultLatencyTimeout
            : 10000
        const refreshTick = delaySortRefresh

        list.sort((a, b) => {
          const kindDiff = KIND_WEIGHT[a.kind] - KIND_WEIGHT[b.kind]
          if (kindDiff !== 0) return kindDiff

          const recordA = state.proxyData.records[a.name]
          const recordB = state.proxyData.records[b.name]

          const [ar, av] = recordA
            ? categorizeDelay(
                delayManager.getDelayFix(recordA, state.selection.group),
                effectiveTimeout,
              )
            : [6, Number.MAX_SAFE_INTEGER]
          const [br, bv] = recordB
            ? categorizeDelay(
                delayManager.getDelayFix(recordB, state.selection.group),
                effectiveTimeout,
              )
            : [6, Number.MAX_SAFE_INTEGER]

          if (ar !== br) return ar - br
          if (av !== bv) return av - bv
          return refreshTick >= 0 ? a.name.localeCompare(b.name) : 0
        })

        return list
      }

      list.sort((a, b) => {
        const kindDiff = KIND_WEIGHT[a.kind] - KIND_WEIGHT[b.kind]
        if (kindDiff !== 0) return kindDiff
        return a.name.localeCompare(b.name)
      })

      return list
    }

    if (isGlobalMode && proxies?.global) {
      const options = buildProxyDisplayOptionsFromNames({
        names: (proxies.global.all || [])
          .map((item: any) => (typeof item === 'string' ? item : item?.name))
          .filter((name: string) => name && name !== 'DIRECT' && name !== 'REJECT'),
        records: state.proxyData.records,
      }).filter(
        (option): option is ProxyOption =>
          option.kind === 'manual' || option.kind === 'strategy',
      )

      return sortWithLatency(options)
    }

    const currentGroup = state.selection.group
      ? state.proxyData.groupMap[state.selection.group]
      : null

    if (currentGroup) {
      const options = buildProxyDisplayOptionsFromNames({
        names: currentGroup.all,
        records: state.proxyData.records,
      }).filter(
        (option): option is ProxyOption =>
          option.kind === 'manual' || option.kind === 'strategy',
      )

      return sortWithLatency(options)
    }

    return []
  }, [
    defaultLatencyTimeout,
    delaySortRefresh,
    isGlobalMode,
    proxies,
    sortType,
    state.proxyData.groupMap,
    state.proxyData.records,
    state.selection.group,
  ])

  return {
    state,
    sortType,
    proxyOptions,
    handleGroupChange,
    handleProxyChange,
    handleSortTypeChange,
    triggerDelaySortRefresh: () => setDelaySortRefresh((prev) => prev + 1),
  }
}
