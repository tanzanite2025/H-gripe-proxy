import { useLockFn } from 'ahooks'
import { delayGroup, healthcheckProxyProvider } from 'tauri-plugin-mihomo-api'

import { useProxiesData } from '@/providers/app-data-context'
import { resolveDelayTimeout } from '@/services/delay-config'
import delayManager from '@/services/delay'
import { debugLog } from '@/utils/misc'

interface UseProxyDelayCheckProps {
  currentGroup: string
  defaultLatencyTimeout: number
  proxyRecords: Record<string, any>
  refreshProxy: () => void
  onDelayCheckComplete?: () => void
}

export const useProxyDelayCheck = ({
  currentGroup,
  defaultLatencyTimeout,
  proxyRecords,
  refreshProxy,
  onDelayCheckComplete,
}: UseProxyDelayCheckProps) => {
  const { proxies } = useProxiesData()

  const handleCheckAllDelay = useLockFn(async (isGlobalMode: boolean) => {
    const groupName = currentGroup
    if (!groupName) return

    debugLog(`[CurrentProxyCard] Start delay check, group: ${groupName}`)

    const timeout = resolveDelayTimeout(defaultLatencyTimeout)
    const proxyNames: string[] = []
    const providers: Set<string> = new Set()

    if (isGlobalMode && proxies?.global) {
      const allProxies = proxies.global.all
        .filter((proxy: any) => {
          const name = typeof proxy === 'string' ? proxy : proxy.name
          return name !== 'DIRECT' && name !== 'REJECT'
        })
        .map((proxy: any) =>
          typeof proxy === 'string' ? proxy : proxy.name,
        )

      allProxies.forEach((name: string) => {
        const proxy = proxyRecords[name]
        if (proxy?.provider) {
          providers.add(proxy.provider)
        } else {
          proxyNames.push(name)
        }
      })
    } else {
      debugLog(
        '[CurrentProxyCard] Rule mode batch delay check requires group proxy data',
      )
    }

    debugLog(
      `[CurrentProxyCard] Proxy count: ${proxyNames.length}, provider count: ${providers.size}`,
    )

    if (providers.size > 0) {
      debugLog('[CurrentProxyCard] Start provider health checks')
      await Promise.allSettled(
        [...providers].map((provider) => healthcheckProxyProvider(provider)),
      )
    }

    if (proxyNames.length > 0) {
      const url = delayManager.getUrl(groupName)
      debugLog(`[CurrentProxyCard] Test URL: ${url}, timeout: ${timeout}ms`)

      try {
        proxyNames.forEach((name) => {
          delayManager.setDelay(name, groupName, -2)
        })

        const result = await delayGroup(groupName, url, timeout, false)
        debugLog(
          `[CurrentProxyCard] Group delay result count: ${Object.keys(result || {}).length}`,
        )

        proxyNames.forEach((name) => {
          delayManager.setDelay(name, groupName, result?.[name] ?? 0)
        })

        debugLog(`[CurrentProxyCard] Delay check finished, group: ${groupName}`)
      } catch (error) {
        console.warn(
          `[CurrentProxyCard] Group delay failed, fallback to per-proxy checks, group: ${groupName}`,
          error,
        )

        try {
          await delayManager.checkListDelay(proxyNames, groupName, timeout)
        } catch (fallbackError) {
          console.error(
            `[CurrentProxyCard] Fallback delay check failed, group: ${groupName}`,
            fallbackError,
          )
        }
      }
    }

    refreshProxy()
    onDelayCheckComplete?.()
  })

  return {
    handleCheckAllDelay,
  }
}
