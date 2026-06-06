import { useLockFn } from 'ahooks'
import { useCallback, useRef } from 'react'
import { delayGroup, healthcheckProxyProvider } from 'tauri-plugin-mihomo-api'

import delayManager from '@/services/delay'
import { debugLog } from '@/utils/misc'

import type { HeadState } from '../../use-head-state'
import type { IRenderItem } from '../../use-render-list'

interface UseDelayCheckOptions {
  renderList: IRenderItem[]
  timeout: number
  getGroupHeadState: (groupName: string) => HeadState | undefined
  onProxies: () => void
  onHeadState: (groupName: string, patch: Partial<HeadState>) => void
}

export function useDelayCheck(options: UseDelayCheckOptions) {
  const { renderList, timeout, getGroupHeadState, onProxies, onHeadState } =
    options

  const fnRef = useRef<(groupName: string) => Promise<void>>(async () => {})

  const handleCheckAll = useLockFn(async (groupName: string) => {
    debugLog(`[ProxyGroups] Start delay check, group: ${groupName}`)

    const proxies = renderList
      .filter(
        (e) => e.group?.name === groupName && (e.type === 2 || e.type === 4),
      )
      .flatMap((e) => e.proxyCol || e.proxy!)
      .filter(Boolean)

    debugLog(`[ProxyGroups] Proxy count: ${proxies.length}`)

    const providers = new Set(
      proxies.map((proxy) => proxy!.provider!).filter(Boolean),
    )

    if (providers.size > 0) {
      debugLog(`[ProxyGroups] Provider count: ${providers.size}`)
      Promise.allSettled(
        [...providers].map((provider) => healthcheckProxyProvider(provider)),
      ).then(() => {
        debugLog('[ProxyGroups] Provider health check finished')
        onProxies()
      })
    }

    const names = proxies
      .filter((proxy) => !proxy!.provider)
      .map((proxy) => proxy!.name)

    debugLog(`[ProxyGroups] Names to test: ${names.length}`)

    const url = delayManager.getUrl(groupName)
    debugLog(`[ProxyGroups] Test URL: ${url}, timeout: ${timeout}ms`)

    try {
      names.forEach((name) => {
        delayManager.setDelay(name, groupName, -2)
      })

      const result = await delayGroup(groupName, url, timeout, false)
      debugLog(
        `[ProxyGroups] Group delay result count: ${Object.keys(result || {}).length}`,
      )

      names.forEach((name) => {
        delayManager.setDelay(name, groupName, result?.[name] ?? 0)
      })

      debugLog(`[ProxyGroups] Delay check finished, group: ${groupName}`)
    } catch (error) {
      console.warn(
        `[ProxyGroups] Group delay failed, fallback to per-proxy checks, group: ${groupName}`,
        error,
      )

      try {
        await delayManager.checkListDelay(names, groupName, timeout)
      } catch (fallbackError) {
        console.error(
          `[ProxyGroups] Fallback delay check failed, group: ${groupName}`,
          fallbackError,
        )
      }
    } finally {
      const headState = getGroupHeadState(groupName)
      if (headState?.sortType === 1) {
        onHeadState(groupName, { sortType: headState.sortType })
      }
      onProxies()
    }
  })

  fnRef.current = handleCheckAll

  const stableHandleCheckAll = useCallback(
    (groupName: string) => fnRef.current(groupName),
    [],
  )

  return {
    handleCheckAll: stableHandleCheckAll,
  }
}
