import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { SelectChangeEvent } from '@/components/tailwind/Select'
import { useProxySelection } from '@/hooks/data'
import delayManager from '@/services/delay'

import { categorizeDelay, normalizePolicyName } from '../utils/proxy-helpers'

import {
  STORAGE_KEY_GROUP,
  STORAGE_KEY_PROXY,
  useProxyStorage,
} from './use-proxy-storage'

// 代理组选项接口
type ProxyGroupOption = {
  name: string
  now: string
  all: string[]
  type?: string
}

// 代理节点选项接口
interface ProxyOption {
  name: string
}

// 排序类型: 默认 | 按延迟 | 按字母
export type ProxySortType = 0 | 1 | 2

// 代理状态接口
export type ProxyState = {
  proxyData: {
    groups: ProxyGroupOption[]
    records: Record<string, any>
  }
  selection: {
    group: string
    proxy: string
  }
  displayProxy: any
}

interface UseCurrentProxyDataProps {
  proxies: any
  rules: any[]
  clashConfig: any
  currentProfileId: string | null
  isGlobalMode: boolean
  isDirectMode: boolean
  defaultLatencyTimeout: number
  refreshProxy: () => void
}

/**
 * 当前代理数据管理 Hook
 * 处理代理数据、选择状态、排序等
 */
