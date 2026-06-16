import { invoke } from '@tauri-apps/api/core'

import type {
  DnsDefaultRuntimeExpandedControlPlaneCompletionReport,
  DnsDefaultRuntimeExpandedControlPlaneCompletionStatus,
  DnsResolverPlan,
} from './dns-api'

export type AppProcessMatcherKind =
  | 'process_name'
  | 'process_path'
  | 'process_name_regex'
  | 'process_path_regex'
  | 'bundle_id'

export type AppRoutingIntent =
  | 'direct'
  | 'proxy'
  | 'reject'
  | 'auto'
  | 'fallback'

export interface AppEnvironmentVariable {
  key: string
  value: string
}

export interface AppProcessMatcher {
  kind: AppProcessMatcherKind
  pattern: string
}

export interface AppRegistryEntry {
  appId: string
  name: string
  executablePath?: string
  bundleId?: string
  launchArgs: string[]
  workingDirectory?: string
  env: AppEnvironmentVariable[]
  processMatchers: AppProcessMatcher[]
  platformMetadata: Record<string, string>
  tags: string[]
  updatedAt: number
}

export interface NodePoolHealthConstraints {
  maxLatencyMs?: number
  requireAlive?: boolean
  minAvailableNodes?: number
}

export interface NodePoolCandidate {
  nodeName: string
  proxyGroup?: string
  protocol?: string
  region?: string
  tags: string[]
  priority?: number
}

export interface NodePool {
  poolId: string
  name: string
  tags: string[]
  region?: string
  protocols: string[]
  purpose?: string
  costTier?: string
  healthConstraints: NodePoolHealthConstraints
  candidateNodes: NodePoolCandidate[]
  updatedAt: number
}

export interface DnsProfile {
  profileId: string
  name: string
  configYaml: string
  testDomain?: string
  tags: string[]
  updatedAt: number
}

export interface SecurityProfileControls {
  requireNodePool: boolean
  requireDnsProfile: boolean
  minRuntimeSupportedNameservers?: number
  allowedRoutingIntents: AppRoutingIntent[]
}

export interface SecurityProfile {
  profileId: string
  name: string
  controls: SecurityProfileControls
  tags: string[]
  updatedAt: number
}

export interface AppPolicyBinding {
  bindingId: string
  appId: string
  nodePoolId?: string
  dnsProfileId?: string
  securityProfileId?: string
  routingIntent: AppRoutingIntent
  enabled: boolean
  updatedAt: number
}

export interface AppRuntimeStateDocument {
  apps: AppRegistryEntry[]
  nodePools: NodePool[]
  dnsProfiles: DnsProfile[]
  securityProfiles: SecurityProfile[]
  policyBindings: AppPolicyBinding[]
  sessions: AppRuntimeSessionRecord[]
  runtimeApplyAudits: AppRuntimeProjectionRuntimeApplyAuditRecord[]
  activeProjection?: AppRuntimeActiveProjectionRecord
}

export interface AppRuntimeActiveProjectionRecord {
  artifactId: string
  appId: string
  checksum: string
  storagePath: string
  activatedAt: number
  activationKind: string
  mutatesRuntime: boolean
  rollback: AppRuntimeProjectionRollbackMetadata
}

export type AppRuntimeDnsHandoffStatus =
  | 'accepted'
  | 'watching'
  | 'rollbackRecommended'
  | 'blocked'

export type AppRuntimeControlPlaneCompletionStatus =
  | 'ready'
  | 'degraded'
  | 'blocked'

export type AppRuntimeStagedActivationLifecycleStatus =
  | 'ready'
  | 'degraded'
  | 'blocked'

export type AppRuntimeStagedActivationCloseoutStatus =
  | 'complete'
  | 'degraded'
  | 'blocked'

