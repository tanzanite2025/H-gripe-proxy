/**
 * 核心协调器服务
 */

import { invoke } from '@tauri-apps/api/core'

import type { BlackholeBreakerConfig } from '@/services/blackhole-breaker'
import type { ResolvedEgressIdentity } from '@/services/egress-identity'
import type { IpReputationConfig } from '@/services/ip-reputation'
import type { MultipathConfig } from '@/services/multipath'
import type { BindingInfo, SessionAffinityConfig } from '@/services/session-affinity'
import type { TimezoneSpoofConfig } from '@/services/timezone-spoof'

export type {
  MultipathConfig,
  NodePool,
  PathNode,
  PoolType,
  SessionBinding,
  SlicingStrategy,
} from '@/services/multipath'
export type {
  ConnectionBindingConfig,
  DomainBindingRule,
  ProcessBindingRule,
  SessionAffinityConfig,
} from '@/services/session-affinity'

/**
 * 协调器状态
 */
export interface CoordinatorResolvedEgressIdentity extends ResolvedEgressIdentity {
  sourceGroupName?: string | null
  sourceGroupSelectedNode?: string | null
}

export interface CoordinatorBindingInfo extends BindingInfo {
  sourceGroupName?: string | null
  sourceGroupSelectedNode?: string | null
}

export interface StableEgressBackwriteStatus {
  domainPatternAssignments: CoordinatorResolvedEgressIdentity[]
  domainRuleBindings: CoordinatorBindingInfo[]
}

export interface CoordinatorRuntimeState {
  egressIdentityAssignments: CoordinatorResolvedEgressIdentity[]
  sessionAffinityBindings: CoordinatorBindingInfo[]
  stableEgressBackwrite: StableEgressBackwriteStatus
}

export interface CoordinatorStatus {
  initialized: boolean
  securityEnabled: boolean
  securityCompromised: boolean
  antiProbeEnabled: boolean
  tlsFingerprint: string | null
  egressIdentityEnabled: boolean
  sessionAffinityEnabled: boolean
  egressIdentityActiveAssignments: number
  sessionAffinityActiveBindings: number
  runtimeState: CoordinatorRuntimeState
  multipathEnabled: boolean
  xdpEnabled?: boolean
  xdpRunning?: boolean
}

/**
 * 高级配置
 */
export interface AdvancedConfig {
  security: SecurityConfig
  multipath: MultipathConfig
  session_affinity: SessionAffinityConfig
  egress_identity: EgressIdentityConfig
  egress_monitor: EgressMonitorConfig
  dns: AdvancedDnsConfig
  security_policies: ISecurityPolicy[]
  residential_pool: ResidentialProxyPool
  ip_reputation: IpReputationConfig
  blackhole_breaker: BlackholeBreakerConfig
  timezone_spoof: TimezoneSpoofConfig
  xdp?: XdpConfig
}

export interface SecurityConfig {
  enabled: boolean
  anti_probe: AntiProbeConfig
  tls_fingerprint: string | null
  config_decoy: ConfigDecoyConfig
  sniffer: SnifferConfig
  obfuscation: ObfuscationConfig
}

export interface SnifferConfig {
  enabled: boolean
  overrideDest: boolean
  forceDomain: string[]
  skipDomain: string[]
  parsePureIp: boolean
  forceDnsMapping: boolean
  sniffing: string[]
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

export type ResidentialProxyType = 'socks5' | 'http' | 'ss' | 'vmess' | 'trojan'

export interface ResidentialProxy {
  name: string
  proxyType: ResidentialProxyType
  server: string
  port: number
  username?: string
  password?: string
  cipher?: string
  uuid?: string
  trojanPassword?: string
  tls?: boolean
  sni?: string
  skipCertVerify?: boolean
  region?: string
  enabled: boolean
}

export interface ResidentialProxyPool {
  enabled: boolean
  proxies: ResidentialProxy[]
}

export type ObfuscationLevel = 'none' | 'low' | 'medium' | 'high' | 'paranoid'

export interface ObfuscationConfig {
  enabled: boolean
  level: ObfuscationLevel
  autoAdjust: boolean
}

export type IpType = 'Datacenter' | 'Residential' | 'Mobile' | 'Unknown'

export interface AdvancedDnsConfig {
  enable_cache: boolean
  enable_prefetch: boolean
  enable_health_check: boolean
  prefetch_interval: number
  health_check_interval: number
  routing_mode: DnsRoutingMode
  leak_protection_level: DnsLeakProtectionLevel
}

export type DnsRoutingMode = 'speed' | 'privacy' | 'balanced' | 'custom'

export type DnsLeakProtectionLevel = 'none' | 'basic' | 'strict' | 'paranoid'

export interface EgressIdentityConfig {
  enabled: boolean
  default_profile: string | null
  profiles: EgressIdentityProfile[]
  app_rules: AppEgressRule[]
  shortcut_rules: ShortcutEgressRule[]
}

export interface EgressIdentityProfile {
  id: string
  name: string
  enabled: boolean
  preferred_nodes: string[]
  preferred_pools: string[]
  required_ip_type: IpType | null
  max_fraud_score: number | null
  dns_policy: DnsPolicy
  tls_fingerprint: string | null
  session_policy: IdentitySessionPolicy
  failover_policy: EgressFailoverPolicy
  allowed_nodes?: string[]
  strict_node_scope?: boolean
  use_residential_chain?: boolean
  residential_proxy_name?: string | null
  description: string
}

export interface AppEgressRule {
  process_name: string | null
  exe_path: string | null
  domains: string[]
  profile_id: string
  priority: number
  enabled: boolean
}

export interface ShortcutEgressRule {
  shortcut_id: string
  profile_id: string
  enabled: boolean
}

export interface DnsPolicy {
  mode: DnsMode
  force_remote_dns: boolean
}

export type DnsMode = 'Inherit' | 'Hijack' | 'Remote'

export interface IdentitySessionPolicy {
  strict_affinity: boolean
  ttl_override: number | null
}

export type EgressFailoverPolicy = 'Block' | 'Manual' | 'AutoSwitch'

export type RebindStrategyType = 'smart' | 'round-robin'

export interface EgressMonitorConfig {
  enabled: boolean
  probeIntervalSecs: number
  autoRebindOnChange: boolean
  notifyOnChange: boolean
  probeTimeoutSecs: number
  watchPollIntervalSecs: number
  watchDebounceSecs: number
  rebindStrategy: RebindStrategyType
}

export interface EgressIpProbeResult {
  ip: string
  countryCode: string | null
  probedAtMs: number
  latencyMs: number
}

export interface EgressIpChangeEvent {
  previousIp: string
  currentIp: string
  previousCountry: string | null
  currentCountry: string | null
  timestampMs: number
  autoRebindApplied: boolean
}

export interface EgressMonitorStats {
  totalProbes: number
  successfulProbes: number
  failedProbes: number
  ipChangeCount: number
  autoRebindCount: number
  lastProbe: EgressIpProbeResult | null
  lastChange: EgressIpChangeEvent | null
  uptimeSecs: number
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