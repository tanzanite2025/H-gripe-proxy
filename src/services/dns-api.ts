/**
 * DNS API 调用包装器
 * 封装 Tauri 后端 DNS 命令调用
 */

import { invoke } from '@tauri-apps/api/core'

/**
 * DNS 查询结果
 */
export interface DnsQueryResult {
  domain: string
  ip: string
  latency: number
  success: boolean
  error?: string
}

/**
 * DNS 健康检查结果
 */
export interface DnsHealthCheckResult {
  server: string
  latency: number
  success: boolean
  error?: string
}

/**
 * DNS 查询
 */
export async function dnsQuery(domain: string): Promise<DnsQueryResult> {
  try {
    const result = await invoke<DnsQueryResult>('dns_query', { domain })
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
): Promise<DnsHealthCheckResult> {
  try {
    const result = await invoke<DnsHealthCheckResult>('dns_health_check', {
      server,
      testDomain,
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
export async function dnsBatchQuery(domains: string[]): Promise<DnsQueryResult[]> {
  try {
    const results = await invoke<DnsQueryResult[]>('dns_batch_query', { domains })
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
): Promise<DnsHealthCheckResult[]> {
  try {
    const results = await invoke<DnsHealthCheckResult[]>('dns_batch_health_check', {
      servers,
      testDomain,
    })
    return results
  } catch (err) {
    console.error('DNS batch health check failed:', err)
    throw err
  }
}
