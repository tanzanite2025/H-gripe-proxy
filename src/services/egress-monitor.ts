/**
 * 出口 IP 监控服务
 */

import { invoke } from '@tauri-apps/api/core'

import type {
  EgressMonitorConfig,
  EgressMonitorStats,
  EgressIpProbeResult,
} from '@/services/coordinator'

export type {
  EgressMonitorConfig,
  EgressMonitorStats,
  EgressIpProbeResult,
  EgressIpChangeEvent,
  RebindStrategyType,
} from '@/services/coordinator'

/**
 * 获取出口监控配置
 */
export async function egressMonitorGetConfig(): Promise<EgressMonitorConfig> {
  return await invoke<EgressMonitorConfig>('egress_monitor_get_config')
}

/**
 * 更新出口监控配置
 */
export async function egressMonitorUpdateConfig(
  config: EgressMonitorConfig,
): Promise<void> {
  await invoke('egress_monitor_update_config', { config })
}

/**
 * 启动出口监控
 */
export async function egressMonitorStart(): Promise<void> {
  await invoke('egress_monitor_start')
}

/**
 * 停止出口监控
 */
export async function egressMonitorStop(): Promise<void> {
  await invoke('egress_monitor_stop')
}

/**
 * 获取出口监控统计
 */
export async function egressMonitorGetStats(): Promise<EgressMonitorStats> {
  return await invoke<EgressMonitorStats>('egress_monitor_get_stats')
}

/**
 * 重置出口监控统计
 */
export async function egressMonitorResetStats(): Promise<void> {
  await invoke('egress_monitor_reset_stats')
}

/**
 * 手动探测出口 IP
 */
export async function egressMonitorProbeNow(): Promise<EgressIpProbeResult> {
  return await invoke<EgressIpProbeResult>('egress_monitor_probe_now')
}

/**
 * 查询出口监控是否运行中
 */
export async function egressMonitorIsRunning(): Promise<boolean> {
  return await invoke<boolean>('egress_monitor_is_running')
}
