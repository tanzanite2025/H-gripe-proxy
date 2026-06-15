import { useCallback, useMemo } from 'react'

import { useProxySelection } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import { useProxiesData } from '@/providers/app-data-context'
import { resolveVergeDelayTimeout } from '@/services/delay-config'

import { useRenderList } from '../../use-render-list'

import {
  findProxyGroupHeaderIndex,
  findProxyGroupScrollTarget,
  getGroupHeadStateFromRenderList,
  getProxyGroupNamesFromRenderList,
} from './proxy-group-render-list'
import { useAuxiliarySelectionCorrection } from './use-auxiliary-selection-correction'

interface UseProxyGroupsOptions {
  isChainMode: boolean
}

export function useProxyGroups(options: UseProxyGroupsOptions) {
  const { isChainMode } = options

  const { verge } = useVerge()
  const { proxies: proxiesData } = useProxiesData()

  const { renderList, onProxies, onHeadState } = useRenderList(isChainMode)

  const getGroupHeadState = useCallback(
    (groupName: string) =>
      getGroupHeadStateFromRenderList(renderList, groupName),
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

  useAuxiliarySelectionCorrection(proxiesData, changeProxy)

  const timeout = resolveVergeDelayTimeout(verge)

  const proxyGroupNames = useMemo(() => {
    return getProxyGroupNamesFromRenderList(renderList)
  }, [renderList])

  const handleLocation = useCallback(
    (
      group: IProxyGroupItem,
      scrollToIndex: (index: number, options?: any) => void,
    ) => {
      if (!group) return

      const target = findProxyGroupScrollTarget(renderList, group)
      if (!target) return

      scrollToIndex(target.index, {
        align: target.align,
        behavior: 'smooth',
      })
    },
    [renderList],
  )

  const handleGroupLocationByName = useCallback(
    (groupName: string, scrollToIndex: (index: number, options?: any) => void) => {
      const index = findProxyGroupHeaderIndex(renderList, groupName)

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
