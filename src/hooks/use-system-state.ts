import { useQuery } from '@tanstack/react-query'
import { useEffect, useRef, useState } from 'react'

import { getRunningMode, isAdmin, isServiceAvailable, stopCore } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import { useVerge } from './use-verge'

export interface SystemState {
  runningMode: 'Sidecar' | 'Service' | 'NotRunning'
  isAdminMode: boolean
  isServiceOk: boolean
}

const defaultSystemState = {
  runningMode: 'Sidecar',
  isAdminMode: false,
  isServiceOk: false,
} as SystemState

// Grace period for service initialization during startup
const STARTUP_GRACE_MS = 10_000

/**
 * 自定义 hook 用于获取系统运行状态
 * 包括运行模式、管理员状态、系统服务是否可用
 */
export function useSystemState() {
  const { verge } = useVerge()
  const enforcingFailClosedRef = useRef(false)
  const [isStartingUp, setIsStartingUp] = useState(true)
  const enable_tun_mode = verge?.enable_tun_mode

  useEffect(() => {
    const timer = setTimeout(() => setIsStartingUp(false), STARTUP_GRACE_MS)
    return () => clearTimeout(timer)
  }, [])

  const {
    data: systemState = defaultSystemState,
    refetch: mutateSystemState,
    isLoading,
  } = useQuery({
    queryKey: ['getSystemState'],
    queryFn: async () => {
      const [runningMode, isAdminMode, isServiceOk] = await Promise.all([
        getRunningMode(),
        isAdmin(),
        isServiceAvailable(),
      ])
      return { runningMode, isAdminMode, isServiceOk } as SystemState
    },
    refetchInterval: isStartingUp ? 2000 : enable_tun_mode ? 5000 : 30000,
  })

  const isSidecarMode = systemState.runningMode === 'Sidecar'
  const isServiceMode = systemState.runningMode === 'Service'
  const isNotRunningMode = systemState.runningMode === 'NotRunning'
  const isTunModeAvailable = systemState.isAdminMode || systemState.isServiceOk

  const cooldownTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  useEffect(() => {
    if (enable_tun_mode === undefined) return

    if (
      !enforcingFailClosedRef.current &&
      enable_tun_mode &&
      !isTunModeAvailable &&
      !isLoading &&
      !isStartingUp &&
      !isNotRunningMode
    ) {
      enforcingFailClosedRef.current = true
      stopCore()
        .catch((err) => {
          console.error('[useSystemState] TUN fail-closed stop core failed:', err)
        })
        .finally(() => {
          showNotice.error(
            'TUN protection unavailable. Core has been stopped to avoid traffic leaks. Repair the service or run as administrator.',
          )
          void mutateSystemState()
          cooldownTimerRef.current = setTimeout(() => {
            enforcingFailClosedRef.current = false
            cooldownTimerRef.current = null
          }, 1000)
        })
    }

    return () => {
      if (cooldownTimerRef.current != null) {
        clearTimeout(cooldownTimerRef.current)
        cooldownTimerRef.current = null
        enforcingFailClosedRef.current = false
      }
    }
  }, [
    enable_tun_mode,
    isTunModeAvailable,
    isLoading,
    isStartingUp,
    isNotRunningMode,
    mutateSystemState,
  ])

  return {
    runningMode: systemState.runningMode,
    isAdminMode: systemState.isAdminMode,
    isServiceOk: systemState.isServiceOk,
    isSidecarMode,
    isServiceMode,
    isNotRunningMode,
    isTunModeAvailable,
    mutateSystemState,
    isLoading,
  }
}
