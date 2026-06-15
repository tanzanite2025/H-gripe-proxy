/**
 * Rust 连接指标监控服务
 *
 * Wraps the Tauri commands and event listener for the Rust-side
 * `ConnectionMetricsAggregator` / `ConnectionMonitorController`.
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

export async function connectionMonitorStart(): Promise<void> {
  await invoke('connection_monitor_start')
}

export async function connectionMonitorStop(): Promise<void> {
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

export function onConnectionMetrics(
  handler: (payload: ConnectionMetricsEventPayload) => void,
): Promise<UnlistenFn> {
  return listen<ConnectionMetricsEventPayload>(
    'verge://connection-metrics',
    (event) => handler(event.payload),
  )
}
