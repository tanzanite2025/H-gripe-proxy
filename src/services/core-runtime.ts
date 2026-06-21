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

export interface RuntimeKernelLoopbackForwardingLeakCheckReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  listenerPort: number
  targetPort: number
  listenerPortReleased: boolean
  targetPortReleased: boolean
  isolatedTestListenerRunning: boolean
  preflight: RuntimeKernelLoopbackForwardingPreflightReport
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackPlatformMatrixRow {
  platform: string
  currentPlatform: boolean
  evidenceStatus: string
  listenerPortReleased?: boolean | null
  targetPortReleased?: boolean | null
  isolatedTestListenerStopped?: boolean | null
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackPlatformMatrixReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  requiredPlatforms: string[]
  coveredPlatforms: string[]
  pendingPlatforms: string[]
  currentPlatformPassed: boolean
  expandedOptInAllowed: boolean
  leakCheck: RuntimeKernelLoopbackForwardingLeakCheckReport
  rows: RuntimeKernelLoopbackPlatformMatrixRow[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackHoldWindowRow {
  platform: string
  currentPlatform: boolean
  evidenceStatus: string
  holdStartedAtEpochMs?: number | null
  observedAtEpochMs?: number | null
  minimumHoldSeconds: number
  elapsedHoldSeconds?: number | null
  holdWindowSatisfied: boolean
  platformMatrixPassed?: boolean | null
  leakCheckPassed?: boolean | null
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackHoldWindowReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  holdStartedAtEpochMs: number
  observedAtEpochMs: number
  minimumHoldSeconds: number
  elapsedHoldSeconds: number
  requiredPlatforms: string[]
  coveredHoldPlatforms: string[]
  pendingHoldPlatforms: string[]
  currentPlatformPassed: boolean
  currentPlatformHoldWindowSatisfied: boolean
  expandedOptInAllowed: boolean
  platformMatrix: RuntimeKernelLoopbackPlatformMatrixReport
  rows: RuntimeKernelLoopbackHoldWindowRow[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackPlatformRollbackDrillRow {
  platform: string
  currentPlatform: boolean
  evidenceStatus: string
  smokePassed?: boolean | null
  portsReleased?: boolean | null
  systemProxyUnchanged?: boolean | null
  tunUnchanged?: boolean | null
  runtimeConfigUnchanged?: boolean | null
  holdWindowSatisfied?: boolean | null
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackPlatformRollbackDrillsReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  requiredPlatforms: string[]
  coveredRollbackPlatforms: string[]
  pendingRollbackPlatforms: string[]
  currentPlatformPassed: boolean
  expandedOptInAllowed: boolean
  holdWindow: RuntimeKernelLoopbackHoldWindowReport
  rollbackDrill: RuntimeKernelLoopbackForwardingRollbackDrillReport
  rows: RuntimeKernelLoopbackPlatformRollbackDrillRow[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInPreflightCheck {
  name: string
  status: string
  passed: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR4ExpandedOptInPreflightRow {
  platform: string
  currentPlatform: boolean
  rollbackDrillObserved: boolean
  holdWindowSatisfied?: boolean | null
  evidenceStatus: string
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR4ExpandedOptInPreflightReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  explicitDecision: boolean
  requiredPlatforms: string[]
  observedRollbackPlatforms: string[]
  pendingRollbackPlatforms: string[]
  currentPlatformHoldWindowSatisfied: boolean
  preflightPassed: boolean
  expandedOptInAllowed: boolean
  holdWindow: RuntimeKernelLoopbackHoldWindowReport
  rows: RuntimeKernelLoopbackR4ExpandedOptInPreflightRow[]
  checks: RuntimeKernelLoopbackR4ExpandedOptInPreflightCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInExecutionPlanStep {
  order: number
  name: string
  action: string
  mutatesRuntime: boolean
  requiresExplicitDecision: boolean
  enabledInThisBatch: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR4ExpandedOptInExecutionPlanReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  candidateScope: string
  explicitDecision: boolean
  planReady: boolean
  executionAllowed: boolean
  expandedOptInAllowed: boolean
  preflight: RuntimeKernelLoopbackR4ExpandedOptInPreflightReport
  steps: RuntimeKernelLoopbackR4ExpandedOptInExecutionPlanStep[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInExecutionGuardCheck {
  name: string
  status: string
  passed: boolean
  requiredForExecution: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR4ExpandedOptInSafetyPlanStep {
  order: number
  phase: string
  action: string
  mutatesRuntime: boolean
  requiredBeforeExpansion: boolean
  enabledInThisBatch: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR4ExpandedOptInExecutionGuardReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  requestedExecution: boolean
  explicitDecision: boolean
  guardReady: boolean
  syntheticExecutionAllowed: boolean
  executionAllowed: boolean
  expandedOptInAllowed: boolean
  plan: RuntimeKernelLoopbackR4ExpandedOptInExecutionPlanReport
  guardChecks: RuntimeKernelLoopbackR4ExpandedOptInExecutionGuardCheck[]
  verificationPlan: RuntimeKernelLoopbackR4ExpandedOptInSafetyPlanStep[]
  rollbackPlan: RuntimeKernelLoopbackR4ExpandedOptInSafetyPlanStep[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout {
  rollbackDrillPassed: boolean
  leakCheckPassed: boolean
  portsReleased: boolean
  systemProxyUnchanged: boolean
  tunUnchanged: boolean
  runtimeConfigUnchanged: boolean
  isolatedTestListenerStopped: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR4ExpandedOptInSyntheticExecutionReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  requestedExecution: boolean
  explicitDecision: boolean
  syntheticExecutionAllowed: boolean
  executionAttempted: boolean
  expandedOptInAllowed: boolean
  guard: RuntimeKernelLoopbackR4ExpandedOptInExecutionGuardReport
  rollbackDrill?: RuntimeKernelLoopbackForwardingRollbackDrillReport | null
  leakCheck?: RuntimeKernelLoopbackForwardingLeakCheckReport | null
  closeout: RuntimeKernelLoopbackR4ExpandedOptInSyntheticExecutionCloseout
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInPostExecutionHoldReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  requestedExecution: boolean
  explicitDecision: boolean
  postExecutionHoldStartedAtEpochMs: number
  observedAtEpochMs: number
  minimumHoldSeconds: number
  elapsedHoldSeconds: number
  postExecutionHoldSatisfied: boolean
  executionAttempted: boolean
  syntheticExecutionPassed: boolean
  closeoutPassed: boolean
  expandedOptInAllowed: boolean
  syntheticExecution: RuntimeKernelLoopbackR4ExpandedOptInSyntheticExecutionReport
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInDecisionReadinessCheck {
  name: string
  status: string
  passed: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR4ExpandedOptInDecisionReadinessReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  requestedExecution: boolean
  explicitDecision: boolean
  widerOptInDecision: boolean
  decisionReady: boolean
  widerOptInAllowed: boolean
  expandedOptInAllowed: boolean
  postExecutionHold: RuntimeKernelLoopbackR4ExpandedOptInPostExecutionHoldReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInDecisionReadinessCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck {
  name: string
  status: string
  passed: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  listenerPort: number
  targetPort: number
  requestedExecution: boolean
  explicitDecision: boolean
  widerOptInDecision: boolean
  limitedRolloutDecision: boolean
  canaryScope: string
  maxCanarySessions: number
  gateReady: boolean
  limitedRolloutAllowed: boolean
  expandedOptInAllowed: boolean
  decisionReadiness: RuntimeKernelLoopbackR4ExpandedOptInDecisionReadinessReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInRolloutAuditRow {
  name: string
  status: string
  passed: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR4ExpandedOptInRolloutAuditReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  canaryScope: string
  maxCanarySessions: number
  auditReady: boolean
  limitedRolloutAllowed: boolean
  expandedOptInAllowed: boolean
  gate: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateReport
  rows: RuntimeKernelLoopbackR4ExpandedOptInRolloutAuditRow[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInCloseoutReadinessReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  closeoutDecision: boolean
  closeoutReady: boolean
  limitedRolloutAllowed: boolean
  expandedOptInAllowed: boolean
  audit: RuntimeKernelLoopbackR4ExpandedOptInRolloutAuditReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInCloseoutReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  requestedExecution: boolean
  explicitDecision: boolean
  closeoutDecision: boolean
  closeoutReady: boolean
  r4CloseoutComplete: boolean
  limitedRolloutAllowed: boolean
  expandedOptInAllowed: boolean
  closeoutReadiness: RuntimeKernelLoopbackR4ExpandedOptInCloseoutReadinessReport
  evidence: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInCompletionReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  r4Complete: boolean
  completedBatches: string[]
  openBoundaries: string[]
  nextPhaseCandidate: string
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  closeoutReport: RuntimeKernelLoopbackR4ExpandedOptInCloseoutReport
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR4ExpandedOptInNextPhaseHandoffReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  handoffDecision: boolean
  handoffReady: boolean
  nextPhase: string
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  completion: RuntimeKernelLoopbackR4ExpandedOptInCompletionReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverPreflightReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  r5PreflightDecision: boolean
  preflightReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  handoff: RuntimeKernelLoopbackR4ExpandedOptInNextPhaseHandoffReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverRiskRow {
  name: string
  severity: string
  status: string
  passed: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR5DefaultCutoverRiskMatrixReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  riskMatrixReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  preflight: RuntimeKernelLoopbackR5DefaultCutoverPreflightReport
  rows: RuntimeKernelLoopbackR5DefaultCutoverRiskRow[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverRollbackAbortPlanReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  rollbackPlanDecision: boolean
  rollbackAbortReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  riskMatrix: RuntimeKernelLoopbackR5DefaultCutoverRiskMatrixReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverExecutionPlanStep {
  order: number
  name: string
  phase: string
  allowed: boolean
  mutatesRuntime: boolean
  facts: string[]
}

export interface RuntimeKernelLoopbackR5DefaultCutoverExecutionPlanReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  executionPlanDecision: boolean
  executionPlanReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  rollbackAbortPlan: RuntimeKernelLoopbackR5DefaultCutoverRollbackAbortPlanReport
  steps: RuntimeKernelLoopbackR5DefaultCutoverExecutionPlanStep[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverGuardReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  guardDecision: boolean
  guardReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  executionPlan: RuntimeKernelLoopbackR5DefaultCutoverExecutionPlanReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverDryRunReadinessReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  dryRunDecision: boolean
  dryRunReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  guard: RuntimeKernelLoopbackR5DefaultCutoverGuardReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverDryRunEvidenceReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  dryRunExecuted: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  readiness: RuntimeKernelLoopbackR5DefaultCutoverDryRunReadinessReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverDryRunCloseoutReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  dryRunCloseoutReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  evidence: RuntimeKernelLoopbackR5DefaultCutoverDryRunEvidenceReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverPostDryRunHoldReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  holdDecision: boolean
  holdReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  closeout: RuntimeKernelLoopbackR5DefaultCutoverDryRunCloseoutReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverDecisionReadinessReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  decisionReadinessDecision: boolean
  decisionReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  postDryRunHold: RuntimeKernelLoopbackR5DefaultCutoverPostDryRunHoldReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverFinalGateReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  finalGateDecision: boolean
  finalGateReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  decisionReadiness: RuntimeKernelLoopbackR5DefaultCutoverDecisionReadinessReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverNextStepHandoffReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  r5HandoffDecision: boolean
  handoffReady: boolean
  nextStep: string
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  finalGate: RuntimeKernelLoopbackR5DefaultCutoverFinalGateReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverFinalHoldReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  finalHoldStartedAtEpochMs?: number | null
  finalHoldElapsedSeconds: number
  finalHoldDecision: boolean
  finalHoldReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  handoff: RuntimeKernelLoopbackR5DefaultCutoverNextStepHandoffReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  independentRollbackDecision: boolean
  rollbackValidationReady: boolean
  requiredPlatforms: string[]
  observedRollbackPlatforms: string[]
  pendingRollbackPlatforms: string[]
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  finalHold: RuntimeKernelLoopbackR5DefaultCutoverFinalHoldReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5DefaultCutoverCloseoutReadinessReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  r5CloseoutDecision: boolean
  closeoutReady: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  rollbackValidation: RuntimeKernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackForwardingRollbackDrillReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  listenerPort: number
  targetPort: number
  smokePassed: boolean
  portsReleased: boolean
  postPreflight: RuntimeKernelLoopbackForwardingPreflightReport
  systemProxyUnchanged: boolean
  tunUnchanged: boolean
  runtimeConfigUnchanged: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackForwardingSmokeEvidenceReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  requestedHost: string
  listenerPort: number
  targetPort: number
  requestPath: string
  listenerAccepted: boolean
  targetReceived: boolean
  responseStatus?: string | null
  bytesFromClient: number
  bytesFromTarget: number
  loopbackForwarded: boolean
  systemProxyUnchanged: boolean
  tunUnchanged: boolean
  runtimeConfigUnchanged: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
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

export async function getRuntimeKernelLoopbackForwardingSmokeEvidence(
  listenerPort?: number,
  targetPort?: number,
) {
  return invoke<RuntimeKernelLoopbackForwardingSmokeEvidenceReport>(
    'get_runtime_kernel_loopback_forwarding_smoke_evidence',
    { listenerPort, targetPort },
  )
}

export async function getRuntimeKernelLoopbackForwardingRollbackDrill(
  listenerPort?: number,
  targetPort?: number,
) {
  return invoke<RuntimeKernelLoopbackForwardingRollbackDrillReport>(
    'get_runtime_kernel_loopback_forwarding_rollback_drill',
    { listenerPort, targetPort },
  )
}

export async function getRuntimeKernelLoopbackForwardingLeakCheck(
  listenerPort?: number,
  targetPort?: number,
) {
  return invoke<RuntimeKernelLoopbackForwardingLeakCheckReport>(
    'get_runtime_kernel_loopback_forwarding_leak_check',
    { listenerPort, targetPort },
  )
}

export async function getRuntimeKernelLoopbackPlatformMatrix(
  listenerPort?: number,
  targetPort?: number,
) {
  return invoke<RuntimeKernelLoopbackPlatformMatrixReport>(
    'get_runtime_kernel_loopback_platform_matrix',
    { listenerPort, targetPort },
  )
}

export async function getRuntimeKernelLoopbackHoldWindow(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
) {
  return invoke<RuntimeKernelLoopbackHoldWindowReport>(
    'get_runtime_kernel_loopback_hold_window',
    { listenerPort, targetPort, holdStartedAtEpochMs },
  )
}

export async function getRuntimeKernelLoopbackPlatformRollbackDrills(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
) {
  return invoke<RuntimeKernelLoopbackPlatformRollbackDrillsReport>(
    'get_runtime_kernel_loopback_platform_rollback_drills',
    { listenerPort, targetPort, holdStartedAtEpochMs },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInPreflight(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInPreflightReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_preflight',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInExecutionPlan(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInExecutionPlanReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_execution_plan',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInExecutionGuard(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInExecutionGuardReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_execution_guard',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInSyntheticExecution(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInSyntheticExecutionReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_synthetic_execution',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInPostExecutionHold(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInPostExecutionHoldReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_post_execution_hold',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInDecisionReadiness(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInDecisionReadinessReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_decision_readiness',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGate(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_limited_rollout_gate',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInRolloutAudit(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInRolloutAuditReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_rollout_audit',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInCloseoutReadiness(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInCloseoutReadinessReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_closeout_readiness',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInCloseoutReport(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInCloseoutReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_closeout_report',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInCompletionSummary(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInCompletionReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_completion_summary',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR4ExpandedOptInNextPhaseHandoff(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR4ExpandedOptInNextPhaseHandoffReport>(
    'get_runtime_kernel_loopback_r4_expanded_opt_in_next_phase_handoff',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverPreflight(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverPreflightReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_preflight',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverRiskMatrix(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverRiskMatrixReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_risk_matrix',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverRollbackAbortPlan(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverRollbackAbortPlanReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_rollback_abort_plan',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverExecutionPlan(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverExecutionPlanReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_execution_plan',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverGuard(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverGuardReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_guard',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverDryRunReadiness(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverDryRunReadinessReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_dry_run_readiness',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverDryRunEvidence(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
  dryRunExecutionDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverDryRunEvidenceReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_dry_run_evidence',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
      dryRunExecutionDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverDryRunCloseout(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
  dryRunExecutionDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverDryRunCloseoutReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_dry_run_closeout',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
      dryRunExecutionDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverPostDryRunHold(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
  dryRunExecutionDecision?: boolean,
  postDryRunHoldStartedAtEpochMs?: number,
  holdDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverPostDryRunHoldReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_post_dry_run_hold',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
      dryRunExecutionDecision,
      postDryRunHoldStartedAtEpochMs,
      holdDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverDecisionReadiness(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
  dryRunExecutionDecision?: boolean,
  postDryRunHoldStartedAtEpochMs?: number,
  holdDecision?: boolean,
  decisionReadinessDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverDecisionReadinessReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_decision_readiness',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
      dryRunExecutionDecision,
      postDryRunHoldStartedAtEpochMs,
      holdDecision,
      decisionReadinessDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverFinalGate(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
  dryRunExecutionDecision?: boolean,
  postDryRunHoldStartedAtEpochMs?: number,
  holdDecision?: boolean,
  decisionReadinessDecision?: boolean,
  finalGateDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverFinalGateReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_final_gate',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
      dryRunExecutionDecision,
      postDryRunHoldStartedAtEpochMs,
      holdDecision,
      decisionReadinessDecision,
      finalGateDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverNextStepHandoff(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
  dryRunExecutionDecision?: boolean,
  postDryRunHoldStartedAtEpochMs?: number,
  holdDecision?: boolean,
  decisionReadinessDecision?: boolean,
  finalGateDecision?: boolean,
  r5HandoffDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverNextStepHandoffReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_next_step_handoff',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
      dryRunExecutionDecision,
      postDryRunHoldStartedAtEpochMs,
      holdDecision,
      decisionReadinessDecision,
      finalGateDecision,
      r5HandoffDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverFinalHold(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
  dryRunExecutionDecision?: boolean,
  postDryRunHoldStartedAtEpochMs?: number,
  holdDecision?: boolean,
  decisionReadinessDecision?: boolean,
  finalGateDecision?: boolean,
  r5HandoffDecision?: boolean,
  finalHoldStartedAtEpochMs?: number,
  finalHoldDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverFinalHoldReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_final_hold',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
      dryRunExecutionDecision,
      postDryRunHoldStartedAtEpochMs,
      holdDecision,
      decisionReadinessDecision,
      finalGateDecision,
      r5HandoffDecision,
      finalHoldStartedAtEpochMs,
      finalHoldDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverIndependentRollbackValidation(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
  dryRunExecutionDecision?: boolean,
  postDryRunHoldStartedAtEpochMs?: number,
  holdDecision?: boolean,
  decisionReadinessDecision?: boolean,
  finalGateDecision?: boolean,
  r5HandoffDecision?: boolean,
  finalHoldStartedAtEpochMs?: number,
  finalHoldDecision?: boolean,
  independentRollbackDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverIndependentRollbackValidationReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_independent_rollback_validation',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
      dryRunExecutionDecision,
      postDryRunHoldStartedAtEpochMs,
      holdDecision,
      decisionReadinessDecision,
      finalGateDecision,
      r5HandoffDecision,
      finalHoldStartedAtEpochMs,
      finalHoldDecision,
      independentRollbackDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverCloseoutReadiness(
  listenerPort?: number,
  targetPort?: number,
  holdStartedAtEpochMs?: number,
  observedRollbackPlatforms?: string[],
  explicitDecision?: boolean,
  requestedExecution?: boolean,
  postExecutionHoldStartedAtEpochMs?: number,
  widerOptInDecision?: boolean,
  limitedRolloutDecision?: boolean,
  canaryScope?: string,
  maxCanarySessions?: number,
  closeoutDecision?: boolean,
  handoffDecision?: boolean,
  r5PreflightDecision?: boolean,
  rollbackPlanDecision?: boolean,
  executionPlanDecision?: boolean,
  guardDecision?: boolean,
  dryRunDecision?: boolean,
  dryRunExecutionDecision?: boolean,
  postDryRunHoldStartedAtEpochMs?: number,
  holdDecision?: boolean,
  decisionReadinessDecision?: boolean,
  finalGateDecision?: boolean,
  r5HandoffDecision?: boolean,
  finalHoldStartedAtEpochMs?: number,
  finalHoldDecision?: boolean,
  independentRollbackDecision?: boolean,
  r5CloseoutDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverCloseoutReadinessReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_closeout_readiness',
    {
      listenerPort,
      targetPort,
      holdStartedAtEpochMs,
      observedRollbackPlatforms,
      explicitDecision,
      requestedExecution,
      postExecutionHoldStartedAtEpochMs,
      widerOptInDecision,
      limitedRolloutDecision,
      canaryScope,
      maxCanarySessions,
      closeoutDecision,
      handoffDecision,
      r5PreflightDecision,
      rollbackPlanDecision,
      executionPlanDecision,
      guardDecision,
      dryRunDecision,
      dryRunExecutionDecision,
      postDryRunHoldStartedAtEpochMs,
      holdDecision,
      decisionReadinessDecision,
      finalGateDecision,
      r5HandoffDecision,
      finalHoldStartedAtEpochMs,
      finalHoldDecision,
      independentRollbackDecision,
      r5CloseoutDecision,
    },
  )
}
