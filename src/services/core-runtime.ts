import { invoke } from '@tauri-apps/api/core'
import type {
  BaseConfig,
  BufferPoolStats,
  CoreUpdaterChannel,
  DnsMetrics,
  EngineStats,
  HotReloadStatus,
  MihomoVersion,
  PerfStats,
  RuleTrafficSnapshot,
  TLSFingerprintStats,
  TLSRotationResult,
  XDPStatus,
} from 'tauri-plugin-mihomo-api'

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

export async function reloadRuntimeConfig() {
  await invoke<void>('reload_runtime_config')
}

export async function getRuntimeEngineStats() {
  return invoke<EngineStats>('get_runtime_engine_stats')
}

export async function getRuntimePerfStats() {
  return invoke<PerfStats>('get_runtime_perf_stats')
}

export async function getRuntimeBufferPoolStats() {
  return invoke<BufferPoolStats>('get_runtime_buffer_pool_stats')
}

export async function getRuntimeHotReloadStatus() {
  return invoke<HotReloadStatus>('get_runtime_hot_reload_status')
}

export async function getRuntimeXdpStatus() {
  return invoke<XDPStatus>('get_runtime_xdp_status')
}

export async function getRuntimeRuleTraffic() {
  return invoke<Record<string, RuleTrafficSnapshot>>('get_runtime_rule_traffic')
}

export async function getRuntimeTlsFingerprintStats() {
  return invoke<TLSFingerprintStats>('get_runtime_tls_fingerprint_stats')
}

export async function forceRuntimeTlsRotation() {
  return invoke<TLSRotationResult>('force_runtime_tls_rotation')
}

export interface RuntimeLifecycleRecord {
  kind: string
  success: boolean
  error?: string | null
  detail?: string | null
  updatedAt: number
}

export async function getRuntimeLifecycleState() {
  return invoke<{ records: RuntimeLifecycleRecord[] }>(
    'get_runtime_lifecycle_state',
  )
}

export async function getRuntimeUpgradeHistory() {
  return invoke<{ records: RuntimeLifecycleRecord[] }>(
    'get_runtime_upgrade_history',
  )
}

export async function upgradeRuntimeCore(
  channel: CoreUpdaterChannel,
  force: boolean,
) {
  await invoke<void>('upgrade_runtime_core', { channel, force })
}

export async function upgradeRuntimeUi() {
  await invoke<void>('upgrade_runtime_ui')
}

export async function upgradeRuntimeGeo() {
  await invoke<void>('upgrade_runtime_geo')
}

export interface RuntimeKernelReplacementBlocker {
  area: string
  reason: string
  requiredNextStep: string
}

export interface RuntimeKernelPreflightReport {
  runtimeId: string
  artifactId?: string | null
  mutatesRuntime: boolean
  canApplyWithRustKernel: boolean
  mihomoFallback: boolean
  facts: string[]
  blockedReplacementAreas: RuntimeKernelReplacementBlocker[]
  nextSafeBatch: string
}

export interface RuntimeKernelShadowComponent {
  component: string
  kernelArea: string
  status: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  evidence: string[]
  nextStep: string
}

export interface RuntimeKernelShadowComponentsReport {
  runtimeId: string
  activeKernel: string
  mutatesRuntime: boolean
  components: RuntimeKernelShadowComponent[]
  liveExecutionBlockers: RuntimeKernelReplacementBlocker[]
  nextSafeBatch: string
}

export interface RuntimeKernelReplacementReadiness {
  mutatesRuntime: boolean
  activeKernel: string
  controllerTransport: string
  rustOwnedControlPlane: string[]
  mihomoOwnedDataPlane: string[]
  blockedReplacementAreas: RuntimeKernelReplacementBlocker[]
  nextSafeBatch: string
}

export async function getRuntimeKernelReplacementReadiness() {
  return invoke<RuntimeKernelReplacementReadiness>(
    'get_runtime_kernel_replacement_readiness',
  )
}

export async function getRuntimeKernelApplyPreflight(artifactId?: string) {
  return invoke<RuntimeKernelPreflightReport>(
    'get_runtime_kernel_apply_preflight',
    { artifactId },
  )
}

export async function getRuntimeKernelShadowComponents() {
  return invoke<RuntimeKernelShadowComponentsReport>(
    'get_runtime_kernel_shadow_components',
  )
}
