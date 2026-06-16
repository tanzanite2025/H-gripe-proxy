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

export interface DnsResolverRuntimeProbeSummary {
  totalTargets: number
  runtimeSupportedTargets: number
  healthyTargets: number
  failedTargets: number
  unsupportedTargets: number
}

export interface DnsResolverRuntimeProbeTargetReport {
  server: string
  protocol: string
  providerKind?: DnsServerProviderKind | null
  providerLabel?: string | null
  runtimeSupported: boolean
  healthy: boolean
  latencyMs?: number | null
  message: string
}

export interface DnsResolverRuntimeProbeReport {
  plan: DnsResolverPlan
  testDomain: string
  targets: DnsResolverRuntimeProbeTargetReport[]
  summary: DnsResolverRuntimeProbeSummary
  metrics: DnsResolverRuntimeMetrics
  warnings: string[]
}

export type DnsDefaultRuntimeReadinessStatus =
  | 'ready'
  | 'degraded'
  | 'blocked'

export type DnsDefaultRuntimeReadinessCheckStatus =
  | 'passed'
  | 'warning'
  | 'failed'
  | 'skipped'

export interface DnsDefaultRuntimeReadinessCheck {
  checkId: string
  status: DnsDefaultRuntimeReadinessCheckStatus
  message: string
  details: string[]
}

export interface DnsDefaultRuntimeReadinessSummary {
  passed: number
  warnings: number
  failed: number
  skipped: number
}

