/**
 * Rust 连接指标监控服务
 *
 * Manages the Rust ConnectionMonitorController lifecycle (ref-counted)
 * and broadcasts `verge://connection-metrics` events to subscribers.
 */

import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface ConnectionSpeed {
  id: string
  curUpload: number
  curDownload: number
}

export interface TrafficSnapshot {
  uploadTotal: number
  downloadTotal: number
  uploadSpeed: number
  downloadSpeed: number
  activeConnectionCount: number
  closedSinceLast: number
  memory: number
}

export interface ConnectionMetricsSnapshot {
  traffic: TrafficSnapshot
  speeds: ConnectionSpeed[]
  stale: boolean
}

export interface ConnectionMetricsEventPayload {
  metrics: ConnectionMetricsSnapshot
  raw: IConnections
}

// ── commands ─────────────────────────────────────────────────────────

async function connectionMonitorStart(): Promise<void> {
  await invoke('connection_monitor_start')
}

async function connectionMonitorStop(): Promise<void> {
  await invoke('connection_monitor_stop')
}

export async function connectionMonitorIsRunning(): Promise<boolean> {
  return await invoke<boolean>('connection_monitor_is_running')
}

export async function getConnectionMetricsSnapshot(): Promise<ConnectionMetricsSnapshot> {
  return await invoke<ConnectionMetricsSnapshot>(
    'traffic_get_connection_metrics_snapshot',
  )
}

export async function resetConnectionMetrics(): Promise<void> {
  await invoke('traffic_reset_connection_metrics')
}

// ── ref-counted monitor lifecycle + pub/sub ──────────────────────────

type MetricsHandler = (payload: ConnectionMetricsEventPayload) => void

const listeners = new Set<MetricsHandler>()
let monitorRefCount = 0
let cleanupFn: (() => void) | null = null

function acquireMonitor() {
  monitorRefCount++
  if (monitorRefCount === 1) {
    void connectionMonitorStart()
    const unlistenPromise = listen<ConnectionMetricsEventPayload>(
      'verge://connection-metrics',
      (event) => {
        for (const handler of listeners) {
          handler(event.payload)
        }
      },
    )
    cleanupFn = () => {
      void unlistenPromise.then((fn: UnlistenFn) => fn())
      void connectionMonitorStop()
    }
  }
}

function releaseMonitor() {
  monitorRefCount--
  if (monitorRefCount <= 0) {
    monitorRefCount = 0
    cleanupFn?.()
    cleanupFn = null
  }
}

/**
 * Subscribe to Rust connection metrics events.
 * Automatically starts the monitor on first subscriber
 * and stops it when the last subscriber unsubscribes.
 * Returns an unsubscribe function.
 */
export function subscribeMetrics(handler: MetricsHandler): () => void {
  listeners.add(handler)
  acquireMonitor()
  return () => {
    listeners.delete(handler)
    releaseMonitor()
  }
}
