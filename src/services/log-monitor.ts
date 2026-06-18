import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

import type { LogLevel } from '@/types/mihomo'

type CoreLogHandler = (payload: unknown) => void

const listeners = new Set<CoreLogHandler>()
let monitorRefCount = 0
let activeLevel: LogLevel | null = null
let cleanupListener: (() => void) | null = null

async function logMonitorStart(level: LogLevel): Promise<void> {
  await invoke('log_monitor_start', { level })
}

async function logMonitorStop(): Promise<void> {
  await invoke('log_monitor_stop')
}

function attachListener(level: LogLevel) {
  cleanupListener?.()
  activeLevel = level
  const unlistenPromise = listen<unknown>('verge://core-log', (event) => {
    for (const handler of listeners) {
      handler(event.payload)
    }
  })
  cleanupListener = () => {
    void unlistenPromise.then((fn: UnlistenFn) => fn())
  }
  void logMonitorStart(level)
}

function acquireMonitor(level: LogLevel) {
  monitorRefCount++
  if (monitorRefCount === 1 || activeLevel !== level) {
    attachListener(level)
  }
}

function releaseMonitor() {
  monitorRefCount--
  if (monitorRefCount <= 0) {
    monitorRefCount = 0
    activeLevel = null
    cleanupListener?.()
    cleanupListener = null
    void logMonitorStop()
  }
}

export function subscribeCoreLogs(
  level: LogLevel,
  handler: CoreLogHandler,
): () => void {
  listeners.add(handler)
  acquireMonitor(level)
  return () => {
    listeners.delete(handler)
    releaseMonitor()
  }
}