export const useCurrentProxyData = ({
  proxies,
  rules,
  clashConfig: _clashConfig,
  currentProfileId,
  isGlobalMode,
  isDirectMode,
  defaultLatencyTimeout,
  refreshProxy,
}: UseCurrentProxyDataProps) => {
  const { readProfileScopedItem, writeProfileScopedItem } =
    useProxyStorage(currentProfileId)

  // 统一代理选择器
  const { handleSelectChange } = useProxySelection({
    onSuccess: () => {
      refreshProxy()
    },
    onError: (error) => {
      console.error('代理切换失败', error)
      refreshProxy()
    },
  })

  // 排序类型状态
  const [sortType, setSortType] = useState<ProxySortType>(() => {
    const savedSortType = localStorage.getItem('clash-verge-proxy-sort-type')
    return savedSortType ? (Number(savedSortType) as ProxySortType) : 0
  })
  const [delaySortRefresh, setDelaySortRefresh] = useState(0)

  // 代理状态
  const [state, setState] = useState<ProxyState>({
    proxyData: {
      groups: [],
      records: {},
    },
    selection: {
      group: '',
      proxy: '',
    },
    displayProxy: null,
  })

  // 防抖状态更新
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

  // 获取匹配策略名称
  const matchPolicyName = useMemo(() => {
    if (!Array.isArray(rules)) return ''
    for (let index = rules.length - 1; index >= 0; index -= 1) {
      const rule = rules[index]
      if (!rule) continue

      if (
        typeof rule?.type === 'string' &&
        rule.type.toUpperCase() === 'MATCH'
      ) {
        const policy = normalizePolicyName(rule.proxy)
        if (policy) {
          return policy
        }
      }
    }
    return ''
  }, [rules])

  // 初始化选择的组
  useEffect(() => {
    if (!proxies) return

    const getPrimaryGroupName = () => {
      if (!proxies?.groups?.length) return ''

      const primaryKeywords = [
        'auto',
        'select',
        'proxy',
        '节点选择',
        '自动选择',
      ]
      const primaryGroup =
        proxies.groups.find((group: { name: string }) =>
          primaryKeywords.some((keyword) =>
            group.name.toLowerCase().includes(keyword.toLowerCase()),
          ),
        ) ||
        proxies.groups.filter((g: { name: string }) => g.name !== 'GLOBAL')[0]

      return primaryGroup?.name || ''
    }

    const primaryGroupName = getPrimaryGroupName()

    // 根据模式确定初始组
    if (isGlobalMode) {
      setState((prev) => ({
        ...prev,
        selection: {
          ...prev.selection,
          group: 'GLOBAL',
        },
      }))
    } else if (isDirectMode) {
      setState((prev) => ({
        ...prev,
        selection: {
          ...prev.selection,
          group: 'DIRECT',
        },
      }))
    } else {
      const savedGroup = readProfileScopedItem(STORAGE_KEY_GROUP)
      setState((prev) => ({
        ...prev,
        selection: {
          ...prev.selection,
          group: savedGroup || primaryGroupName || '',
        },
      }))
    }
  }, [isGlobalMode, isDirectMode, proxies, readProfileScopedItem])

  // 监听代理数据变化，更新状态
  useEffect(() => {
    if (!proxies) return

    setState((prev) => {
      const groupsMap = new Map<string, ProxyGroupOption>()

      const registerGroup = (group: any, fallbackName?: string) => {
        if (!group && !fallbackName) return

        const rawName =
          typeof group?.name === 'string' && group.name.length > 0
            ? group.name
            : fallbackName
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

        groupsMap.set(name, {
          name,
          now: normalizePolicyName(group?.now),
          all: uniqueAll,
          type: group?.type,
        })
      }

      if (matchPolicyName) {
        const matchGroup =
          proxies.groups?.find(
            (g: { name: string }) => g.name === matchPolicyName,
          ) ||
          (proxies.global?.name === matchPolicyName ? proxies.global : null) ||
          proxies.records?.[matchPolicyName]
        registerGroup(matchGroup, matchPolicyName)
      }

      ;(proxies.groups || [])
        .filter((g: { type?: string }) => g?.type === 'Selector')
        .forEach((selectorGroup: any) => registerGroup(selectorGroup))

      const filteredGroups = Array.from(groupsMap.values())

      let newProxy = ''
      let newDisplayProxy = null
      let newGroup = prev.selection.group

      if (isDirectMode) {
        newGroup = 'DIRECT'
        newProxy = 'DIRECT'
        newDisplayProxy = proxies.records?.DIRECT || { name: 'DIRECT' }
      } else if (isGlobalMode && proxies.global) {
        newGroup = 'GLOBAL'
        newProxy = proxies.global.now || ''
        newDisplayProxy = proxies.records?.[newProxy] || null
      } else {
        const currentGroup = filteredGroups.find(
          (g: { name: string }) => g.name === prev.selection.group,
        )

        if (!currentGroup && filteredGroups.length > 0) {
          const firstGroup = filteredGroups[0]
          if (firstGroup) {
            newGroup = firstGroup.name
            newProxy = firstGroup.now || firstGroup.all[0] || ''
            newDisplayProxy = proxies.records?.[newProxy] || null

            if (!isGlobalMode && !isDirectMode) {
              writeProfileScopedItem(STORAGE_KEY_GROUP, newGroup)
              if (newProxy) {
                writeProfileScopedItem(STORAGE_KEY_PROXY, newProxy)
              }
            }
          }
        } else if (currentGroup) {
          newProxy = currentGroup.now || currentGroup.all[0] || ''
          newDisplayProxy = proxies.records?.[newProxy] || null
        }
      }

      return {
        proxyData: {
          groups: filteredGroups,
          records: proxies.records || {},
        },
        selection: {
          group: newGroup,
          proxy: newProxy,
        },
        displayProxy: newDisplayProxy,
      }
    })
  }, [
    proxies,
    isGlobalMode,
    isDirectMode,
    writeProfileScopedItem,
    matchPolicyName,
  ])

  // 处理代理组变更
  const handleGroupChange = useCallback(
    (value: string | number) => {
      if (isGlobalMode || isDirectMode) return

      const newGroup = String(value)

      writeProfileScopedItem(STORAGE_KEY_GROUP, newGroup)

      setState((prev) => {
        const group = prev.proxyData.groups.find(
          (g: { name: string }) => g.name === newGroup,
        )
        if (group) {
          return {
            ...prev,
            selection: {
              group: newGroup,
              proxy: group.now,
            },
            displayProxy: prev.proxyData.records[group.now] || null,
          }
        }
        return {
          ...prev,
          selection: {
            ...prev.selection,
            group: newGroup,
          },
        }
      })
    },
    [isGlobalMode, isDirectMode, writeProfileScopedItem],
  )

  // 处理代理节点变更
  const handleProxyChange = useCallback(
    (event: SelectChangeEvent<string>) => {
      if (isDirectMode) return

      const newProxy = event.target.value
      const currentGroup = state.selection.group
      const previousProxy = state.selection.proxy

      debouncedSetState((prev: ProxyState) => ({
        ...prev,
        selection: {
          ...prev.selection,
          proxy: newProxy,
        },
        displayProxy: prev.proxyData.records[newProxy] || null,
      }))

      if (!isGlobalMode && !isDirectMode) {
        writeProfileScopedItem(STORAGE_KEY_PROXY, newProxy)
      }

      const skipConfigSave = isGlobalMode || isDirectMode
      handleSelectChange(currentGroup, previousProxy, skipConfigSave)(event)
    },
    [
      isDirectMode,
      isGlobalMode,
      state.selection,
      debouncedSetState,
      handleSelectChange,
      writeProfileScopedItem,
    ],
  )

  // 排序类型变更
  const handleSortTypeChange = useCallback(() => {
    const newSortType = ((sortType + 1) % 3) as ProxySortType
    setSortType(newSortType)
    localStorage.setItem('clash-verge-proxy-sort-type', newSortType.toString())
  }, [sortType])

  // 计算要显示的代理选项
  const proxyOptions = useMemo(() => {
    const sortWithLatency = (proxiesToSort: ProxyOption[]) => {
      if (!proxiesToSort || sortType === 0) return proxiesToSort

      if (!state.proxyData.records || !state.selection.group) {
        return proxiesToSort
      }

      const list = [...proxiesToSort]

      if (sortType === 1) {
        const refreshTick = delaySortRefresh
        const effectiveTimeout =
          typeof defaultLatencyTimeout === 'number' && defaultLatencyTimeout > 0
            ? defaultLatencyTimeout
            : 10000

        list.sort((a, b) => {
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
      } else {
        list.sort((a, b) => a.name.localeCompare(b.name))
      }

      return list
    }

    if (isDirectMode) {
      return [{ name: 'DIRECT' }]
    }
    if (isGlobalMode && proxies?.global) {
      const options = proxies.global.all
        .filter((p: any) => {
          const name = typeof p === 'string' ? p : p.name
          return name !== 'DIRECT' && name !== 'REJECT'
        })
        .map((p: any) => ({
          name: typeof p === 'string' ? p : p.name,
        }))

      return sortWithLatency(options)
    }

    // 规则模式
    const group = state.selection.group
      ? state.proxyData.groups.find((g) => g.name === state.selection.group)
      : null

    if (group) {
      const options = group.all.map((name) => ({ name }))
      return sortWithLatency(options)
    }

    return []
  }, [
    isDirectMode,
    isGlobalMode,
    proxies,
    state.proxyData,
    state.selection.group,
    sortType,
    delaySortRefresh,
    defaultLatencyTimeout,
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
