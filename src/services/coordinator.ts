/**
 * 核心协调器服务
 */

import { invoke } from '@tauri-apps/api/core'

/**
 * 协调器配置
 */
export interface CoordinatorConfig {
  security_enabled: boolean
  anti_probe_enabled: boolean
  tls_fingerprint: string | null
  multipath_enabled: boolean
  xdp_enabled?: boolean
}

/**
 * 协调器状态
 */
export interface CoordinatorStatus {
  initialized: boolean
  security_enabled: boolean
  security_compromised: boolean
  anti_probe_enabled: boolean
  tls_fingerprint: string | null
  multipath_enabled: boolean
  xdp_enabled?: boolean
  xdp_running?: boolean
}

/**
 * 高级配置
 */
export interface AdvancedConfig {
  security: SecurityConfig
  multipath: MultipathConfig
  xdp?: XdpConfig
}

export interface SecurityConfig {
  enabled: boolean
  anti_probe: AntiProbeConfig
  tls_fingerprint: string | null
  config_decoy: ConfigDecoyConfig
}

export interface AntiProbeConfig {
  enabled: boolean
  secret_key: string
  time_window: number
  whitelist: string[]
  strict_mode: boolean
}

export interface ConfigDecoyConfig {
  enabled: boolean
  decoy_path: string | null
}

export interface MultipathConfig {
  enabled: boolean
  strategy: SlicingStrategy
  node_pools: NodePool[]
  min_fragment_size: number
  max_fragment_size: number
  reassembly_timeout: number
  session_persistence: boolean
}

export type SlicingStrategy = 
  | 'RoundRobin'
  | 'Random'
  | 'Weighted'
  | 'LeastConnections'
  | 'LatencyBased'

export interface NodePool {
  name: string
  pool_type: PoolType
  nodes: PathNode[]
  enabled: boolean
}

export type PoolType = 
  | 'General'
  | 'Streaming'
  | 'Gaming'
  | 'Download'
  | 'Social'

export interface PathNode {
  name: string
  server: string
  port: number
  protocol: string
  weight: number
  enabled: boolean
  location?: string
  max_connections?: number
}

export interface XdpConfig {
  enabled: boolean
  interface: string
  mode: XdpMode
  queue_size: number
}

export type XdpMode = 'Native' | 'Skb' | 'Generic'

/**
 * 初始化协调器
 */
export async function coordinatorInitialize(): Promise<void> {
  await invoke('coordinator_initialize')
}

/**
 * 获取协调器配置
 */
export async function coordinatorGetConfig(): Promise<CoordinatorConfig> {
  return await invoke('coordinator_get_config')
}

/**
 * 更新协调器配置
 */
export async function coordinatorUpdateConfig(config: CoordinatorConfig): Promise<void> {
  await invoke('coordinator_update_config', { config })
}

/**
 * 关闭协调器
 */
export async function coordinatorShutdown(): Promise<void> {
  await invoke('coordinator_shutdown')
}

/**
 * 获取高级配置
 */
export async function getAdvancedConfig(): Promise<AdvancedConfig> {
  return await invoke('get_advanced_config')
}

/**
 * 保存高级配置
 */
export async function saveAdvancedConfig(config: AdvancedConfig): Promise<void> {
  await invoke('save_advanced_config', { config })
}

/**
 * 获取推荐配置
 */
export async function getRecommendedAdvancedConfig(): Promise<AdvancedConfig> {
  return await invoke('get_recommended_advanced_config')
}

/**
 * 验证高级配置
 */
export async function validateAdvancedConfig(config: AdvancedConfig): Promise<void> {
  await invoke('validate_advanced_config', { config })
}

/**
 * 获取协调器状态
 */
export async function coordinatorGetStatus(): Promise<CoordinatorStatus> {
  return await invoke('coordinator_get_status')
}
