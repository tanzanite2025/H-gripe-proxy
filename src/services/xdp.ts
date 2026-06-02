/**
 * XDP 代理服务
 */

import { invoke } from '@tauri-apps/api/core'

export interface XdpConfig {
  enabled: boolean
  interface: string
  mode: XdpMode
  queue_size: number
}

export type XdpMode = 'Native' | 'Skb' | 'Generic'

export interface XdpStatus {
  running: boolean
  interface: string
  mode: XdpMode
  stats: XdpStats
}

export interface XdpStats {
  total_packets: number
  proxied_packets: number
  direct_packets: number
  rejected_packets: number
  errors: number
  bytes_processed: number
}

export interface XdpRoute {
  dest_ip: string
  action: 'Pass' | 'Proxy' | 'Reject'
  proxy_ip?: string
  proxy_port?: number
}

export interface XdpSupportInfo {
  kernel_version: string
  xdp_supported: boolean
  native_mode_supported: boolean
  hw_mode_supported: boolean
  available_interfaces: string[]
}

/**
 * 获取 XDP 配置
 */
export async function xdpGetConfig(): Promise<XdpConfig> {
  return invoke<XdpConfig>('xdp_get_config')
}

/**
 * 更新 XDP 配置
 */
export async function xdpUpdateConfig(config: XdpConfig): Promise<void> {
  return invoke<void>('xdp_update_config', { config })
}

/**
 * 获取 XDP 状态
 */
export async function xdpGetStatus(): Promise<XdpStatus> {
  return invoke<XdpStatus>('xdp_get_status')
}

/**
 * 启动 XDP 代理
 */
export async function xdpStart(): Promise<void> {
  return invoke<void>('xdp_start')
}

/**
 * 停止 XDP 代理
 */
export async function xdpStop(): Promise<void> {
  return invoke<void>('xdp_stop')
}

/**
 * 添加路由规则
 */
export async function xdpAddRoute(route: XdpRoute): Promise<void> {
  return invoke<void>('xdp_add_route', { route })
}

/**
 * 删除路由规则
 */
export async function xdpRemoveRoute(destIp: string): Promise<void> {
  return invoke<void>('xdp_remove_route', { destIp })
}

/**
 * 更新统计信息
 */
export async function xdpUpdateStats(): Promise<void> {
  return invoke<void>('xdp_update_stats')
}

/**
 * 检查系统支持
 */
export async function xdpCheckSupport(): Promise<XdpSupportInfo> {
  return invoke<XdpSupportInfo>('xdp_check_support')
}

/**
 * 获取可用网卡列表
 */
export async function xdpGetInterfaces(): Promise<string[]> {
  return invoke<string[]>('xdp_get_interfaces')
}
