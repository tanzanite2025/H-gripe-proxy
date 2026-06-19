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

import type { DnsDefaultRuntimeShadowEvidenceReport } from './dns-api'

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

export interface RuntimeKernelIsolatedTestListenerStatus {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  running: boolean
  host: string
  port?: number | null
  startedAtEpochMs?: number | null
  acceptedConnections: number
  loopbackOnly: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelIsolatedTestListenerSmokeEvidenceReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  requestedHost: string
  requestedPort: number
  startedBySmoke: boolean
  responseStatus?: string | null
  acceptedConnectionsBefore: number
  acceptedConnectionsAfter: number
  statusIncremented: boolean
  stoppedAfterSmoke: boolean
  systemProxyUnchanged: boolean
  tunUnchanged: boolean
  runtimeConfigUnchanged: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackForwardingPortCheck {
  host: string
  listenerPort: number
  targetPort: number
  listenerAvailable: boolean
  targetAvailable: boolean
  targetLoopbackOnly: boolean
  notes: string[]
}

export interface RuntimeKernelLoopbackForwardingPreflightReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  requestedHost: string
  listenerPort: number
  targetPort: number
  canStartAfterOptIn: boolean
  portCheck: RuntimeKernelLoopbackForwardingPortCheck
  systemProxyEnabled: boolean
  tunEnabled: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersAllowed: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackDnsSmokeEvidenceReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  requestedHost: string
  requestedPort: number
  queryName: string
  udpBound: boolean
  localResponseReceived: boolean
  responseAddress?: string | null
  systemProxyUnchanged: boolean
  tunUnchanged: boolean
  runtimeConfigUnchanged: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackDnsPortCheck {
  host: string
  port: number
  udpAvailable: boolean
  tcpAvailable: boolean
  notes: string[]
}

export interface RuntimeKernelLoopbackDnsPreflightReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  requestedHost: string
  requestedPort: number
  canStartAfterOptIn: boolean
  portCheck: RuntimeKernelLoopbackDnsPortCheck
  runtimeDnsPresent: boolean
  appDnsSettingsEnabled: boolean
  systemProxyEnabled: boolean
  tunEnabled: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelIsolatedListenerPortCheck {
  host: string
  port: number
  available: boolean
  conflictsWithRuntimePort: boolean
  notes: string[]
}

export interface RuntimeKernelIsolatedListenerPreflightReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  requestedHost: string
  requestedPort: number
  canStartAfterOptIn: boolean
  portCheck: RuntimeKernelIsolatedListenerPortCheck
  runtimePorts: Record<string, number>
  systemProxyEnabled: boolean
  tunEnabled: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelConnectionSessionSample {
  sampleIndex: number
  network: string
  connectionType: string
  chainLen: number
  providerChainLen: number
  hasHost: boolean
  hasProcess: boolean
  hasRemoteDestination: boolean
  rule: string
  uploadedBytes: number
  downloadedBytes: number
}

export interface RuntimeKernelConnectionSessionShadowReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  connectionCount: number
  uploadTotal: number
  downloadTotal: number
  memory: number
  networkCounts: Record<string, number>
  connectionTypeCounts: Record<string, number>
  ruleCounts: Record<string, number>
  samples: RuntimeKernelConnectionSessionSample[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelAdapterCapabilityEntry {
  proxyType: string
  appCount: number
  mihomoCount: number
  inventoryMatched: boolean
  rustShadowSupported: boolean
  liveExecutionAllowed: boolean
  notes: string[]
}

export interface RuntimeKernelAdapterCapabilityReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  appProxyCount: number
  mihomoProxyCount: number
  capabilities: RuntimeKernelAdapterCapabilityEntry[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelRuleShadowRule {
  index: number
  ruleType: string
  payload: string
  proxy: string
  source: string
}

export interface RuntimeKernelRuleShadowSample {
  sampleIndex: number
  appRule?: RuntimeKernelRuleShadowRule | null
  mihomoRule?: RuntimeKernelRuleShadowRule | null
  matched: boolean
  mismatchReason?: string | null
}

export interface RuntimeKernelRuleShadowEvidenceReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  status: string
  appRuleCount: number
  mihomoRuleCount: number
  comparedSampleSize: number
  matchedSampleCount: number
  mismatchedSampleCount: number
  samples: RuntimeKernelRuleShadowSample[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelDnsShadowEvidenceReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  evidence: DnsDefaultRuntimeShadowEvidenceReport
  blockers: string[]
  nextSafeBatch: string
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

export async function getRuntimeKernelDnsShadowEvidence(
  yaml?: string,
  domain?: string,
) {
  return invoke<RuntimeKernelDnsShadowEvidenceReport>(
    'get_runtime_kernel_dns_shadow_evidence',
    { yaml, domain },
  )
}

export async function getRuntimeKernelRuleShadowEvidence() {
  return invoke<RuntimeKernelRuleShadowEvidenceReport>(
    'get_runtime_kernel_rule_shadow_evidence',
  )
}

export async function getRuntimeKernelAdapterCapabilityReport() {
  return invoke<RuntimeKernelAdapterCapabilityReport>(
    'get_runtime_kernel_adapter_capability_report',
  )
}

export async function getRuntimeKernelConnectionSessionShadow() {
  return invoke<RuntimeKernelConnectionSessionShadowReport>(
    'get_runtime_kernel_connection_session_shadow',
  )
}

export async function getRuntimeKernelIsolatedListenerPreflight(port?: number) {
  return invoke<RuntimeKernelIsolatedListenerPreflightReport>(
    'get_runtime_kernel_isolated_listener_preflight',
    { port },
  )
}

export async function getRuntimeKernelIsolatedTestListenerStatus() {
  return invoke<RuntimeKernelIsolatedTestListenerStatus>(
    'get_runtime_kernel_isolated_test_listener_status',
  )
}

export async function startRuntimeKernelIsolatedTestListener(port?: number) {
  return invoke<RuntimeKernelIsolatedTestListenerStatus>(
    'start_runtime_kernel_isolated_test_listener',
    { port },
  )
}

export async function stopRuntimeKernelIsolatedTestListener() {
  return invoke<RuntimeKernelIsolatedTestListenerStatus>(
    'stop_runtime_kernel_isolated_test_listener',
  )
}

export async function getRuntimeKernelIsolatedTestListenerSmokeEvidence(
  port?: number,
) {
  return invoke<RuntimeKernelIsolatedTestListenerSmokeEvidenceReport>(
    'get_runtime_kernel_isolated_test_listener_smoke_evidence',
    { port },
  )
}

export async function getRuntimeKernelLoopbackDnsPreflight(port?: number) {
  return invoke<RuntimeKernelLoopbackDnsPreflightReport>(
    'get_runtime_kernel_loopback_dns_preflight',
    { port },
  )
}

export async function getRuntimeKernelLoopbackDnsSmokeEvidence(port?: number) {
  return invoke<RuntimeKernelLoopbackDnsSmokeEvidenceReport>(
    'get_runtime_kernel_loopback_dns_smoke_evidence',
    { port },
  )
}

export async function getRuntimeKernelLoopbackForwardingPreflight(
  listenerPort?: number,
  targetPort?: number,
) {
  return invoke<RuntimeKernelLoopbackForwardingPreflightReport>(
    'get_runtime_kernel_loopback_forwarding_preflight',
    { listenerPort, targetPort },
  )
}