export interface AppRuntimeDnsHandoffRecord {
  handoffId: string
  action: string
  dnsCompletionStatus: DnsDefaultRuntimeExpandedControlPlaneCompletionStatus
  dnsControlPlaneComplete: boolean
  dnsHandoffReady: boolean
  dnsManifestPath?: string | null
  appRuntimeAcceptsHandoff: boolean
  appRuntimeFollowupScope: string
  nextAppRuntimeStep: string
  phase8Allowed: boolean
  promotionAllowed: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  createdAt: number
}

export interface AppRuntimeDnsHandoffReport {
  status: AppRuntimeDnsHandoffStatus
  reason: string
  dnsCompletion: DnsDefaultRuntimeExpandedControlPlaneCompletionReport
  handoffRecord: AppRuntimeDnsHandoffRecord
  handoffRecordPath?: string | null
  handoffRecordPersisted: boolean
  appRuntimeAcceptsHandoff: boolean
  nextAppRuntimeStep: string
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

export interface AppRuntimeControlPlaneCompletionReport {
  status: AppRuntimeControlPlaneCompletionStatus
  reason: string
  appId: string
  dnsHandoff: AppRuntimeDnsHandoffReport
  projectionArtifact: AppRuntimeProjectionArtifact
  projectionArtifactPath?: string | null
  projectionArtifactPersisted: boolean
  activationPreflight: AppRuntimeProjectionActivationPreflightReport
  readyForStagedActivation: boolean
  runtimeApplyAllowed: boolean
  phase8Allowed: boolean
  promotionAllowed: boolean
  userTriggerRequired: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  nextAppRuntimeStep: string
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface AppRuntimeStagedActivationLifecycleReport {
  status: AppRuntimeStagedActivationLifecycleStatus
  reason: string
  appId: string
  controlPlaneCompletion: AppRuntimeControlPlaneCompletionReport
  activeProjection?: AppRuntimeActiveProjectionRecord | null
  markerActivated: boolean
  activeMarkerMatchesArtifact: boolean
  rollbackBoundaryAvailable: boolean
  rollbackStrategy?: string | null
  runtimeApplyAllowed: boolean
  phase8Allowed: boolean
  promotionAllowed: boolean
  userTriggerRequired: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  nextAppRuntimeStep: string
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface AppRuntimeRuntimeApplyBoundaryManifest {
  manifestId: string
  appId: string
  artifactId: string
  checksum: string
  activeMarkerMatchesArtifact: boolean
  rollbackBoundaryAvailable: boolean
  rollbackStrategy?: string | null
  runtimeApplyAllowed: boolean
  phase8Allowed: boolean
  promotionAllowed: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  nextAppRuntimeStep: string
  createdAt: number
}

export interface AppRuntimeStagedActivationCloseoutReport {
  status: AppRuntimeStagedActivationCloseoutStatus
  reason: string
  lifecycle: AppRuntimeStagedActivationLifecycleReport
  boundaryManifest: AppRuntimeRuntimeApplyBoundaryManifest
  boundaryManifestPath?: string | null
  boundaryManifestPersisted: boolean
  closeoutComplete: boolean
  runtimeApplyAllowed: boolean
  phase8Allowed: boolean
  promotionAllowed: boolean
  userTriggerRequired: boolean
  autoRollout: boolean
  autoRollback: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  nextAppRuntimeStep: string
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface AppRuntimeProjectionRollbackMetadata {
  previousArtifactId?: string
  previousChecksum?: string
  previousStoragePath?: string
  capturedAt: number
  rollbackStrategy: string
}

export interface AppRuntimePlanRequest {
  appId: string
  sessionId?: string
}

export interface NodePoolPlanView {
  poolId: string
  name: string
  candidateCount: number
  protocols: string[]
  tags: string[]
  constraints: NodePoolHealthConstraints
  candidates: NodePoolCandidate[]
}

export interface DnsProfilePlanView {
  profileId: string
  name: string
  testDomain?: string | null
  tags: string[]
  resolverPlan: DnsResolverPlan
}

export interface SecurityProfilePlanView {
  profileId: string
  name: string
  controls: SecurityProfileControls
  tags: string[]
}

export interface RuntimeProjectionPlan {
  status: 'planningOnly'
  backend: string
  mutatesRuntime: boolean
  outputs: string[]
}

export interface MihomoRuleProjection {
  matcher: string
  value: string
  target: string
  rule: string
}

export interface MihomoProxyGroupProjection {
  name: string
  type: string
  proxies: string[]
  url?: string
  interval?: number
}

export interface MihomoDnsProjection {
  profileId: string
  name: string
  nameservers: string[]
  runtimeSupportedNameservers: number
}

export interface AppRuntimePlan {
  status: 'ready' | 'rejected'
  reason: string
  appId: string
  sessionId?: string
  app?: AppRegistryEntry
  policyBinding?: AppPolicyBinding
  nodePool?: NodePoolPlanView
  dnsProfile?: DnsProfilePlanView
  securityProfile?: SecurityProfilePlanView
  routingIntent?: AppRoutingIntent
  projection: RuntimeProjectionPlan
  facts: string[]
  warnings: string[]
}

export interface AppRuntimeMihomoProjection {
  status: 'ready' | 'rejected'
  reason: string
  appId: string
  sessionId?: string
  mutatesRuntime: boolean
  proxyGroups: MihomoProxyGroupProjection[]
  rules: MihomoRuleProjection[]
  dns?: MihomoDnsProjection
  yamlPatch: string
  facts: string[]
  warnings: string[]
}

export type AppRuntimeDiagnosticStatus = 'healthy' | 'degraded' | 'blocked'

export type AppRuntimeDiagnosticSeverity = 'info' | 'warning' | 'error'

export type AppRuntimeDiagnosticCheckStatus =
  | 'passed'
  | 'warning'
  | 'failed'
  | 'skipped'

export type AppRuntimeDiagnosticCategory =
  | 'registry'
  | 'policyBinding'
  | 'nodePool'
  | 'dns'
  | 'security'
  | 'projection'
  | 'runtimeBoundary'

export interface AppRuntimeDiagnosticCheck {
  checkId: string
  category: AppRuntimeDiagnosticCategory
  severity: AppRuntimeDiagnosticSeverity
  status: AppRuntimeDiagnosticCheckStatus
  message: string
  details: string[]
}

export interface AppRuntimeDiagnosticsSummary {
  passed: number
  warnings: number
  failed: number
  skipped: number
}

export interface AppRuntimeDiagnosticsReport {
  status: AppRuntimeDiagnosticStatus
  reason: string
  appId: string
  sessionId?: string
  plan: AppRuntimePlan
  mihomoProjection: AppRuntimeMihomoProjection
  checks: AppRuntimeDiagnosticCheck[]
  summary: AppRuntimeDiagnosticsSummary
  facts: string[]
  warnings: string[]
}

export type AppRuntimeProjectionActivationMode = 'staged'

export interface AppRuntimeProjectionValidationReport {
  status: AppRuntimeDiagnosticStatus
  reason: string
  checks: AppRuntimeDiagnosticCheck[]
  summary: AppRuntimeDiagnosticsSummary
  facts: string[]
  warnings: string[]
}

export interface AppRuntimeProjectionArtifact {
  artifactId: string
  appId: string
  sessionId?: string
  bindingId?: string
  nodePoolId?: string
  dnsProfileId?: string
  securityProfileId?: string
  generatedAt: number
  storagePath?: string
  activationMode: AppRuntimeProjectionActivationMode
  mutatesRuntime: boolean
  checksum: string
  plan: AppRuntimePlan
  projection: AppRuntimeMihomoProjection
  diagnostics: AppRuntimeDiagnosticsReport
  validation: AppRuntimeProjectionValidationReport
  facts: string[]
  warnings: string[]
}

export interface AppRuntimeProjectionActivationPreflightRequest {
  artifactId: string
  expectedChecksum?: string
}

export interface AppRuntimeProjectionRuntimeApplyRequest {
  artifactId: string
  expectedChecksum?: string
  force?: boolean
}

export interface AppRuntimeProjectionRuntimeVerificationRequest {
  artifactId?: string
}

export type AppRuntimeProjectionRuntimeApplyAuditStatus =
  | 'active'
  | 'rolledBack'
  | 'superseded'

export interface AppRuntimeProjectionRuntimeApplyCandidateSummary {
  profileItemUid: string
  profileItemFile: string
  proxyGroupCount: number
  ruleCount: number
  dnsProfileProjected: boolean
}

export interface AppRuntimeProjectionRuntimeApplyMarkerSnapshot {
  artifactId: string
  checksum: string
  storagePath: string
  activationKind: string
  mutatesRuntime: boolean
  activatedAt: number
}

export interface AppRuntimeProjectionRuntimeApplyAuditRecord {
  auditId: string
  artifactId: string
  appId: string
  checksum: string
  activationKind: string
  appliedAt: number
  validationOutcome: string
  candidateSummary: AppRuntimeProjectionRuntimeApplyCandidateSummary
  previousMarker?: AppRuntimeProjectionRuntimeApplyMarkerSnapshot
  rollbackStrategy: string
  status: AppRuntimeProjectionRuntimeApplyAuditStatus
  statusUpdatedAt: number
  latestVerificationStatus?: AppRuntimeDiagnosticStatus
  latestVerificationReason?: string
  latestVerificationAt?: number
}

export interface AppRuntimeProjectionRuntimeVerificationReport {
  status: AppRuntimeDiagnosticStatus
  reason: string
  artifactId?: string
  checksum?: string
  auditId?: string
  observedAt: number
  checks: AppRuntimeDiagnosticCheck[]
  summary: AppRuntimeDiagnosticsSummary
  facts: string[]
  warnings: string[]
}

export interface AppRuntimeProjectionActivationPreflightReport {
  status: AppRuntimeDiagnosticStatus
  reason: string
  artifactId: string
  appId?: string
  checksum?: string
  storagePath?: string
  activationMode?: AppRuntimeProjectionActivationMode
  mutatesRuntime?: boolean
  checks: AppRuntimeDiagnosticCheck[]
  summary: AppRuntimeDiagnosticsSummary
  facts: string[]
  warnings: string[]
}

export type AppRuntimeSessionStatus =
  | 'planned'
  | 'blocked'
  | 'completed'
  | 'failed'

export type AppRuntimeSessionObservationSource = 'connectionMetricsSnapshot'

export type AppRuntimeSessionAttributionStatus =
  | 'unattributed'
  | 'appMatched'
  | 'appMismatch'

export interface AppRuntimeSessionTrafficObservation {
  uploadTotal: number
  downloadTotal: number
  uploadSpeed: number
  downloadSpeed: number
  activeConnectionCount: number
  closedSinceLast: number
  memory: number
  stale: boolean
}

export interface AppRuntimeSessionAttributionCandidate {
  connectionId: string
  process: string
  processPath: string
  host: string
  rule: string
  rulePayload: string
  chains: string[]
  upload: number
  download: number
  matchedBy: string[]
}

export interface AppRuntimeSessionObservationRecord {
  observationId: string
  sessionId: string
  recordedAt: number
  source: AppRuntimeSessionObservationSource
  attributionStatus: AppRuntimeSessionAttributionStatus
  traffic: AppRuntimeSessionTrafficObservation
  connectionSpeedCount: number
  attributionCandidates: AppRuntimeSessionAttributionCandidate[]
  facts: string[]
  warnings: string[]
}

export interface AppRuntimeSessionEvaluationSummary {
  observationCount: number
  matchedObservations: number
  mismatchObservations: number
  unattributedObservations: number
  staleObservations: number
  attributionCandidateCount: number
  uploadTotal: number
  downloadTotal: number
  maxActiveConnections: number
  observedChains: string[]
  observedHosts: string[]
  matchedBy: string[]
}

export interface AppRuntimeSessionEvaluationReport {
  sessionId: string
  appId: string
  status: AppRuntimeDiagnosticStatus
  reason: string
  summary: AppRuntimeSessionEvaluationSummary
  facts: string[]
  warnings: string[]
}

export type AppRuntimeLeakDimension =
  | 'proxyLeak'
  | 'dnsLeak'
  | 'exitVerification'
  | 'nodePoolConsistency'

export type AppRuntimeLeakCheckStatus =
  | 'pass'
  | 'warn'
  | 'fail'
  | 'notApplicable'

export interface AppRuntimeLeakCheck {
  dimension: AppRuntimeLeakDimension
  status: AppRuntimeLeakCheckStatus
  severity: AppRuntimeDiagnosticSeverity
  message: string
  facts: string[]
  warnings: string[]
}

export interface AppRuntimeLeakSummary {
  pass: number
  warn: number
  fail: number
  notApplicable: number
}

export interface AppRuntimeSessionLeakReport {
  sessionId: string
  appId: string
  status: AppRuntimeDiagnosticStatus
  reason: string
  routingIntent?: AppRoutingIntent
  evaluationSummary: AppRuntimeSessionEvaluationSummary
  checks: AppRuntimeLeakCheck[]
  summary: AppRuntimeLeakSummary
  facts: string[]
  warnings: string[]
}

export interface AppRuntimeSessionRecord {
  sessionId: string
  appId: string
  status: AppRuntimeSessionStatus
  planStatus: 'ready' | 'rejected'
  diagnosticsStatus: AppRuntimeDiagnosticStatus
  diagnosticsSummary: AppRuntimeDiagnosticsSummary
  reason: string
  startedAt: number
  endedAt?: number
  projectedRules: string[]
  projectedProxyGroups: string[]
  observations: AppRuntimeSessionObservationRecord[]
  facts: string[]
  warnings: string[]
}

export interface AppRuntimeSessionStartReport {
  session: AppRuntimeSessionRecord
  diagnostics: AppRuntimeDiagnosticsReport
}

export interface AppRuntimeSessionFinishRequest {
  sessionId: string
  status: Exclude<AppRuntimeSessionStatus, 'planned'>
  reason?: string
}

export async function getAppRuntimeState(): Promise<AppRuntimeStateDocument> {
  return invoke('get_app_runtime_state')
}

export async function buildAppRuntimeDemoSeed(): Promise<AppRuntimeStateDocument> {
  return invoke('build_app_runtime_demo_seed')
}

export async function acceptAppRuntimeDnsHandoff(): Promise<AppRuntimeDnsHandoffReport> {
  return invoke('accept_app_runtime_dns_handoff')
}

export async function completeAppRuntimeControlPlane(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimeControlPlaneCompletionReport> {
  return invoke('complete_app_runtime_control_plane', { request })
}

export async function completeAppRuntimeStagedActivationLifecycle(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimeStagedActivationLifecycleReport> {
  return invoke('complete_app_runtime_staged_activation_lifecycle', { request })
}

export async function closeoutAppRuntimeStagedActivationLifecycle(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimeStagedActivationCloseoutReport> {
  return invoke('closeout_app_runtime_staged_activation_lifecycle', { request })
}

export async function upsertAppRegistryEntry(
  entry: AppRegistryEntry,
): Promise<AppRuntimeStateDocument> {
  return invoke('upsert_app_registry_entry', { entry })
}

export async function deleteAppRegistryEntry(
  appId: string,
): Promise<AppRuntimeStateDocument> {
  return invoke('delete_app_registry_entry', { appId })
}

export async function upsertNodePool(
  nodePool: NodePool,
): Promise<AppRuntimeStateDocument> {
  return invoke('upsert_node_pool', { nodePool })
}

export async function deleteNodePool(
  poolId: string,
): Promise<AppRuntimeStateDocument> {
  return invoke('delete_node_pool', { poolId })
}

export async function upsertDnsProfile(
  dnsProfile: DnsProfile,
): Promise<AppRuntimeStateDocument> {
  return invoke('upsert_dns_profile', { dnsProfile })
}

export async function deleteDnsProfile(
  profileId: string,
): Promise<AppRuntimeStateDocument> {
  return invoke('delete_dns_profile', { profileId })
}

export async function upsertSecurityProfile(
  securityProfile: SecurityProfile,
): Promise<AppRuntimeStateDocument> {
  return invoke('upsert_security_profile', { securityProfile })
}

export async function deleteSecurityProfile(
  profileId: string,
): Promise<AppRuntimeStateDocument> {
  return invoke('delete_security_profile', { profileId })
}

export async function upsertAppPolicyBinding(
  binding: AppPolicyBinding,
): Promise<AppRuntimeStateDocument> {
  return invoke('upsert_app_policy_binding', { binding })
}

export async function deleteAppPolicyBinding(
  bindingId: string,
): Promise<AppRuntimeStateDocument> {
  return invoke('delete_app_policy_binding', { bindingId })
}

export async function explainAppRuntimePlan(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimePlan> {
  return invoke('explain_app_runtime_plan', { request })
}

export async function projectAppRuntimePlanToMihomo(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimeMihomoProjection> {
  return invoke('project_app_runtime_plan_to_mihomo', { request })
}

export async function diagnoseAppRuntime(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimeDiagnosticsReport> {
  return invoke('diagnose_app_runtime', { request })
}

export async function buildAppRuntimeProjectionArtifact(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimeProjectionArtifact> {
  return invoke('build_app_runtime_projection_artifact', { request })
}

export async function preflightAppRuntimeProjectionActivation(
  request: AppRuntimeProjectionActivationPreflightRequest,
): Promise<AppRuntimeProjectionActivationPreflightReport> {
  return invoke('preflight_app_runtime_projection_activation', { request })
}

export async function activateAppRuntimeProjectionArtifact(
  request: AppRuntimeProjectionActivationPreflightRequest,
): Promise<AppRuntimeStateDocument> {
  return invoke('activate_app_runtime_projection_artifact', { request })
}

export async function applyAppRuntimeProjectionArtifactToRuntime(
  request: AppRuntimeProjectionRuntimeApplyRequest,
): Promise<AppRuntimeStateDocument> {
  return invoke('apply_app_runtime_projection_artifact_to_runtime', { request })
}

export async function listAppRuntimeProjectionRuntimeApplyAudits(
  artifactId?: string,
): Promise<AppRuntimeProjectionRuntimeApplyAuditRecord[]> {
  return invoke('list_app_runtime_projection_runtime_apply_audits', {
    artifactId,
  })
}

export async function verifyAppRuntimeProjectionRuntimeApply(
  request: AppRuntimeProjectionRuntimeVerificationRequest,
): Promise<AppRuntimeProjectionRuntimeVerificationReport> {
  return invoke('verify_app_runtime_projection_runtime_apply', { request })
}

export async function rollbackAppRuntimeProjectionActivation(): Promise<AppRuntimeStateDocument> {
  return invoke('rollback_app_runtime_projection_activation')
}

export async function listAppRuntimeSessions(
  appId?: string,
): Promise<AppRuntimeSessionRecord[]> {
  return invoke('list_app_runtime_sessions', { appId })
}

export async function startAppRuntimeSession(
  request: AppRuntimePlanRequest,
): Promise<AppRuntimeSessionStartReport> {
  return invoke('start_app_runtime_session', { request })
}

export async function finishAppRuntimeSession(
  request: AppRuntimeSessionFinishRequest,
): Promise<AppRuntimeSessionRecord> {
  return invoke('finish_app_runtime_session', { request })
}

export async function recordAppRuntimeSessionObservation(
  sessionId: string,
): Promise<AppRuntimeSessionRecord> {
  return invoke('record_app_runtime_session_observation', { sessionId })
}

export async function evaluateAppRuntimeSession(
  sessionId: string,
): Promise<AppRuntimeSessionEvaluationReport> {
  return invoke('evaluate_app_runtime_session', { sessionId })
}

export async function verifyAppRuntimeSessionLeak(
  sessionId: string,
): Promise<AppRuntimeSessionLeakReport> {
  return invoke('verify_app_runtime_session_leak', { sessionId })
}
