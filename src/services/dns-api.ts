/**
 * DNS API 调用包装器
 * 封装 Tauri 后端 DNS 命令调用
 */

import { invoke } from '@tauri-apps/api/core'
import {
  getDnsMetrics as pluginGetDnsMetrics,
  dnsWarmup as pluginDnsWarmup,
  type DnsMetrics,
} from 'tauri-plugin-mihomo-api'

/**
 * DNS 协议类型
 */
export type DnsProtocol = 'udp' | 'tcp' | 'doh' | 'dot'

/**
 * DNS 查询结果
 */
export interface DnsQueryResult {
  domain: string
  ip: string
  latency: number
  success: boolean
  error?: string
  protocol: string
}

/**
 * DNS 健康检查结果
 */
export interface DnsHealthCheckResult {
  server: string
  latency: number
  success: boolean
  error?: string
  protocol: string
}

export interface DnsServerProbeTarget {
  server: string
  protocol: DnsProtocol
  protocolName: string
  socketAddr: string
  tlsDnsName?: string | null
}

export type DnsServerProviderKind =
  | 'cloudflare'
  | 'google'
  | 'quad9'
  | 'aliDns'
  | 'dohPub'
  | 'dotPub'

export type DnsServerProviderAvailability =
  | 'ready'
  | 'experimental'
  | 'placeholder'

export interface DnsServerProviderEndpointRegistration {
  protocol: DnsProtocol
  server: string
}

export interface DnsServerProviderRegistration {
  kind: DnsServerProviderKind
  label: string
  availability: DnsServerProviderAvailability
  description: string
  canonical_host: string
  host_aliases: string[]
  bootstrap_ips: string[]
  supported_protocols: DnsProtocol[]
  recommended_servers: DnsServerProviderEndpointRegistration[]
}

export interface DnsServerProviderHealthReport {
  provider_kind: DnsServerProviderKind
  provider_label: string
  server: string
  protocol: string
  test_domain: string
  healthy: boolean
  message: string
  latency_ms: number | null
  checked_at:
    | string
    | number
    | {
        secs_since_epoch?: number
        nanos_since_epoch?: number
        secsSinceEpoch?: number
        nanosSinceEpoch?: number
        secs?: number
        seconds?: number
        nanos?: number
      }
}

export type DnsConfigProbePlanStatus = 'ready' | 'skipped'

export interface DnsConfigExplainReport {
  valid: boolean
  explanation: string
  enabled?: boolean | null
  enhancedMode?: string | null
  fakeIpRange?: string | null
  serverSections: DnsConfigServerSection[]
  nameserverPolicyCount: number
  fallbackFilterKeys: string[]
  probePlan: DnsConfigProbePlan
  errors: string[]
  warnings: string[]
}

export interface DnsConfigServerSection {
  key: string
  serverCount: number
  probeableCount: number
  skippedCount: number
  servers: DnsConfigServerExplain[]
}

export interface DnsConfigServerExplain {
  section: string
  policyKey?: string | null
  server: string
  probeable: boolean
  reason: string
  target?: DnsServerProbeTarget | null
}

export interface DnsConfigProbePlan {
  status: DnsConfigProbePlanStatus
  reason: string
  testDomain: string
  targetCount: number
  targets: DnsServerProbeTarget[]
  skipped: DnsConfigProbeSkipped[]
}

export interface DnsConfigProbeSkipped {
  section: string
  policyKey?: string | null
  server: string
  reason: string
}

export type DnsResolverPlanStatus = 'ready' | 'disabled' | 'rejected'

export interface DnsResolverRuntimeFeaturePlan {
  configured: boolean
  runtimeApplied: boolean
  reason: string
}

export interface DnsResolverRuntimeProjection {
  fakeIp: DnsResolverRuntimeFeaturePlan
  fallbackFilter: DnsResolverRuntimeFeaturePlan
  nameserverPolicy: DnsResolverRuntimeFeaturePlan
}

export interface DnsResolverNameserverPlan {
  server: string
  protocol: DnsProtocol
  protocolName: string
  target?: DnsServerProbeTarget | null
  runtimeSupported: boolean
  reason: string
}

export interface DnsResolverPlan {
  status: DnsResolverPlanStatus
  reason: string
  enabled?: boolean | null
  timeoutMs: number
  attempts: number
  nameservers: DnsResolverNameserverPlan[]
  runtimeProjection: DnsResolverRuntimeProjection
  warnings: string[]
}

export interface DnsResolverRuntimeMetrics {
  totalQueries: number
  successfulQueries: number
  failedQueries: number
  totalLatencyMs: number
  lastError?: string | null
}

