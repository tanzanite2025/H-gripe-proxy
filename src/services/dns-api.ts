/**
 * DNS API 调用包装器
 * 封装 Tauri 后端 DNS 命令调用
 */

import { invoke } from '@tauri-apps/api/core'

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
    const results = await invoke<DnsHealthCheckResult[]>('dns_batch_health_check', {
      servers,
      testDomain,
      protocol,
    })
    return results
  } catch (err) {
    console.error('DNS batch health check failed:', err)
    throw err
  }
}
