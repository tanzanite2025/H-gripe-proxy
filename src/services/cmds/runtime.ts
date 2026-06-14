import { invoke } from '@tauri-apps/api/core'
import dayjs from 'dayjs'

export async function getClashInfo() {
  return invoke<IClashInfo | null>('get_clash_info')
}

export async function getRuntimeConfig() {
  return invoke<IConfigData | null>('get_runtime_config')
}

export interface GeoDataUpdateTime {
  mmdb: number | null
  geoip: number | null
  asn: number | null
  city: number | null
  geosite: number | null
}

export async function getGeoDataUpdateTime() {
  return invoke<GeoDataUpdateTime>('get_geo_data_update_time')
}

export async function getRuntimeYaml() {
  return invoke<string | null>('get_runtime_yaml')
}

export async function getRuntimeExists() {
  return invoke<string[]>('get_runtime_exists')
}

export async function getRuntimeLogs() {
  return invoke<Record<string, [string, string][]>>('get_runtime_logs')
}

export async function getRuntimeProxyChainConfig(proxyChainExitNode: string) {
  return invoke<string>('get_runtime_proxy_chain_config', {
    proxyChainExitNode,
  })
}

export async function updateProxyChainConfigInRuntime(
  proxyChainConfig: string[] | null,
) {
  return invoke<void>('update_proxy_chain_config_in_runtime', {
    proxyChainConfig,
  })
}

export async function patchClashConfig(payload: Partial<IConfigData>) {
  return invoke<void>('patch_clash_config', { payload })
}

export async function syncTrayProxySelection() {
  return invoke<void>('sync_tray_proxy_selection')
}

export async function getClashLogs() {
  const regex = /time="(.+?)"\s+level=(.+?)\s+msg="(.+?)"/
  const newRegex = /(.+?)\s+(.+?)\s+(.+)/
  const logs = await invoke<string[]>('get_clash_logs')

  return logs.reduce<ILogItem[]>((acc, log) => {
    const result = log.match(regex)
    if (result) {
      const [_, _time, type, payload] = result
      const time = dayjs(_time).format('MM-DD HH:mm:ss')
      acc.push({ time, type, payload })
      return acc
    }

    const result2 = log.match(newRegex)
    if (result2) {
      const [_, time, type, payload] = result2
      acc.push({ time, type, payload })
    }
    return acc
  }, [])
}

export async function clearLogs() {
  return invoke<void>('clear_logs')
}

export async function applyDnsConfig(apply: boolean) {
  return invoke<void>('apply_dns_config', { apply })
}

export async function cmdTestDelay(url: string) {
  return invoke<number>('test_delay', { url })
}

export type LatencyNetworkQuality = 'good' | 'poor' | 'offline'
export type LatencyTestPlanStatus = 'ready' | 'skipped'

export interface LatencyTestPlanRequest {
  proxyNames?: string[]
  group?: string | null
  url?: string | null
  timeoutMs?: number | null
  concurrency?: number | null
  networkQuality?: LatencyNetworkQuality | null
}

export interface LatencyTestPlan {
  status: LatencyTestPlanStatus
  reason: string
  group: string | null
  normalizedUrl: string
  timeoutMs: number
  requestedCount: number
  scheduledCount: number
  concurrency: number
  estimatedMaxDurationMs: number | null
  proxyNames: string[]
}

export async function planLatencyTest(request: LatencyTestPlanRequest) {
  return invoke<LatencyTestPlan>('plan_latency_test', { request })
}
