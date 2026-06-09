import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

import { SelectChangeEvent } from '@/components/tailwind/Select'
import { useProxySelection } from '@/hooks/data'

import { buildCurrentProxyOptions } from './current-proxy-data/build-proxy-options'
import { buildProxyState } from './current-proxy-data/build-proxy-state'
import { resolveAuxiliarySelectionCorrection } from './current-proxy-data/resolve-auxiliary-selection-correction'
import {
  INITIAL_PROXY_STATE,
  buildSelectionSnapshot,
  pickVisibleProxyName,
  type ProxySortType,
  type ProxyState,
  type UseCurrentProxyDataProps,
} from './current-proxy-data/shared'
import {
  STORAGE_KEY_GROUP,
  STORAGE_KEY_PROXY,
  STORAGE_KEY_SORT_TYPE,
  useProxyStorage,
} from './use-proxy-storage'

export const useCurrentProxyData = ({
  proxies,
  rules,
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
    const savedSortType = localStorage.getItem(STORAGE_KEY_SORT_TYPE)
    return savedSortType ? (Number(savedSortType) as ProxySortType) : 0
  })
  const [delaySortRefresh, setDelaySortRefresh] = useState(0)
  const [state, setState] = useState<ProxyState>(INITIAL_PROXY_STATE)

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
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current)
      }
    }
  }, [])

  useEffect(() => {
    if (!proxies) return

    const savedGroup = readProfileScopedItem(STORAGE_KEY_GROUP)
    const savedProxy = readProfileScopedItem(STORAGE_KEY_PROXY)

    setState((prev) => {
      const next = buildProxyState({
        isGlobalMode,
        prevState: prev,
        proxies,
        rules,
        savedGroup,
        savedProxy,
      })

      if (next.persistedGroup) {
        writeProfileScopedItem(STORAGE_KEY_GROUP, next.persistedGroup)
      }
      if (next.persistedProxy) {
        writeProfileScopedItem(STORAGE_KEY_PROXY, next.persistedProxy)
      }

      return next.state
    })
  }, [isGlobalMode, proxies, readProfileScopedItem, rules, writeProfileScopedItem])

  const correctionAttemptRef = useRef('')

  useEffect(() => {
    const correction = resolveAuxiliarySelectionCorrection({
      isGlobalMode,
      proxies,
      state,
    })

    if (!correction) {
      correctionAttemptRef.current = ''
      return
    }

    if (correctionAttemptRef.current === correction.signature) {
      return
    }

    correctionAttemptRef.current = correction.signature
    changeProxy(
      state.selection.group,
      correction.targetProxy,
      correction.currentNow,
      isGlobalMode,
    )
  }, [changeProxy, isGlobalMode, proxies, state])

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
        const snapshot = buildSelectionSnapshot(
          prev.proxyData.records,
          newGroup,
          newProxy,
        )

        return {
          ...prev,
          selection: {
            group: newGroup,
            proxy: newProxy,
          },
          displayProxy: snapshot.displayProxy,
          resolvedPath: snapshot.resolvedPath,
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

      debouncedSetState((prev) => {
        const snapshot = buildSelectionSnapshot(
          prev.proxyData.records,
          currentGroup || null,
          newProxy,
        )

        return {
          ...prev,
          selection: {
            ...prev.selection,
            proxy: newProxy,
          },
          displayProxy: snapshot.displayProxy,
          resolvedPath: snapshot.resolvedPath,
        }
      })

      if (!isGlobalMode) {
        writeProfileScopedItem(STORAGE_KEY_PROXY, newProxy)
      }

      handleSelectChange(currentGroup, previousProxy, isGlobalMode)({
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
    localStorage.setItem(STORAGE_KEY_SORT_TYPE, newSortType.toString())
  }, [sortType])

  const proxyOptions = useMemo(
    () =>
      buildCurrentProxyOptions({
        defaultLatencyTimeout,
        delaySortRefresh,
        groupMap: state.proxyData.groupMap,
        isGlobalMode,
        proxies,
        records: state.proxyData.records,
        selectionGroup: state.selection.group,
        sortType,
      }),
    [
      defaultLatencyTimeout,
      delaySortRefresh,
      isGlobalMode,
      proxies,
      sortType,
      state.proxyData.groupMap,
      state.proxyData.records,
      state.selection.group,
    ],
  )

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
