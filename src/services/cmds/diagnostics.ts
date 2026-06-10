import { invoke } from '@tauri-apps/api/core'

import { normalizeIpReputation, type IpReputation } from '@/services/ip-reputation'

export interface DnsRuntimeSnapshot {
  enhanced_mode: string | null
  ipv6: boolean | null
  nameserver_count: number
  fallback_count: number
  nameserver_policy_count: number
  use_hosts: boolean | null
  use_system_hosts: boolean | null
  respect_rules: boolean | null
}

export interface DnsRuntimeDerivedState {
  routing_mode: string | null
  domestic_dns: string[]
  foreign_dns: string[]
  default_nameserver_count: number
  default_nameserver_plain_count: number
  prefer_h3: boolean | null
  leak_protection_level: string | null
  leak_protection_security: string | null
  leak_protection_safe: boolean | null
}

export interface DnsRuntimeStatus {
  enable_dns_settings: boolean
  dns_config_exists: boolean
  dns_config_valid: boolean
  runtime_has_dns: boolean
  runtime_has_hosts: boolean
  runtime_dns_matches_saved: boolean
  runtime_hosts_matches_saved: boolean
  runtime_matches_saved: boolean
  snapshot: DnsRuntimeSnapshot
  derived: DnsRuntimeDerivedState
}

export async function getDnsRuntimeStatus() {
  return invoke<DnsRuntimeStatus>('get_dns_runtime_status')
}

export interface DnsLeakServer {
  ip: string
  hostname: string | null
  country: string | null
  city: string | null
  isp: string | null
}

export interface DnsMetrics {
  cache: {
    hit: number
    miss: number
    size: number
    hitRate: number
  }
  queries: {
    total: number
    success: number
    failed: number
    avgLatencyUs: number
    maxLatencyUs: number
  }
  servers: Array<{
    server: string
    queries: number
    successes: number
    failures: number
    avgLatencyUs: number
    lastQuery: string
    lastError?: string | null
  }>
  recent: Array<{
    domain: string
    qType: string
    server: string
    protocol: string
    proxyName?: string | null
    proxyChain?: string | null
    egress?: string | null
    rule?: string | null
    rulePayload?: string | null
    success: boolean
    error?: string | null
    latencyUs: number
    timestamp: string
  }>
  pollution: {
    totalChecked: number
    pollutedCount: number
    pollutionRate: number
    recentPolluted: Array<{
      domain: string
      ip: string
      reason: string
      timestamp: string
    }>
  }
  trust: {
    total: number
    encrypted: number
    unencrypted: number
    byTrustLevel: Record<string, number>
    servers: Array<{
      address: string
      protocol: string
      trustLevel: string
      encrypted: boolean
      description?: string | null
    }>
    leakRiskScore: number
    lastEvaluated: string
  }
}

export interface DnsLeakTestResult {
  has_leak: boolean
  observed_leak: boolean
  runtime_risk_detected: boolean
  observation_incomplete: boolean
  confidence: 'high' | 'medium' | 'low' | string
  assessment: 'safe' | 'observed-leak' | 'runtime-risk' | 'inconclusive' | string
  leak_type: string[]
  observed_leak_type: string[]
  runtime_risk_type: string[]
  warnings: string[]
  recommendations: string[]
  dns_servers: DnsLeakServer[]
  dns_metrics: DnsMetrics | null
  dns_location: string | null
  ip_location: string
  location_match: boolean
  location_comparable: boolean
  risk_level: 'safe' | 'warning' | 'danger' | string
  timestamp: number
  checked_via_core_proxy: boolean
  observation_path: 'core-proxy' | 'core-proxy-fallback-direct' | 'direct' | string
  error: string | null
}

export async function testDnsLeak() {
  return invoke<DnsLeakTestResult>('test_dns_leak')
}

export interface ProxyDetectionLocation {
  country_code: string | null
  country: string | null
  region: string | null
  city: string | null
  organization: string | null
  asn: number | null
  asn_organization: string | null
}

export interface ProxyDetectionResult {
  checked: boolean
  core_running: boolean
  direct_observed: boolean
  proxy_observed: boolean
  checked_via_core_proxy: boolean
  proxy_effective: boolean
  ip_changed: boolean
  location_changed: boolean
  observation_incomplete: boolean
  runtime_risk_detected: boolean
  confidence: 'high' | 'medium' | 'low' | string
  assessment: 'effective' | 'same-egress' | 'runtime-risk' | 'inconclusive' | string
  runtime_risk_type: string[]
  warnings: string[]
  recommendations: string[]
  direct_ip: string | null
  proxy_ip: string | null
  direct_location: ProxyDetectionLocation | null
  proxy_location: ProxyDetectionLocation | null
  proxy_reputation: IpReputation | null
  observation_path: 'direct-vs-core-proxy' | 'direct-only' | 'core-proxy-only' | string
  error: string | null
  timestamp: number
}

export async function testProxyDetection() {
  const result = await invoke<ProxyDetectionResult & { proxy_reputation?: unknown }>(
    'test_proxy_detection',
  )

  return {
    ...result,
    proxy_reputation: result.proxy_reputation
      ? normalizeIpReputation(result.proxy_reputation)
      : null,
  }
}

