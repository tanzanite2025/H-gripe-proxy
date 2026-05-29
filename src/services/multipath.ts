/**
 * 多路径路由服务
 */

import { invoke } from '@tauri-apps/api/core'

export type SlicingStrategy =
  | 'RoundRobin'
  | 'Random'
  | 'Weighted'
  | 'LeastConnections'
  | 'LatencyBased'

export type PoolType =
  | 'General'
  | 'Streaming'
  | 'Gaming'
  | 'Download'
  | 'Social'

export interface MultipathConfig {
  enabled: boolean
  strategy: SlicingStrategy
  node_pools: NodePool[]
  min_fragment_size: number
  max_fragment_size: number
  reassembly_timeout: number
  session_persistence: boolean
  bindings: SessionBinding[]
}

export interface NodePool {
  name: string
  pool_type: PoolType
  nodes: PathNode[]
  enabled: boolean
}

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

export interface SessionBinding {
  domain_pattern: string
  pool_type: PoolType
  force_single_node: boolean
  description: string
}

export interface TestResult {
  success: boolean
  latency: number
  message: string
}

export interface ImportResult {
  success: boolean
  imported_count: number
  message: string
}

export async function multipathGetConfig(): Promise<MultipathConfig> {
  return invoke<MultipathConfig>('multipath_get_config')
}

export async function multipathUpdateConfig(
  config: MultipathConfig,
): Promise<void> {
  return invoke<void>('multipath_update_config', { config })
}

export async function multipathGetBindings(): Promise<SessionBinding[]> {
  return invoke<SessionBinding[]>('multipath_get_bindings')
}

export async function multipathAddBinding(
  binding: SessionBinding,
): Promise<void> {
  return invoke<void>('multipath_add_binding', { binding })
}

export async function multipathRemoveBinding(
  domainPattern: string,
): Promise<void> {
  return invoke<void>('multipath_remove_binding', { domainPattern })
}

export async function multipathGetPredefinedBindings(): Promise<
  SessionBinding[]
> {
  return invoke<SessionBinding[]>('multipath_get_predefined_bindings')
}

export async function multipathAddPool(pool: NodePool): Promise<void> {
  return invoke<void>('multipath_add_pool', { pool })
}

export async function multipathRemovePool(poolName: string): Promise<void> {
  return invoke<void>('multipath_remove_pool', { poolName })
}

export async function multipathUpdatePool(pool: NodePool): Promise<void> {
  return invoke<void>('multipath_update_pool', { pool })
}

export async function multipathAddNode(
  poolName: string,
  node: PathNode,
): Promise<void> {
  return invoke<void>('multipath_add_node', { poolName, node })
}

export async function multipathRemoveNode(
  poolName: string,
  nodeName: string,
): Promise<void> {
  return invoke<void>('multipath_remove_node', { poolName, nodeName })
}

export async function multipathTestNode(node: PathNode): Promise<TestResult> {
  return invoke<TestResult>('multipath_test_node', { node })
}

export async function multipathImportNodes(
  poolName: string,
  nodesYaml: string,
): Promise<ImportResult> {
  return invoke<ImportResult>('multipath_import_nodes', {
    poolName,
    nodesYaml,
  })
}

export async function multipathExportNodes(poolName: string): Promise<string> {
  return invoke<string>('multipath_export_nodes', { poolName })
}

export async function multipathGetRecommendedConfig(): Promise<MultipathConfig> {
  return invoke<MultipathConfig>('multipath_get_recommended_config')
}
