import { invoke } from '@tauri-apps/api/core'

import type { BaseConfig, DnsMetrics, MihomoVersion } from '@/types/mihomo'

export async function getRuntimeVersion() {
  return invoke<MihomoVersion>('get_runtime_version')
}

export async function getRuntimeBaseConfig() {
  return invoke<BaseConfig>('get_runtime_base_config')
}

export async function patchRuntimeBaseConfig(data: Record<string, unknown>) {
  await invoke<void>('patch_runtime_base_config', { data })
}

export async function updateRuntimeGeo() {
  await invoke<void>('update_runtime_geo')
}

export async function getRuntimeDnsMetrics() {
  return invoke<DnsMetrics>('get_runtime_dns_metrics')
}

export async function runtimeDnsWarmup() {
  await invoke<void>('runtime_dns_warmup')
}
