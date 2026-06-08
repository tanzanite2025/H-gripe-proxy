import { useCallback, useEffect, useMemo, useRef } from 'react'

import { useProxySelection } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import { useProxiesData } from '@/providers/app-data-context'
import {
  getDisplayableTopLevelGroups,
  isAuxiliarySelectionName,
  pickPreferredProxyNameFromGroup,
} from '@/services/proxy-display'

import { useRenderList } from '../../use-render-list'

interface UseProxyGroupsOptions {
  mode: string
  isChainMode: boolean
}

export function useProxyGroups(options: UseProxyGroupsOptions) {
  const { mode, isChainMode } = options

  const { verge } = useVerge()
  const { proxies: proxiesData } = useProxiesData()

  const { renderList, onProxies, onHeadState } = useRenderList(
    mode,
    isChainMode,
  )

  const getGroupHeadState = useCallback(
    (groupName: string) => {
      const headItem = renderList.find(
        (item) => item.type === 1 && item.group?.name === groupName,
      )
      return headItem?.headState
    },
    [renderList],
  )

  const { changeProxy, handleProxyGroupChange } = useProxySelection({
    onSuccess: () => {
      onProxies()
    },
    onError: (error) => {
      console.error('Proxy switch failed', error)
      onProxies()
    },
  })

  const timeout = verge?.default_latency_timeout || 10000
  const correctionSignatureRef = useRef('')

  useEffect(() => {
    if (!proxiesData?.records) {
      correctionSignatureRef.current = ''
      return
    }

    const groups = [proxiesData.global, ...(proxiesData.groups || [])].filter(
      Boolean,
    ) as IProxyGroupItem[]

    const corrections = groups
      .map((group) => {
        const currentName = group.now?.trim() || ''
        if (
          !currentName ||
          !isAuxiliarySelectionName(currentName, proxiesData.records)
        ) {
          return null
        }

        const targetName = pickPreferredProxyNameFromGroup(
          group,
          proxiesData.records,
          group.now,
        )

        if (!targetName || targetName === currentName) {
          return null
        }

        return {
          groupName: group.name,
          previousProxy: currentName,
          proxyName: targetName,
        }
      })
      .filter(
        (
          correction,
        ): correction is {
          groupName: string
          previousProxy: string
          proxyName: string
        } => Boolean(correction),
      )

    if (corrections.length === 0) {
      correctionSignatureRef.current = ''
      return
    }

    const signature = corrections
      .map(
        ({ groupName, previousProxy, proxyName }) =>
          `${groupName}:${previousProxy}->${proxyName}`,
      )
      .join('|')

    if (correctionSignatureRef.current === signature) {
      return
    }

    correctionSignatureRef.current = signature

    corrections.forEach(({ groupName, previousProxy, proxyName }) => {
      changeProxy(groupName, proxyName, previousProxy)
    })
  }, [changeProxy, proxiesData])

  const proxyGroupNames = useMemo(() => {
    if (!proxiesData) return []
    return getDisplayableTopLevelGroups({
      groups: proxiesData.groups,
      global: proxiesData.global,
    }).map((group) => group.name)
  }, [proxiesData])

  const handleLocation = useCallback(
    (
      group: IProxyGroupItem,
      scrollToIndex: (index: number, options?: any) => void,
    ) => {
      if (!group) return
      const { name, now } = group

      const index = renderList.findIndex(
        (item) =>
          item.group?.name === name &&
          ((item.type === 2 && item.proxy?.name === now) ||
            (item.type === 4 && item.proxyCol?.some((proxy) => proxy.name === now))),
      )

      if (index >= 0) {
        scrollToIndex(index, { align: 'center', behavior: 'smooth' })
        return
      }

      const groupIndex = renderList.findIndex(
        (item) => item.type === 0 && item.group?.name === name,
      )

      if (groupIndex >= 0) {
        scrollToIndex(groupIndex, { align: 'start', behavior: 'smooth' })
      }
    },
    [renderList],
  )

  const handleGroupLocationByName = useCallback(
    (groupName: string, scrollToIndex: (index: number, options?: any) => void) => {
      const index = renderList.findIndex(
        (item) => item.type === 0 && item.group?.name === groupName,
      )

      if (index >= 0) {
        scrollToIndex(index, { align: 'start', behavior: 'smooth' })
      }
    },
    [renderList],
  )

  return {
    proxiesData,
    renderList,
    timeout,
    proxyGroupNames,
    onProxies,
    onHeadState,
    getGroupHeadState,
    handleProxyGroupChange,
    handleLocation,
    handleGroupLocationByName,
  }
}
