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

/**
 * 管理延迟测试逻辑
 */
export function useDelayCheck(options: UseDelayCheckOptions) {
  const { renderList, timeout, getGroupHeadState, onProxies, onHeadState } =
    options

  // 创建稳定的回调引用
  const fnRef = useRef<(groupName: string) => Promise<void>>(async () => {})

  // 测试全部延迟
  const handleCheckAll = useLockFn(async (groupName: string) => {
      debugLog(`[ProxyGroups] 开始测试所有延迟，组: ${groupName}`)

      const proxies = renderList
        .filter(
          (e) => e.group?.name === groupName && (e.type === 2 || e.type === 4),
        )
        .flatMap((e) => e.proxyCol || e.proxy!)
        .filter(Boolean)

      debugLog(`[ProxyGroups] 找到代理数量: ${proxies.length}`)

      const providers = new Set(
        proxies.map((p) => p!.provider!).filter(Boolean),
      )

      if (providers.size) {
        debugLog(`[ProxyGroups] 发现提供者，数量: ${providers.size}`)
        Promise.allSettled(
          [...providers].map((p) => healthcheckProxyProvider(p)),
        ).then(() => {
          debugLog(`[ProxyGroups] 提供者健康检查完成`)
          onProxies()
        })
      }

      const names = proxies.filter((p) => !p!.provider).map((p) => p!.name)
      debugLog(`[ProxyGroups] 过滤后需要测试的代理数量: ${names.length}`)

      const url = delayManager.getUrl(groupName)
      debugLog(`[ProxyGroups] 测试URL: ${url}, 超时: ${timeout}ms`)

      try {
        await Promise.race([
          delayManager.checkListDelay(names, groupName, timeout),
          delayGroup(groupName, url, timeout).then((result) => {
            debugLog(
              `[ProxyGroups] getGroupProxyDelays返回结果数量:`,
              Object.keys(result || {}).length,
            )
          }),
        ])
        debugLog(`[ProxyGroups] 延迟测试完成，组: ${groupName}`)
      } catch (error) {
        console.error(`[ProxyGroups] 延迟测试出错，组: ${groupName}`, error)
      } finally {
        const headState = getGroupHeadState(groupName)
        if (headState?.sortType === 1) {
          onHeadState(groupName, { sortType: headState.sortType })
        }
        onProxies()
      }
    })

  fnRef.current = handleCheckAll

  // 返回稳定的回调引用
  const stableHandleCheckAll = useCallback(
    (groupName: string) => fnRef.current(groupName),
    [],
  )

  return {
    handleCheckAll: stableHandleCheckAll,
  }
}
