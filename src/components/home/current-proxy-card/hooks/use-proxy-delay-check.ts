import { useLockFn } from 'ahooks'
import { useCallback, useEffect, useRef } from 'react'
import { delayGroup, healthcheckProxyProvider } from 'tauri-plugin-mihomo-api'

import { useProxiesData } from '@/providers/app-data-context'
import delayManager from '@/services/delay'
import { debugLog } from '@/utils/misc'

const AUTO_CHECK_DEFAULT_INTERVAL_MINUTES = 5
const AUTO_CHECK_INITIAL_DELAY_MS = 100

interface UseProxyDelayCheckProps {
  currentGroup: string
  currentProxy: string
  currentProxyRecord: any
  isDirectMode: boolean
  autoDelayEnabled: boolean
  autoDelayIntervalMs: number
  defaultLatencyTimeout: number
  proxyRecords: Record<string, any>
  refreshProxy: () => void
  onDelayCheckComplete?: () => void
}

/**
 * 代理延迟检测 Hook
 * 处理手动延迟测试和自动延迟检测
 */
export const useProxyDelayCheck = ({
  currentGroup,
  currentProxy,
  currentProxyRecord,
  isDirectMode,
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

  // 更新最新的超时值
  useEffect(() => {
    latestTimeoutRef.current = defaultLatencyTimeout
  }, [defaultLatencyTimeout])

  // 更新最新的代理记录
  useEffect(() => {
    if (!currentProxy) {
      latestProxyRecordRef.current = null
      return
    }
    latestProxyRecordRef.current = currentProxyRecord || null
  }, [currentProxy, currentProxyRecord])

  /**
   * 检测当前代理的延迟
   */
  const checkCurrentProxyDelay = useCallback(async () => {
    if (autoCheckInProgressRef.current) return
    if (isDirectMode) return

    const groupName = currentGroup
    const proxyName = currentProxy

    if (!groupName || !proxyName) return

    const proxyRecord = latestProxyRecordRef.current
    if (!proxyRecord) {
      debugLog(
        `[CurrentProxyCard] 自动延迟检测跳过，组: ${groupName}, 节点: ${proxyName} 未找到`,
      )
      return
    }

    autoCheckInProgressRef.current = true

    const timeout = latestTimeoutRef.current || 10000

    try {
      debugLog(
        `[CurrentProxyCard] 自动检测当前节点延迟，组: ${groupName}, 节点: ${proxyName}`,
      )
      if (proxyRecord.provider) {
        await healthcheckProxyProvider(proxyRecord.provider)
      } else {
        await delayManager.checkDelay(proxyName, groupName, timeout)
      }
    } catch (error) {
      console.error(
        `[CurrentProxyCard] 自动检测当前节点延迟失败，组: ${groupName}, 节点: ${proxyName}`,
        error,
      )
    } finally {
      autoCheckInProgressRef.current = false
      refreshProxy()
      onDelayCheckComplete?.()
    }
  }, [
    isDirectMode,
    refreshProxy,
    currentGroup,
    currentProxy,
    onDelayCheckComplete,
  ])

  /**
   * 自动延迟检测定时器
   */
  useEffect(() => {
    if (isDirectMode) return
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
    isDirectMode,
    currentGroup,
    currentProxy,
    autoDelayEnabled,
  ])

  /**
   * 手动检测所有代理的延迟
   */
  const handleCheckAllDelay = useLockFn(async (isGlobalMode: boolean) => {
    const groupName = currentGroup
    if (!groupName || isDirectMode) return

    debugLog(`[CurrentProxyCard] 开始测试所有延迟，组: ${groupName}`)

    const timeout = defaultLatencyTimeout || 10000

    // 获取当前组的所有代理
    const proxyNames: string[] = []
    const providers: Set<string> = new Set()

    if (isGlobalMode && proxies?.global) {
      // 全局模式
      const allProxies = proxies.global.all
        .filter((p: any) => {
          const name = typeof p === 'string' ? p : p.name
          return name !== 'DIRECT' && name !== 'REJECT'
        })
        .map((p: any) => (typeof p === 'string' ? p : p.name))

      allProxies.forEach((name: string) => {
        const proxy = proxyRecords[name]
        if (proxy?.provider) {
          providers.add(proxy.provider)
        } else {
          proxyNames.push(name)
        }
      })
    } else {
      // 规则模式 - 需要从外部传入组信息
      // 这里简化处理，实际使用时需要传入完整的组数据
      debugLog(`[CurrentProxyCard] 规则模式延迟测试需要组数据`)
    }

    debugLog(
      `[CurrentProxyCard] 找到代理数量: ${proxyNames.length}, 提供者数量: ${providers.size}`,
    )

    // 测试提供者的节点
    if (providers.size > 0) {
      debugLog(`[CurrentProxyCard] 开始测试提供者节点`)
      await Promise.allSettled(
        [...providers].map((p) => healthcheckProxyProvider(p)),
      )
    }

    // 测试非提供者的节点
    if (proxyNames.length > 0) {
      const url = delayManager.getUrl(groupName)
      debugLog(`[CurrentProxyCard] 测试URL: ${url}, 超时: ${timeout}ms`)

      try {
        await Promise.race([
          delayManager.checkListDelay(proxyNames, groupName, timeout),
          delayGroup(groupName, url, timeout),
        ])
        debugLog(`[CurrentProxyCard] 延迟测试完成，组: ${groupName}`)
      } catch (error) {
        console.error(
          `[CurrentProxyCard] 延迟测试出错，组: ${groupName}`,
          error,
        )
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
