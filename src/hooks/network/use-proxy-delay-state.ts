import { useLockFn } from 'ahooks'
import { useCallback, useEffect, useReducer, useRef } from 'react'

import { useVerge } from '@/hooks/system'
import delayManager, { type DelayUpdate } from '@/services/delay'
import { resolveVergeDelayTimeout } from '@/services/delay-config'
import { resolveDisplayDelayUpdate } from '@/services/delay-display'
import type { IProxyItem } from '@/types/proxy'
const PRESET_PROXY_NAMES = [
  'DIRECT',
  'REJECT',
  'REJECT-DROP',
  'PASS',
  'COMPATIBLE',
]

const identity = (_: DelayUpdate, next: DelayUpdate): DelayUpdate => next

const INITIAL_DELAY: DelayUpdate = { delay: -1, updatedAt: 0 }

export interface UseProxyDelayState {
  delayState: DelayUpdate
  delayValue: number
  isPreset: boolean
  timeout: number
  onDelay: () => Promise<void>
}

export function useProxyDelayState(
  proxy: IProxyItem,
  groupName: string,
): UseProxyDelayState {
  const isPreset = PRESET_PROXY_NAMES.includes(proxy.name)
  const [delayState, setDelayState] = useReducer(identity, INITIAL_DELAY)
  const { verge } = useVerge()
  const timeout = resolveVergeDelayTimeout(verge)
  const latestProxyRef = useRef(proxy)

  useEffect(() => {
    latestProxyRef.current = proxy
  }, [proxy])

  const applyDisplayUpdate = useCallback((update: DelayUpdate | undefined) => {
    const resolved = resolveDisplayDelayUpdate(update, latestProxyRef.current)
    setDelayState(resolved ? { ...resolved } : INITIAL_DELAY)
  }, [])

  useEffect(() => {
    if (isPreset) return
    delayManager.setListener(proxy.name, groupName, applyDisplayUpdate)
    return () => {
      delayManager.removeListener(proxy.name, groupName)
    }
  }, [applyDisplayUpdate, groupName, isPreset, proxy.name])

  const updateDelay = useCallback(() => {
    applyDisplayUpdate(delayManager.getDelayUpdate(proxy.name, groupName))
  }, [applyDisplayUpdate, groupName, proxy.name])

  useEffect(() => {
    updateDelay()
  }, [updateDelay])

  const onDelay = useLockFn(async () => {
    setDelayState({ delay: -2, updatedAt: Date.now() })
    applyDisplayUpdate(
      await delayManager.checkDelay(proxy.name, groupName, timeout),
    )
  })

  return {
    delayState,
    delayValue: delayState.delay,
    isPreset,
    timeout,
    onDelay,
  }
}