export type CurrentEgressIdentitySource =
  | 'mihomoEgressStatus'
  | 'mihomoProxyProbe'
  | 'unavailable'

export interface CurrentEgressIdentity {
  source: CurrentEgressIdentitySource
  proxy_name: string | null
  proxy_chain: string[]
  egress_ip: string | null
  public_egress_ip: string | null
  country_code: string | null
  timezone: string | null
  proxy_endpoint: string | null
  destination_asn: string | null
  asn_org: string | null
  rule: string | null
  rule_payload: string | null
  egress_source: string | null
  confidence: number | null
  sample_count: number | null
  last_verified_at: string | null
  updated_at: string | null
  reputation: IpReputation | null
  message: string
}

export async function getCurrentEgressIdentity(): Promise<CurrentEgressIdentity> {
  const result = await invoke<CurrentEgressIdentity & { reputation?: unknown }>(
    'get_current_egress_identity',
  )

  return {
    ...result,
    proxy_chain: Array.isArray(result.proxy_chain) ? result.proxy_chain : [],
    reputation: result.reputation ? normalizeIpReputation(result.reputation) : null,
  }
}

export type IdentityConsistencyLevel = 'good' | 'warning' | 'danger' | 'unknown'

export type IdentityConsistencyIssueKind =
  | 'missingPublicEgress'
  | 'lowEgressConfidence'
  | 'highIpRisk'
  | 'dnsLeak'
  | 'dnsRuntimeRisk'
  | 'randomTlsFingerprint'
  | 'missingTlsFingerprint'
  | 'observationIncomplete'

export interface IdentityConsistencyIssue {
  kind: IdentityConsistencyIssueKind
  severity: IdentityConsistencyLevel
  message: string
}

export interface IdentityConsistencyReport {
  score: number
  level: IdentityConsistencyLevel
  issues: IdentityConsistencyIssue[]
  public_egress_ip: string | null
  proxy_chain: string[]
  ip_type: string | null
  residential_state: string | null
  egress_source: string | null
  egress_confidence: number | null
  tls_fingerprint: string | null
  dns_assessment: string | null
}

export interface IdentityConsistencySnapshot {
  observed_at: string
  report: IdentityConsistencyReport
}

export type IdentityConsistencyDriftKind =
  | 'publicEgressIp'
  | 'ipType'
  | 'dnsAssessment'
  | 'tlsFingerprint'

export interface IdentityConsistencyDrift {
  kind: IdentityConsistencyDriftKind
  from: string | null
  to: string | null
  first_observed_at: string
  last_observed_at: string
}

export interface IdentityConsistencyDriftReport {
  stable: boolean
  drift_count: number
  drifts: IdentityConsistencyDrift[]
}

export async function getIdentityConsistencyReport(): Promise<IdentityConsistencyReport> {
  const result = await invoke<IdentityConsistencyReport>(
    'get_identity_consistency_report',
  )

  return {
    ...result,
    proxy_chain: Array.isArray(result.proxy_chain) ? result.proxy_chain : [],
    issues: Array.isArray(result.issues) ? result.issues : [],
  }
}

export async function getIdentityConsistencyHistory(): Promise<IdentityConsistencySnapshot[]> {
  const result = await invoke<IdentityConsistencySnapshot[]>(
    'get_identity_consistency_history',
  )

  return Array.isArray(result)
    ? result.map((snapshot) => ({
        ...snapshot,
        report: {
          ...snapshot.report,
          proxy_chain: Array.isArray(snapshot.report.proxy_chain)
            ? snapshot.report.proxy_chain
            : [],
          issues: Array.isArray(snapshot.report.issues)
            ? snapshot.report.issues
            : [],
        },
      }))
    : []
}

export async function getIdentityConsistencyDriftReport(): Promise<IdentityConsistencyDriftReport> {
  const result = await invoke<IdentityConsistencyDriftReport>(
    'get_identity_consistency_drift_report',
  )

  return {
    ...result,
    drift_count: Number.isFinite(result.drift_count) ? result.drift_count : 0,
    drifts: Array.isArray(result.drifts) ? result.drifts : [],
  }
}

export interface TorRuntimeStatus {
  enabled: boolean
  socks_host: string
  socks_port: number
  control_port: number | null
  use_bridges: boolean
  bridge_count: number
  configured_proxy_url: string
  checked: boolean
  status: 'disabled' | 'connected' | 'failed' | string
  connected: boolean
  circuit_established: boolean
  observation_incomplete: boolean
  runtime_risk_detected: boolean
  confidence: 'high' | 'medium' | 'low' | string
  assessment: 'disabled' | 'connected' | 'runtime-risk' | 'inconclusive' | string
  runtime_risk_type: string[]
  current_ip: string | null
  exit_node: string | null
  check_method: string
  observation_path: string
  observation_source: string | null
  warnings: string[]
  error: string | null
  timestamp: number
}

export async function getTorStatus() {
  return invoke<TorRuntimeStatus>('get_tor_status')
}

export async function testTorConnection() {
  return invoke<TorRuntimeStatus>('test_tor_connection')
}