export interface DnsDefaultRuntimeReadinessReport {
  status: DnsDefaultRuntimeReadinessStatus
  reason: string
  plan: DnsResolverPlan
  probeSummary?: DnsResolverRuntimeProbeSummary | null
  checks: DnsDefaultRuntimeReadinessCheck[]
  summary: DnsDefaultRuntimeReadinessSummary
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export type DnsDefaultRuntimeShadowEvidenceStatus =
  | 'matched'
  | 'mismatched'
  | 'blocked'
  | 'incomplete'

export interface DnsDefaultRuntimeShadowQueryEvidence {
  domain: string
  rustReport: DnsResolverRuntimeQueryReport
  systemResult: DnsQueryResult
  ipMatch: boolean
  latencyDeltaMs: number
  mismatchReason?: string | null
}

export interface DnsDefaultRuntimeShadowEvidenceReport {
  status: DnsDefaultRuntimeShadowEvidenceStatus
  reason: string
  readiness: DnsDefaultRuntimeReadinessReport
  query: DnsDefaultRuntimeShadowQueryEvidence
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export type DnsDefaultRuntimeOptInSwitchGuardStatus = 'ready' | 'blocked'

export interface DnsDefaultRuntimeRollbackPlan {
  required: boolean
  supported: boolean
  strategy: string
  previousRuntime: string
  candidateRuntime: string
}

export interface DnsDefaultRuntimeOptInSwitchGuardReport {
  status: DnsDefaultRuntimeOptInSwitchGuardStatus
  reason: string
  readiness: DnsDefaultRuntimeReadinessReport
  shadowEvidence: DnsDefaultRuntimeShadowEvidenceReport
  rollbackPlan: DnsDefaultRuntimeRollbackPlan
  explicitOptIn: boolean
  mutatesRuntime: boolean
  activationMode: string
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export type DnsDefaultRuntimeExecutorPreflightStatus = 'ready' | 'blocked'

export interface DnsDefaultRuntimeMutationDiff {
  previousRuntime: string
  candidateRuntime: string
  runtimeOwnerBefore: string
  runtimeOwnerAfter: string
  nameserverTargets: string[]
  planOnlyFeatures: string[]
}

export interface DnsDefaultRuntimeExecutorAuditRecord {
  eventId: string
  action: string
  dryRun: boolean
  createdAtEpochSeconds: number
  guardStatus: DnsDefaultRuntimeOptInSwitchGuardStatus
  readinessStatus: DnsDefaultRuntimeReadinessStatus
  shadowStatus: DnsDefaultRuntimeShadowEvidenceStatus
}

export interface DnsDefaultRuntimeExecutorRollbackMarker {
  required: boolean
  prepared: boolean
  strategy: string
  restoresRuntime: boolean
  previousRuntime: string
  candidateRuntime: string
}

export interface DnsDefaultRuntimeOptInExecutorPreflightReport {
  status: DnsDefaultRuntimeExecutorPreflightStatus
  reason: string
  guard: DnsDefaultRuntimeOptInSwitchGuardReport
  mutationDiff: DnsDefaultRuntimeMutationDiff
  auditRecord: DnsDefaultRuntimeExecutorAuditRecord
  rollbackMarker: DnsDefaultRuntimeExecutorRollbackMarker
  dryRun: boolean
  wouldMutateRuntime: boolean
  executed: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export type DnsDefaultRuntimeExecutionGuardStatus = 'ready' | 'blocked'

export interface DnsDefaultRuntimeExecutionSupersededState {
  previousRuntime: string
  candidateRuntime: string
  state: string
  supersededAtEpochSeconds: number
  reason: string
}

export interface DnsDefaultRuntimeExecutionPersistence {
  requested: boolean
  prepared: boolean
  auditRecordPath?: string | null
  rollbackMarkerPath?: string | null
  supersededStatePath?: string | null
  auditPersisted: boolean
  rollbackMarkerPersisted: boolean
  supersededStatePersisted: boolean
  errors: string[]
}

export interface DnsDefaultRuntimeOptInExecutionGuardReport {
  status: DnsDefaultRuntimeExecutionGuardStatus
  reason: string
  preflight: DnsDefaultRuntimeOptInExecutorPreflightReport
  persistence: DnsDefaultRuntimeExecutionPersistence
  supersededState: DnsDefaultRuntimeExecutionSupersededState
  executionAllowed: boolean
  userTriggerRequired: boolean
  mutatesRuntime: boolean
  executed: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export type DnsDefaultRuntimeLimitedExecutionStatus = 'executed' | 'blocked'
export type DnsDefaultRuntimeLimitedRollbackStatus = 'restored' | 'blocked'
export type DnsDefaultRuntimePostExecutionVerificationStatus =
  | 'verified'
  | 'failed'
  | 'blocked'
export type DnsDefaultRuntimeRollbackDrillStatus = 'ready' | 'blocked'
export type DnsDefaultRuntimeExpandedOptInExecutionGateStatus =
  | 'ready'
  | 'blocked'
export type DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus =
  | 'ready'
  | 'blocked'
export type DnsDefaultRuntimeExpandedOptInExecutionStatus =
  | 'executed'
  | 'blocked'
  | 'failed'
export type DnsDefaultRuntimeExpandedRollbackStatus =
  | 'restored'
  | 'blocked'
  | 'failed'
export type DnsDefaultRuntimeExpandedPostExecutionVerificationStatus =
  | 'verified'
  | 'failed'
  | 'blocked'
export type DnsDefaultRuntimeExpandedRollbackDrillStatus =
  | 'ready'
  | 'blocked'
export type DnsDefaultRuntimeExpandedStabilityGateStatus =
  | 'ready'
  | 'blocked'
export type DnsDefaultRuntimeExpandedHoldPolicyStatus =
  | 'ready'
  | 'holding'
  | 'rollbackRecommended'
  | 'blocked'
export type DnsDefaultRuntimeExpandedReverifyStatus =
  | 'recorded'
  | 'rollbackRecommended'
  | 'blocked'
export type DnsDefaultRuntimeExpandedReverifyHistoryStatus =
  | 'ready'
  | 'watching'
  | 'rollbackRecommended'
  | 'empty'
  | 'blocked'
export type DnsDefaultRuntimeExpandedLifecycleCloseoutStatus =
  | 'complete'
  | 'watching'
  | 'rollbackRecommended'
  | 'blocked'
export type DnsDefaultRuntimeExpandedControlPlaneCompletionStatus =
  | 'complete'
  | 'watching'
  | 'rollbackRecommended'
  | 'blocked'

export interface DnsDefaultRuntimeExecutionRecord {
  eventId: string
  action: string
  status: string
  guardEventId: string
  previousRuntime: string
  candidateRuntime: string
  createdAtEpochSeconds: number
  metadataVerified: boolean
  error?: string | null
}

export interface DnsDefaultRuntimeActiveState {
  activeRuntime: string
  previousRuntime: string
  state: string
  executionEventId: string
  activatedAtEpochSeconds: number
  rollbackMarkerPath?: string | null
  auditRecordPath?: string | null
}

export interface DnsDefaultRuntimeLimitedOptInExecutionReport {
  status: DnsDefaultRuntimeLimitedExecutionStatus
  reason: string
  guard: DnsDefaultRuntimeOptInExecutionGuardReport
  executionRecord: DnsDefaultRuntimeExecutionRecord
  activeState?: DnsDefaultRuntimeActiveState | null
  activeStatePath?: string | null
  executionRecordPath?: string | null
  metadataVerified: boolean
  rollbackAvailable: boolean
  mutatesRuntime: boolean
  executed: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeLimitedRollbackReport {
  status: DnsDefaultRuntimeLimitedRollbackStatus
  reason: string
  previousState?: DnsDefaultRuntimeActiveState | null
  restoredState?: DnsDefaultRuntimeActiveState | null
  rollbackRecord: DnsDefaultRuntimeExecutionRecord
  activeStatePath?: string | null
  rollbackRecordPath?: string | null
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeRollbackDrillReport {
  status: DnsDefaultRuntimeRollbackDrillStatus
  reason: string
  activeState?: DnsDefaultRuntimeActiveState | null
  executionRecord?: DnsDefaultRuntimeExecutionRecord | null
  rollbackMarker?: DnsDefaultRuntimeExecutorRollbackMarker | null
  wouldRollback: boolean
  wouldRestoreRuntime: string
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimePostExecutionFailureAudit {
  required: boolean
  eventId: string
  activeExecutionEventId?: string | null
  reasons: string[]
  rollbackDrillRequired: boolean
  createdAtEpochSeconds: number
}

export interface DnsDefaultRuntimePostExecutionObservedVerificationReport {
  status: DnsDefaultRuntimePostExecutionVerificationStatus
  reason: string
  activeState?: DnsDefaultRuntimeActiveState | null
  executionRecord?: DnsDefaultRuntimeExecutionRecord | null
  preExecutionAuditRecord?: DnsDefaultRuntimeExecutorAuditRecord | null
  observedEvidence: DnsDefaultRuntimeShadowEvidenceReport
  rollbackDrill: DnsDefaultRuntimeRollbackDrillReport
  failureAudit: DnsDefaultRuntimePostExecutionFailureAudit
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedOptInExecutionScope {
  name: string
  description: string
  maxActiveRuntime: string
  allowedExecutionMode: string
  requiresUserTrigger: boolean
  requiresPostExecutionVerification: boolean
  requiresRollbackDrill: boolean
}

export interface DnsDefaultRuntimeExpandedOptInExecutionGateReport {
  status: DnsDefaultRuntimeExpandedOptInExecutionGateStatus
  reason: string
  postExecution: DnsDefaultRuntimePostExecutionObservedVerificationReport
  candidateScope: DnsDefaultRuntimeExpandedOptInExecutionScope
  expansionAllowed: boolean
  userTriggerRequired: boolean
  rollbackDrillRequired: boolean
  failureAuditRequired: boolean
  autoRollout: boolean
  mutatesRuntime: boolean
  executed: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedRuntimeMutationPlan {
  previousRuntime: string
  candidateRuntime: string
  executionMode: string
  activeProfileWrite: boolean
  mihomoReload: boolean
  profileSource: string
  rollbackStrategy: string
}

export interface DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord {
  eventId: string
  gateStatus: DnsDefaultRuntimeExpandedOptInExecutionGateStatus
  scopeName: string
  mutationPlan: DnsDefaultRuntimeExpandedRuntimeMutationPlan
  createdAtEpochSeconds: number
  explicitOptIn: boolean
}

export interface DnsDefaultRuntimeExpandedOptInExecutionPreflightReport {
  status: DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus
  reason: string
  gate: DnsDefaultRuntimeExpandedOptInExecutionGateReport
  preflightRecord: DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord
  preflightRecordPath?: string | null
  preflightPersisted: boolean
  userTriggerRequired: boolean
  wouldMutateRuntime: boolean
  mutatesRuntime: boolean
  executed: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedOptInExecutionReport {
  status: DnsDefaultRuntimeExpandedOptInExecutionStatus
  reason: string
  preflight: DnsDefaultRuntimeExpandedOptInExecutionPreflightReport
  executionRecord: DnsDefaultRuntimeExecutionRecord
  activeState?: DnsDefaultRuntimeActiveState | null
  activeStatePath?: string | null
  executionRecordPath?: string | null
  dnsConfigApplyAttempted: boolean
  dnsConfigApplied: boolean
  rollbackAvailable: boolean
  mutatesRuntime: boolean
  executed: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedRollbackReport {
  status: DnsDefaultRuntimeExpandedRollbackStatus
  reason: string
  previousState?: DnsDefaultRuntimeActiveState | null
  restoredState?: DnsDefaultRuntimeActiveState | null
  rollbackRecord: DnsDefaultRuntimeExecutionRecord
  activeStatePath?: string | null
  rollbackRecordPath?: string | null
  dnsConfigRestoreAttempted: boolean
  dnsConfigRestored: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedRollbackDrillReport {
  status: DnsDefaultRuntimeExpandedRollbackDrillStatus
  reason: string
  activeState?: DnsDefaultRuntimeActiveState | null
  executionRecord?: DnsDefaultRuntimeExecutionRecord | null
  preflightRecord?: DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord | null
  wouldRollback: boolean
  wouldRestoreRuntime: string
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport {
  status: DnsDefaultRuntimeExpandedPostExecutionVerificationStatus
  reason: string
  activeState?: DnsDefaultRuntimeActiveState | null
  executionRecord?: DnsDefaultRuntimeExecutionRecord | null
  preflightRecord?: DnsDefaultRuntimeExpandedOptInExecutionPreflightRecord | null
  observedEvidence: DnsDefaultRuntimeShadowEvidenceReport
  rollbackDrill: DnsDefaultRuntimeExpandedRollbackDrillReport
  failureAudit: DnsDefaultRuntimePostExecutionFailureAudit
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedStabilityGateReport {
  status: DnsDefaultRuntimeExpandedStabilityGateStatus
  reason: string
  postExecution: DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport
  keepActiveAllowed: boolean
  rollbackRecommended: boolean
  promotionAllowed: boolean
  recommendedAction: string
  userTriggerRequired: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedHoldPolicyReport {
  status: DnsDefaultRuntimeExpandedHoldPolicyStatus
  reason: string
  stabilityGate: DnsDefaultRuntimeExpandedStabilityGateReport
  activeAgeSeconds?: number | null
  minimumHoldSeconds: number
  maximumHoldSeconds: number
  holdStartedAtEpochSeconds?: number | null
  nextVerificationAfterEpochSeconds?: number | null
  holdExpiresAtEpochSeconds?: number | null
  keepActiveAllowed: boolean
  nextVerificationRequired: boolean
  rollbackRecommended: boolean
  promotionAllowed: boolean
  recommendedAction: string
  userTriggerRequired: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedReverifyRecord {
  eventId: string
  action: string
  activeExecutionEventId?: string | null
  holdStatus: DnsDefaultRuntimeExpandedHoldPolicyStatus
  stabilityStatus: DnsDefaultRuntimeExpandedStabilityGateStatus
  postExecutionStatus: DnsDefaultRuntimeExpandedPostExecutionVerificationStatus
  activeAgeSeconds?: number | null
  keepActiveAllowed: boolean
  nextVerificationRequired: boolean
  rollbackRecommended: boolean
  nextVerificationAfterEpochSeconds?: number | null
  holdExpiresAtEpochSeconds?: number | null
  createdAtEpochSeconds: number
}

export interface DnsDefaultRuntimeExpandedReverifyReport {
  status: DnsDefaultRuntimeExpandedReverifyStatus
  reason: string
  holdPolicy: DnsDefaultRuntimeExpandedHoldPolicyReport
  reverifyRecord: DnsDefaultRuntimeExpandedReverifyRecord
  reverifyRecordPath?: string | null
  reverifyPersisted: boolean
  keepActiveAllowed: boolean
  nextVerificationRequired: boolean
  rollbackRecommended: boolean
  userTriggerRequired: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedReverifyHistoryReport {
  status: DnsDefaultRuntimeExpandedReverifyHistoryStatus
  reason: string
  records: DnsDefaultRuntimeExpandedReverifyRecord[]
  latestRecord?: DnsDefaultRuntimeExpandedReverifyRecord | null
  recordCount: number
  recordedCount: number
  rollbackRecommendedCount: number
  blockedCount: number
  keepActiveCount: number
  nextVerificationRequiredCount: number
  stableStreak: number
  requiredStableRecords: number
  firstRecordAtEpochSeconds?: number | null
  latestRecordAtEpochSeconds?: number | null
  closeoutReady: boolean
  rollbackRecommended: boolean
  promotionAllowed: boolean
  recommendedAction: string
  userTriggerRequired: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedLifecycleCloseoutReport {
  status: DnsDefaultRuntimeExpandedLifecycleCloseoutStatus
  reason: string
  history: DnsDefaultRuntimeExpandedReverifyHistoryReport
  activeState?: DnsDefaultRuntimeActiveState | null
  observationClosed: boolean
  handoffReady: boolean
  rollbackRecommended: boolean
  promotionAllowed: boolean
  recommendedAction: string
  nextControlPlaneStep: string
  userTriggerRequired: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface DnsDefaultRuntimeExpandedHandoffManifest {
  manifestId: string
  action: string
  closeoutStatus: DnsDefaultRuntimeExpandedLifecycleCloseoutStatus
  historyStatus: DnsDefaultRuntimeExpandedReverifyHistoryStatus
  activeExecutionEventId?: string | null
  activeState?: string | null
  historyRecordCount: number
  stableStreak: number
  requiredStableRecords: number
  observationClosed: boolean
  handoffReady: boolean
  rollbackRecommended: boolean
  nextControlPlaneStep: string
  phase8Allowed: boolean
  promotionAllowed: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  createdAtEpochSeconds: number
}

export interface DnsDefaultRuntimeExpandedControlPlaneCompletionReport {
  status: DnsDefaultRuntimeExpandedControlPlaneCompletionStatus
  reason: string
  closeout: DnsDefaultRuntimeExpandedLifecycleCloseoutReport
  handoffManifest: DnsDefaultRuntimeExpandedHandoffManifest
  handoffManifestPath?: string | null
  handoffManifestPersisted: boolean
  dnsControlPlaneComplete: boolean
  observationClosed: boolean
  handoffReady: boolean
  rollbackRecommended: boolean
  nextControlPlaneStep: string
  phase8Allowed: boolean
  promotionAllowed: boolean
  userTriggerRequired: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
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

export async function dnsControlledRuntimeProbe(
  yaml: string,
  testDomain?: string,
): Promise<DnsResolverRuntimeProbeReport> {
  try {
    return await invoke<DnsResolverRuntimeProbeReport>(
      'dns_controlled_runtime_probe',
      {
        yaml,
        testDomain,
      },
    )
  } catch (err) {
    console.error('DNS controlled runtime probe failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeReadiness(
  yaml?: string,
  probeReport?: DnsResolverRuntimeProbeReport | null,
): Promise<DnsDefaultRuntimeReadinessReport> {
  try {
    return await invoke<DnsDefaultRuntimeReadinessReport>(
      'dns_default_runtime_readiness',
      {
        yaml,
        probeReport,
      },
    )
  } catch (err) {
    console.error('DNS default runtime readiness failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeShadowEvidence(
  yaml?: string,
  domain?: string,
): Promise<DnsDefaultRuntimeShadowEvidenceReport> {
  try {
    return await invoke<DnsDefaultRuntimeShadowEvidenceReport>(
      'dns_default_runtime_shadow_evidence',
      {
        yaml,
        domain,
      },
    )
  } catch (err) {
    console.error('DNS default runtime shadow evidence failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeOptInSwitchGuard(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeOptInSwitchGuardReport> {
  try {
    return await invoke<DnsDefaultRuntimeOptInSwitchGuardReport>(
      'dns_default_runtime_opt_in_switch_guard',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error('DNS default runtime opt-in switch guard failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeOptInExecutorPreflight(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeOptInExecutorPreflightReport> {
  try {
    return await invoke<DnsDefaultRuntimeOptInExecutorPreflightReport>(
      'dns_default_runtime_opt_in_executor_preflight',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error(
      'DNS default runtime opt-in executor preflight failed:',
      err,
    )
    throw err
  }
}

export async function dnsDefaultRuntimeOptInExecutionGuard(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeOptInExecutionGuardReport> {
  try {
    return await invoke<DnsDefaultRuntimeOptInExecutionGuardReport>(
      'dns_default_runtime_opt_in_execution_guard',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error('DNS default runtime opt-in execution guard failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeLimitedOptInExecution(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeLimitedOptInExecutionReport> {
  try {
    return await invoke<DnsDefaultRuntimeLimitedOptInExecutionReport>(
      'dns_default_runtime_limited_opt_in_execution',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error('DNS default runtime limited opt-in execution failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeRollbackDrill(): Promise<DnsDefaultRuntimeRollbackDrillReport> {
  try {
    return await invoke<DnsDefaultRuntimeRollbackDrillReport>(
      'dns_default_runtime_rollback_drill',
    )
  } catch (err) {
    console.error('DNS default runtime rollback drill failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimePostExecutionObservedVerification(
  yaml?: string,
  domain?: string,
): Promise<DnsDefaultRuntimePostExecutionObservedVerificationReport> {
  try {
    return await invoke<DnsDefaultRuntimePostExecutionObservedVerificationReport>(
      'dns_default_runtime_post_execution_observed_verification',
      {
        yaml,
        domain,
      },
    )
  } catch (err) {
    console.error(
      'DNS default runtime post-execution observed verification failed:',
      err,
    )
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedOptInExecutionGate(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeExpandedOptInExecutionGateReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedOptInExecutionGateReport>(
      'dns_default_runtime_expanded_opt_in_execution_gate',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error(
      'DNS default runtime expanded opt-in execution gate failed:',
      err,
    )
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedOptInExecutionPreflight(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeExpandedOptInExecutionPreflightReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedOptInExecutionPreflightReport>(
      'dns_default_runtime_expanded_opt_in_execution_preflight',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error(
      'DNS default runtime expanded opt-in execution preflight failed:',
      err,
    )
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedOptInExecution(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeExpandedOptInExecutionReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedOptInExecutionReport>(
      'dns_default_runtime_expanded_opt_in_execution',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error(
      'DNS default runtime expanded opt-in execution failed:',
      err,
    )
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedRollback(): Promise<DnsDefaultRuntimeExpandedRollbackReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedRollbackReport>(
      'dns_default_runtime_expanded_rollback',
    )
  } catch (err) {
    console.error('DNS default runtime expanded rollback failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedRollbackDrill(): Promise<DnsDefaultRuntimeExpandedRollbackDrillReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedRollbackDrillReport>(
      'dns_default_runtime_expanded_rollback_drill',
    )
  } catch (err) {
    console.error('DNS default runtime expanded rollback drill failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedPostExecutionObservedVerification(
  yaml?: string,
  domain?: string,
): Promise<DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport>(
      'dns_default_runtime_expanded_post_execution_observed_verification',
      {
        yaml,
        domain,
      },
    )
  } catch (err) {
    console.error(
      'DNS default runtime expanded post-execution observed verification failed:',
      err,
    )
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedStabilityGate(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeExpandedStabilityGateReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedStabilityGateReport>(
      'dns_default_runtime_expanded_stability_gate',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error('DNS default runtime expanded stability gate failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedHoldPolicy(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeExpandedHoldPolicyReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedHoldPolicyReport>(
      'dns_default_runtime_expanded_hold_policy',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error('DNS default runtime expanded hold policy failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedReverify(
  yaml?: string,
  domain?: string,
  explicitOptIn = false,
): Promise<DnsDefaultRuntimeExpandedReverifyReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedReverifyReport>(
      'dns_default_runtime_expanded_reverify',
      {
        yaml,
        domain,
        explicitOptIn,
      },
    )
  } catch (err) {
    console.error('DNS default runtime expanded reverify failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedReverifyHistory(): Promise<DnsDefaultRuntimeExpandedReverifyHistoryReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedReverifyHistoryReport>(
      'dns_default_runtime_expanded_reverify_history',
    )
  } catch (err) {
    console.error('DNS default runtime expanded reverify history failed:', err)
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedLifecycleCloseout(): Promise<DnsDefaultRuntimeExpandedLifecycleCloseoutReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedLifecycleCloseoutReport>(
      'dns_default_runtime_expanded_lifecycle_closeout',
    )
  } catch (err) {
    console.error(
      'DNS default runtime expanded lifecycle closeout failed:',
      err,
    )
    throw err
  }
}

export async function dnsDefaultRuntimeExpandedControlPlaneCompletion(): Promise<DnsDefaultRuntimeExpandedControlPlaneCompletionReport> {
  try {
    return await invoke<DnsDefaultRuntimeExpandedControlPlaneCompletionReport>(
      'dns_default_runtime_expanded_control_plane_completion',
    )
  } catch (err) {
    console.error(
      'DNS default runtime expanded control-plane completion failed:',
      err,
    )
    throw err
  }
}

export async function dnsDefaultRuntimeLimitedRollback(): Promise<DnsDefaultRuntimeLimitedRollbackReport> {
  try {
    return await invoke<DnsDefaultRuntimeLimitedRollbackReport>(
      'dns_default_runtime_limited_rollback',
    )
  } catch (err) {
    console.error('DNS default runtime limited rollback failed:', err)
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
