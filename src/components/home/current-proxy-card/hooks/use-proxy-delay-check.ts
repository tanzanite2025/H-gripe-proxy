import { useLockFn } from 'ahooks'
import { useCallback, useEffect, useRef } from 'react'
import { delayGroup, healthcheckProxyProvider } from 'tauri-plugin-mihomo-api'

import { useProxiesData } from '@/providers/app-data-context'
import delayManager from '@/services/delay'
import { debugLog } from '@/utils/misc'

const _AUTO_CHECK_DEFAULT_INTERVAL_MINUTES = 5
const AUTO_CHECK_INITIAL_DELAY_MS = 100

interface UseProxyDelayCheckProps {
  currentGroup: string
  currentProxy: string
  currentProxyRecord: any
  autoDelayEnabled: boolean
  autoDelayIntervalMs: number
  defaultLatencyTimeout: number
  proxyRecords: Record<string, any>
  refreshProxy: () => void
  onDelayCheckComplete?: () => void
}

export const useProxyDelayCheck = ({
  currentGroup,
  currentProxy,
  currentProxyRecord,
  autoDelayEnabled,
  autoDelayIntervalMs,
  defaultLatencyTimeout,
  proxyRecords,
  refreshProxy,
  onDelayCheckComplete,
}: UseProxyDelayCheckProps) => {
  const { proxies } = useProxiesData()
  const autoCheckInProgressRef = useRef(false)
  const latestTimeoutRef = useRef<number>(defaultLatencyTimeout)
  const latestProxyRecordRef = useRef<any | null>(null)

  useEffect(() => {
    latestTimeoutRef.current = defaultLatencyTimeout
  }, [defaultLatencyTimeout])

  useEffect(() => {
    if (!currentProxy) {
      latestProxyRecordRef.current = null
      return
    }
    latestProxyRecordRef.current = currentProxyRecord || null
  }, [currentProxy, currentProxyRecord])

  const checkCurrentProxyDelay = useCallback(async () => {
    if (autoCheckInProgressRef.current) return

    const groupName = currentGroup
    const proxyName = currentProxy

    if (!groupName || !proxyName) return

    const proxyRecord = latestProxyRecordRef.current
    if (!proxyRecord) {
      debugLog(
        `[CurrentProxyCard] Skip auto delay check, missing proxy record, group: ${groupName}, proxy: ${proxyName}`,
      )
      return
    }

    autoCheckInProgressRef.current = true

    const timeout = latestTimeoutRef.current || 10000

    try {
      debugLog(
        `[CurrentProxyCard] Auto check current proxy delay, group: ${groupName}, proxy: ${proxyName}`,
      )
      if (proxyRecord.provider) {
        await healthcheckProxyProvider(proxyRecord.provider)
      } else {
        await delayManager.checkDelay(proxyName, groupName, timeout)
      }
    } catch (error) {
      console.error(
        `[CurrentProxyCard] Auto delay check failed, group: ${groupName}, proxy: ${proxyName}`,
        error,
      )
    } finally {
      autoCheckInProgressRef.current = false
      refreshProxy()
      onDelayCheckComplete?.()
    }
  }, [
    refreshProxy,
    currentGroup,
    currentProxy,
    onDelayCheckComplete,
  ])

  useEffect(() => {
    if (!autoDelayEnabled) return
    if (!currentGroup || !currentProxy) return

    let disposed = false
    let intervalTimer: ReturnType<typeof setTimeout> | null = null
    let initialTimer: ReturnType<typeof setTimeout> | null = null

    const runAndSchedule = async () => {
      if (disposed) return
      await checkCurrentProxyDelay()
      if (disposed) return
      intervalTimer = setTimeout(runAndSchedule, autoDelayIntervalMs)
    }

    initialTimer = setTimeout(async () => {
      await checkCurrentProxyDelay()
      if (disposed) return
      intervalTimer = setTimeout(runAndSchedule, autoDelayIntervalMs)
    }, AUTO_CHECK_INITIAL_DELAY_MS)

    return () => {
      disposed = true
      if (initialTimer) clearTimeout(initialTimer)
      if (intervalTimer) clearTimeout(intervalTimer)
    }
  }, [
    checkCurrentProxyDelay,
    autoDelayIntervalMs,
    currentGroup,
    currentProxy,
    autoDelayEnabled,
  ])

  const handleCheckAllDelay = useLockFn(async (isGlobalMode: boolean) => {
    const groupName = currentGroup
    if (!groupName) return

    debugLog(`[CurrentProxyCard] Start delay check, group: ${groupName}`)

    const timeout = defaultLatencyTimeout || 10000
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
    checkCurrentProxyDelay,
    handleCheckAllDelay,
  }
}
