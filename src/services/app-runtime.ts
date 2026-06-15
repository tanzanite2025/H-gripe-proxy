import { invoke } from '@tauri-apps/api/core'

import type { DnsResolverPlan } from './dns-api'

export type AppProcessMatcherKind =
  | 'process_name'
  | 'process_path'
  | 'process_name_regex'
  | 'process_path_regex'
  | 'bundle_id'

export type AppRoutingIntent =
  | 'direct'
  | 'proxy'
  | 'reject'
  | 'auto'
  | 'fallback'

export interface AppEnvironmentVariable {
  key: string
  value: string
}

export interface AppProcessMatcher {
  kind: AppProcessMatcherKind
  pattern: string
}

export interface AppRegistryEntry {
  appId: string
  name: string
  executablePath?: string
  bundleId?: string
  launchArgs: string[]
  workingDirectory?: string
  env: AppEnvironmentVariable[]
  processMatchers: AppProcessMatcher[]
  platformMetadata: Record<string, string>
  tags: string[]
  updatedAt: number
}

export interface NodePoolHealthConstraints {
  maxLatencyMs?: number
  requireAlive?: boolean
  minAvailableNodes?: number
}

export interface NodePoolCandidate {
  nodeName: string
  proxyGroup?: string
  protocol?: string
  region?: string
  tags: string[]
  priority?: number
}

export interface NodePool {
  poolId: string
  name: string
  tags: string[]
  region?: string
  protocols: string[]
  purpose?: string
  costTier?: string
  healthConstraints: NodePoolHealthConstraints
  candidateNodes: NodePoolCandidate[]
  updatedAt: number
}

export interface DnsProfile {
  profileId: string
  name: string
  configYaml: string
  testDomain?: string
  tags: string[]
  updatedAt: number
}

export interface AppPolicyBinding {
  bindingId: string
  appId: string
  nodePoolId?: string
  dnsProfileId?: string
  securityProfileId?: string
  routingIntent: AppRoutingIntent
  enabled: boolean
  updatedAt: number
}

export interface AppRuntimeStateDocument {
  apps: AppRegistryEntry[]
  nodePools: NodePool[]
  dnsProfiles: DnsProfile[]
  policyBindings: AppPolicyBinding[]
}

export interface AppRuntimePlanRequest {
  appId: string
  sessionId?: string
}

export interface NodePoolPlanView {
  poolId: string
  name: string
  candidateCount: number
  protocols: string[]
  tags: string[]
  constraints: NodePoolHealthConstraints
  candidates: NodePoolCandidate[]
}

export interface DnsProfilePlanView {
  profileId: string
  name: string
  testDomain?: string | null
  tags: string[]
  resolverPlan: DnsResolverPlan
}

export interface RuntimeProjectionPlan {
  status: 'planningOnly'
  backend: string
  mutatesRuntime: boolean
  outputs: string[]
}

export interface MihomoRuleProjection {
  matcher: string
  value: string
  target: string
  rule: string
}

export interface MihomoProxyGroupProjection {
  name: string
  type: string
  proxies: string[]
  url?: string
  interval?: number
}

export interface MihomoDnsProjection {
  profileId: string
  name: string
  nameservers: string[]
  runtimeSupportedNameservers: number
}

export interface AppRuntimePlan {
  status: 'ready' | 'rejected'
  reason: string
  appId: string
  sessionId?: string
  app?: AppRegistryEntry
  policyBinding?: AppPolicyBinding
  nodePool?: NodePoolPlanView
  dnsProfile?: DnsProfilePlanView
  routingIntent?: AppRoutingIntent
  projection: RuntimeProjectionPlan
  facts: string[]
  warnings: string[]
}

export interface AppRuntimeMihomoProjection {
  status: 'ready' | 'rejected'
  reason: string
  appId: string
  sessionId?: string
  mutatesRuntime: boolean
  proxyGroups: MihomoProxyGroupProjection[]
  rules: MihomoRuleProjection[]
  dns?: MihomoDnsProjection
  yamlPatch: string
  facts: string[]
  warnings: string[]
}

export async function getAppRuntimeState(): Promise<AppRuntimeStateDocument> {
  return invoke('get_app_runtime_state')
}

export async function upsertAppRegistryEntry(
  entry: AppRegistryEntry,
): Promise<AppRuntimeStateDocument> {
  return invoke('upsert_app_registry_entry', { entry })
}

export async function deleteAppRegistryEntry(
  appId: string,
): Promise<AppRuntimeStateDocument> {
  return invoke('delete_app_registry_entry', { appId })
}

export async function upsertNodePool(
  nodePool: NodePool,
): Promise<AppRuntimeStateDocument> {
  return invoke('upsert_node_pool', { nodePool })
}

export async function deleteNodePool(
  poolId: string,
): Promise<AppRuntimeStateDocument> {
  return invoke('delete_node_pool', { poolId })
}

export async function upsertDnsProfile(
  dnsProfile: DnsProfile,
): Promise<AppRuntimeStateDocument> {
  return invoke('upsert_dns_profile', { dnsProfile })
}

export async function deleteDnsProfile(
  profileId: string,
): Promise<AppRuntimeStateDocument> {
  return invoke('delete_dns_profile', { profileId })
}

export async function upsertAppPolicyBinding(
  binding: AppPolicyBinding,
): Promise<AppRuntimeStateDocument> {
  return invoke('upsert_app_policy_binding', { binding })
}

export async function deleteAppPolicyBinding(
  bindingId: string,
): Promise<AppRuntimeStateDocument> {
  return invoke('delete_app_policy_binding', { bindingId })
}

export async function explainAppRuntimePlan(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimePlan> {
  return invoke('explain_app_runtime_plan', { request })
}

export async function projectAppRuntimePlanToMihomo(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimeMihomoProjection> {
  return invoke('project_app_runtime_plan_to_mihomo', { request })
}