export interface DnsResolverRuntimeQueryReport {
  plan: DnsResolverPlan
  domain: string
  result?: DnsQueryResult | null
  attemptedServers: string[]
  metrics: DnsResolverRuntimeMetrics
}

/**
 * DNS 查询选项
 */
export interface DnsQueryOptions {
  server?: string
  protocol?: DnsProtocol
}

/**
 * DNS 查询
 */
export async function dnsQuery(
  domain: string,
  options?: DnsQueryOptions,
): Promise<DnsQueryResult> {
  try {
    const result = await invoke<DnsQueryResult>('dns_query', {
      domain,
      server: options?.server,
      protocol: options?.protocol,
    })
    return result
  } catch (err) {
    console.error(`DNS query failed for ${domain}:`, err)
    throw err
  }
}

/**
 * DNS 健康检查
 */
export async function dnsHealthCheck(
  server: string,
  testDomain?: string,
  protocol?: DnsProtocol,
): Promise<DnsHealthCheckResult> {
  try {
    const result = await invoke<DnsHealthCheckResult>('dns_health_check', {
      server,
      testDomain,
      protocol,
    })
    return result
  } catch (err) {
    console.error(`DNS health check failed for ${server}:`, err)
    throw err
  }
}

/**
 * 批量 DNS 查询
 */
export async function dnsBatchQuery(
  domains: string[],
  options?: DnsQueryOptions,
): Promise<DnsQueryResult[]> {
  try {
    const results = await invoke<DnsQueryResult[]>('dns_batch_query', {
      domains,
      server: options?.server,
      protocol: options?.protocol,
    })
    return results
  } catch (err) {
    console.error('DNS batch query failed:', err)
    throw err
  }
}

/**
 * 批量 DNS 健康检查
 */
export async function dnsBatchHealthCheck(
  servers: string[],
  testDomain?: string,
  protocol?: DnsProtocol,
): Promise<DnsHealthCheckResult[]> {
  try {
    const results = await invoke<DnsHealthCheckResult[]>(
      'dns_batch_health_check',
      {
        servers,
        testDomain,
        protocol,
      },
    )
    return results
  } catch (err) {
    console.error('DNS batch health check failed:', err)
    throw err
  }
}

export async function getDnsProviderRegistrations(): Promise<
  DnsServerProviderRegistration[]
> {
  try {
    return await invoke<DnsServerProviderRegistration[]>(
      'dns_get_provider_registrations',
    )
  } catch (err) {
    console.error('DNS provider registration query failed:', err)
    throw err
  }
}

export async function probeDnsProvider(
  kind: DnsServerProviderKind,
  protocol?: DnsProtocol,
  testDomain?: string,
): Promise<DnsServerProviderHealthReport> {
  try {
    return await invoke<DnsServerProviderHealthReport>('dns_probe_provider', {
      kind,
      protocol,
      testDomain,
    })
  } catch (err) {
    console.error(`DNS provider probe failed for ${kind}:`, err)
    throw err
  }
}

export async function explainDnsConfig(
  yaml: string,
  testDomain?: string,
): Promise<DnsConfigExplainReport> {
  try {
    return await invoke<DnsConfigExplainReport>('dns_explain_config', {
      yaml,
      testDomain,
    })
  } catch (err) {
    console.error('DNS config explain failed:', err)
    throw err
  }
}

export async function planDnsProbe(
  yaml: string,
  testDomain?: string,
): Promise<DnsConfigProbePlan> {
  try {
    return await invoke<DnsConfigProbePlan>('dns_plan_probe', {
      yaml,
      testDomain,
    })
  } catch (err) {
    console.error('DNS probe planning failed:', err)
    throw err
  }
}

export async function buildDnsResolverPlan(
  yaml: string,
): Promise<DnsResolverPlan> {
  try {
    return await invoke<DnsResolverPlan>('dns_build_resolver_plan', { yaml })
  } catch (err) {
    console.error('DNS resolver plan build failed:', err)
    throw err
  }
}

export async function dnsRuntimeQuery(
  yaml: string,
  domain: string,
): Promise<DnsResolverRuntimeQueryReport> {
  try {
    return await invoke<DnsResolverRuntimeQueryReport>('dns_runtime_query', {
      yaml,
      domain,
    })
  } catch (err) {
    console.error(`DNS runtime query failed for ${domain}:`, err)
    throw err
  }
}

export type { DnsMetrics }

export async function getDnsMetrics(): Promise<DnsMetrics> {
  return await pluginGetDnsMetrics()
}

export async function dnsWarmup(): Promise<void> {
  await pluginDnsWarmup()
}
