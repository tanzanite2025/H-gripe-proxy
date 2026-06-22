import { invoke } from '@tauri-apps/api/core'

import type { DnsDefaultRuntimeShadowEvidenceReport } from '../dns-api'

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

export interface RuntimeKernelLoopbackR5DefaultCutoverCloseoutReport {
  runtimeId: string
  component: string
  kernelArea: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  currentPlatform: string
  currentArch: string
  r5CloseoutReportDecision: boolean
  r5CloseoutComplete: boolean
  defaultCutoverAllowed: boolean
  expandedOptInAllowed: boolean
  closeoutReadiness: RuntimeKernelLoopbackR5DefaultCutoverCloseoutReadinessReport
  completedEvidenceBatches: string[]
  openBoundaries: string[]
  passed: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export type RuntimeKernelRuntimeKind = 'mihomo' | 'rust'

export interface RuntimeKernelRuntimeCapability {
  name: string
  status: string
  supported: boolean
  fallbackRequired: boolean
  facts: string[]
}

export interface RuntimeRustKernelRuntimeCandidateReport {
  runtimeId: string
  kind: RuntimeKernelRuntimeKind
  mutatesRuntime: boolean
  selectable: boolean
  defaultAllowed: boolean
  mihomoFallback: boolean
  supportedSafeSubset: string[]
  fallbackBoundaries: string[]
  capabilities: RuntimeKernelRuntimeCapability[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelRuntimeSelectionScaffoldReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  currentDefaultRuntimeKind: RuntimeKernelRuntimeKind
  requestedRuntimeKind: RuntimeKernelRuntimeKind
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rustRuntimeOptInDecision: boolean
  rustCandidateAvailable: boolean
  rustCandidateDefaultAllowed: boolean
  mihomoFallback: boolean
  rustCandidate: RuntimeRustKernelRuntimeCandidateReport
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeKernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  rustRuntimeScaffoldDecision: boolean
  scaffoldReady: boolean
  defaultCutoverAllowed: boolean
  r5Closeout: RuntimeKernelLoopbackR5DefaultCutoverCloseoutReport
  runtimeSelection: RuntimeKernelRuntimeSelectionScaffoldReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeSupportedSubsetReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  ruleDecisionOwned: boolean
  dnsDecisionOwned: boolean
  adapterDecisionOwned: boolean
  forwardingSurfaceOwned: boolean
  appRuleCount: number
  appProxyCount: number
  supportedSubset: string[]
  fallbackBoundaries: string[]
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface RuntimeRustKernelRuntimeHealthStateReport {
  runtimeId: string
  component: string
  status: string
  healthReady: boolean
  rollbackArmed: boolean
  mihomoFallback: boolean
  observedChecks: string[]
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR6OptInRustRuntimeMvpReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  rustRuntimeOptInDecision: boolean
  requestedRuntimeKind: RuntimeKernelRuntimeKind
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  optInReady: boolean
  defaultCutoverAllowed: boolean
  mihomoFallback: boolean
  scaffold: RuntimeKernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport
  supportedSubset: RuntimeRustKernelRuntimeSupportedSubsetReport
  healthState: RuntimeRustKernelRuntimeHealthStateReport
  loopbackForwardingEvidence?: RuntimeKernelLoopbackForwardingRollbackDrillReport | null
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeCanaryProfileReport {
  runtimeId: string
  component: string
  canaryScope: string
  maxCanarySessions: number
  cappedProfile: boolean
  supportedSafeSubset: string[]
  fallbackBoundaries: string[]
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface RuntimeRustKernelRuntimeAutomaticFallbackReport {
  runtimeId: string
  component: string
  healthCheckPassed: boolean
  rollbackTriggered: boolean
  healthReady: boolean
  rollbackArmed: boolean
  fallbackActivated: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  fallbackRuntimeKind: RuntimeKernelRuntimeKind
  triggers: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR6RustDefaultCanaryReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  rustRuntimeOptInDecision: boolean
  canaryDefaultDecision: boolean
  requestedRuntimeKind: RuntimeKernelRuntimeKind
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  canaryDefaultAllowed: boolean
  productionDefaultAllowed: boolean
  mihomoFallback: boolean
  r6OptIn: RuntimeKernelLoopbackR6OptInRustRuntimeMvpReport
  canaryProfile: RuntimeRustKernelRuntimeCanaryProfileReport
  automaticFallback: RuntimeRustKernelRuntimeAutomaticFallbackReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeCanaryCloseoutSummaryReport {
  runtimeId: string
  component: string
  canaryDefaultAllowed: boolean
  canaryHealthReady: boolean
  automaticFallbackArmed: boolean
  rollbackHoldPassed: boolean
  closeoutReady: boolean
  evidence: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeRustKernelRuntimeSupportedProfileDefaultReport {
  runtimeId: string
  component: string
  profileScope: string
  supportedProfileDefault: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  fallbackRuntimeKind: RuntimeKernelRuntimeKind
  supportedSafeSubset: string[]
  fallbackBoundaries: string[]
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface RuntimeRustKernelRuntimeFallbackStateReport {
  runtimeId: string
  component: string
  rollbackSwitchRequested: boolean
  restartRequired: boolean
  healthReady: boolean
  rollbackArmed: boolean
  fallbackActive: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  fallbackRuntimeKind: RuntimeKernelRuntimeKind
  triggers: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR7RustDefaultCutoverReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  rustRuntimeOptInDecision: boolean
  canaryDefaultDecision: boolean
  r7CutoverDecision: boolean
  rollbackHoldDecision: boolean
  rollbackSwitchRequested: boolean
  requestedRuntimeKind: RuntimeKernelRuntimeKind
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  supportedProfileDefaultAllowed: boolean
  productionDefaultAllowed: boolean
  mihomoFallback: boolean
  r6Canary: RuntimeKernelLoopbackR6RustDefaultCanaryReport
  canaryCloseout: RuntimeRustKernelRuntimeCanaryCloseoutSummaryReport
  supportedProfile: RuntimeRustKernelRuntimeSupportedProfileDefaultReport
  fallbackState: RuntimeRustKernelRuntimeFallbackStateReport
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeFallbackRetirementParityReport {
  runtimeId: string
  component: string
  protocolParityPassed: boolean
  tunParityPassed: boolean
  adapterParityPassed: boolean
  dnsRuntimeParityPassed: boolean
  crossPlatformRollbackPassed: boolean
  soakEvidencePassed: boolean
  parityComplete: boolean
  retainedBoundaries: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeRustKernelRuntimeFallbackRetirementPlanReport {
  runtimeId: string
  component: string
  fallbackRetirementDecision: boolean
  emergencyRollbackDecision: boolean
  rollbackSwitchRequested: boolean
  fallbackRetirementAllowed: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  restartRequired: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackR7MihomoFallbackRetirementReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  r7Cutover: RuntimeKernelLoopbackR7RustDefaultCutoverReport
  parity: RuntimeRustKernelRuntimeFallbackRetirementParityReport
  retirementPlan: RuntimeRustKernelRuntimeFallbackRetirementPlanReport
  productionDefaultAllowed: boolean
  mihomoFallbackRetired: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeExtendedSoakReport {
  runtimeId: string
  component: string
  minSoakHours: number
  observedSoakHours: number
  healthRegressionCount: number
  rollbackTriggerCount: number
  soakComplete: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeRustKernelRuntimeRollbackTelemetryReport {
  runtimeId: string
  component: string
  rollbackTelemetryDecision: boolean
  emergencyRollbackReady: boolean
  rollbackEventCount: number
  lastRollbackEventTs?: number | null
  telemetryComplete: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeRustKernelRuntimePlatformHardeningFollowUpReport {
  runtimeId: string
  component: string
  windowsServiceHardening: boolean
  macosServiceHardening: boolean
  linuxServiceHardening: boolean
  platformFollowUpComplete: boolean
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackFullRustRuntimeHardeningReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  hardeningDecision: boolean
  r7FallbackRetirementPassed: boolean
  extendedSoak: RuntimeRustKernelRuntimeExtendedSoakReport
  rollbackTelemetry: RuntimeRustKernelRuntimeRollbackTelemetryReport
  platformFollowUp: RuntimeRustKernelRuntimePlatformHardeningFollowUpReport
  fullRustRuntimeHardened: boolean
  productionDefaultAllowed: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementSurfaceAuditReport {
  runtimeId: string
  component: string
  sidecarSourceAuditPassed: boolean
  bundledMihomoAuditPassed: boolean
  ipcFallbackAuditPassed: boolean
  docsAuditPassed: boolean
  emergencyRollbackRetained: boolean
  auditComplete: boolean
  remainingSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementAuditReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  fullRustRuntimeHardened: boolean
  surfaceAudit: RuntimeRustKernelRuntimeGoMihomoRetirementSurfaceAuditReport
  finalRetirementAuditDecision: boolean
  goMihomoRetirementAuditComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementRemovalPlanReport {
  runtimeId: string
  component: string
  sidecarSourceRemovalPlan: boolean
  bundledArtifactDeprecationPlan: boolean
  ipcFallbackReplacementPlan: boolean
  emergencyRollbackPreservationPlan: boolean
  releaseRolloutPlan: boolean
  removalPlanComplete: boolean
  plannedRemovalSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementPlanReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  goMihomoRetirementAuditComplete: boolean
  removalPlan: RuntimeRustKernelRuntimeGoMihomoRetirementRemovalPlanReport
  finalRetirementPlanDecision: boolean
  goMihomoRetirementPlanComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementExecutionGuardReport {
  runtimeId: string
  component: string
  removalManifestReady: boolean
  abortPlanReady: boolean
  stagedRolloutGuardReady: boolean
  emergencyRollbackDrillPassed: boolean
  operatorAcknowledgement: boolean
  executionGuardComplete: boolean
  guardedExecutionSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementExecutionGuardReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  goMihomoRetirementPlanComplete: boolean
  executionGuard: RuntimeRustKernelRuntimeGoMihomoRetirementExecutionGuardReport
  finalExecutionGuardDecision: boolean
  goMihomoRetirementExecutionGuardComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementDryRunReport {
  runtimeId: string
  component: string
  dryRunManifestReplayed: boolean
  noSourceMutationsObserved: boolean
  noBundledArtifactMutationsObserved: boolean
  rollbackRehearsalPassed: boolean
  dryRunReportArchived: boolean
  dryRunComplete: boolean
  simulatedRemovalSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementDryRunReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  goMihomoRetirementExecutionGuardComplete: boolean
  dryRun: RuntimeRustKernelRuntimeGoMihomoRetirementDryRunReport
  finalDryRunDecision: boolean
  goMihomoRetirementDryRunComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementCloseoutReport {
  runtimeId: string
  component: string
  dryRunEvidenceReviewed: boolean
  closeoutReportArchived: boolean
  rollbackCheckpointVerified: boolean
  artifactInventoryFrozen: boolean
  noRemovalMutationsObserved: boolean
  closeoutComplete: boolean
  closedOutSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementCloseoutReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  goMihomoRetirementDryRunComplete: boolean
  closeout: RuntimeRustKernelRuntimeGoMihomoRetirementCloseoutReport
  finalCloseoutDecision: boolean
  goMihomoRetirementCloseoutComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementFinalRemovalGateReport {
  runtimeId: string
  component: string
  closeoutEvidenceAccepted: boolean
  rollbackBoundaryLocked: boolean
  removalScopeLocked: boolean
  releaseBlockerReviewPassed: boolean
  finalOperatorApproval: boolean
  finalRemovalGateComplete: boolean
  approvedRemovalSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementFinalRemovalGateReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  goMihomoRetirementCloseoutComplete: boolean
  finalRemovalGate: RuntimeRustKernelRuntimeGoMihomoRetirementFinalRemovalGateReport
  finalRemovalDecision: boolean
  goMihomoRetirementFinalRemovalGateComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementExecutionReport {
  runtimeId: string
  component: string
  rollbackCheckpointCreated: boolean
  executionManifestApplied: boolean
  sourceRemovalRecorded: boolean
  artifactRemovalRecorded: boolean
  postExecutionValidationPassed: boolean
  executionComplete: boolean
  executedSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementExecutionReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  goMihomoRetirementFinalRemovalGateComplete: boolean
  execution: RuntimeRustKernelRuntimeGoMihomoRetirementExecutionReport
  finalExecutionDecision: boolean
  goMihomoRetirementExecutionComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementPostExecutionVerificationReport {
  runtimeId: string
  component: string
  rustOnlyBoundaryVerified: boolean
  rollbackCheckpointRetained: boolean
  sourceRemovalVerified: boolean
  artifactRemovalVerified: boolean
  fallbackIpcAbsenceVerified: boolean
  postExecutionVerificationComplete: boolean
  verifiedSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementPostExecutionVerificationReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  goMihomoRetirementExecutionComplete: boolean
  postExecutionVerification: RuntimeRustKernelRuntimeGoMihomoRetirementPostExecutionVerificationReport
  finalVerificationDecision: boolean
  goMihomoRetirementPostExecutionVerificationComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementRollbackSurfaceRetirementReport {
  runtimeId: string
  component: string
  postExecutionVerificationReviewed: boolean
  replacementRecoveryPathVerified: boolean
  rollbackSurfaceInventoryLocked: boolean
  rollbackSurfaceRetirementPlanArchived: boolean
  emergencyRecoveryDrillPassed: boolean
  rollbackSurfaceRetirementComplete: boolean
  plannedRetirementSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementRollbackSurfaceRetirementReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  goMihomoRetirementPostExecutionVerificationComplete: boolean
  rollbackSurfaceRetirement: RuntimeRustKernelRuntimeGoMihomoRetirementRollbackSurfaceRetirementReport
  finalRollbackSurfaceRetirementDecision: boolean
  goMihomoRetirementRollbackSurfaceRetirementComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeGoMihomoRetirementCompletionCloseoutReport {
  runtimeId: string
  component: string
  rollbackSurfaceRetirementReviewed: boolean
  recoveryBoundaryEvidenceRetained: boolean
  completionReportArchived: boolean
  releaseNotesUpdated: boolean
  migrationStateFrozen: boolean
  completionCloseoutComplete: boolean
  closeoutSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackGoMihomoRetirementCompletionCloseoutReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  goMihomoRetirementRollbackSurfaceRetirementComplete: boolean
  completionCloseout: RuntimeRustKernelRuntimeGoMihomoRetirementCompletionCloseoutReport
  finalCompletionDecision: boolean
  goMihomoRetirementCompletionCloseoutComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningBoundaryReport {
  runtimeId: string
  component: string
  protocolParityInventoryComplete: boolean
  tunBoundaryInventoryComplete: boolean
  adapterCompatibilityMatrixComplete: boolean
  dnsLeakVerificationPlanComplete: boolean
  rollbackDrillPlanComplete: boolean
  optInExecutionBoundaryLocked: boolean
  preflightBoundaryComplete: boolean
  evidenceSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningPreflightReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  goMihomoRetirementComplete: boolean
  boundary: RuntimeRustKernelRuntimeDataPlaneHardeningBoundaryReport
  finalPreflightDecision: boolean
  rustDataPlaneHardeningPreflightComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningBoundaryAuditReport {
  runtimeId: string
  component: string
  preflightReviewed: boolean
  protocolBoundaryAudited: boolean
  tunBoundaryAudited: boolean
  adapterBoundaryAudited: boolean
  dnsLeakBoundaryAudited: boolean
  rollbackBoundaryAudited: boolean
  optInBoundaryAudited: boolean
  boundaryAuditComplete: boolean
  auditedSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningBoundaryAuditReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningPreflightComplete: boolean
  boundaryAudit: RuntimeRustKernelRuntimeDataPlaneHardeningBoundaryAuditReport
  finalBoundaryAuditDecision: boolean
  rustDataPlaneHardeningBoundaryAuditComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningOptInExecutionGuardReport {
  runtimeId: string
  component: string
  boundaryAuditReviewed: boolean
  optInScopeLocked: boolean
  rolloutGuardDefined: boolean
  abortPlanApproved: boolean
  telemetryWatchConfigured: boolean
  rollbackSwitchVerified: boolean
  operatorAcknowledged: boolean
  optInExecutionGuardComplete: boolean
  guardedSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningOptInExecutionGuardReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningBoundaryAuditComplete: boolean
  optInExecutionGuard: RuntimeRustKernelRuntimeDataPlaneHardeningOptInExecutionGuardReport
  finalExecutionGuardDecision: boolean
  rustDataPlaneHardeningOptInExecutionGuardComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningOptInDryRunReport {
  runtimeId: string
  component: string
  executionGuardReviewed: boolean
  dryRunScopeLocked: boolean
  manifestReplayCompleted: boolean
  syntheticFlowPlanCompleted: boolean
  leakWatchPlanVerified: boolean
  rollbackRehearsalCompleted: boolean
  productionForwardingUnchangedVerified: boolean
  dryRunEvidenceArchived: boolean
  optInDryRunComplete: boolean
  dryRunSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningOptInDryRunReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningOptInExecutionGuardComplete: boolean
  optInDryRun: RuntimeRustKernelRuntimeDataPlaneHardeningOptInDryRunReport
  finalDryRunDecision: boolean
  rustDataPlaneHardeningOptInDryRunComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningOptInExecutionReport {
  runtimeId: string
  component: string
  dryRunReviewed: boolean
  executionManifestLocked: boolean
  stagedOptInWindowDefined: boolean
  telemetryWatchActive: boolean
  rollbackSwitchArmed: boolean
  productionMutationGuardRetained: boolean
  operatorExecutionAcknowledged: boolean
  optInExecutionComplete: boolean
  executionSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningOptInExecutionReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningOptInDryRunComplete: boolean
  optInExecution: RuntimeRustKernelRuntimeDataPlaneHardeningOptInExecutionReport
  finalExecutionDecision: boolean
  rustDataPlaneHardeningOptInExecutionComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningOptInExecutionVerificationReport {
  runtimeId: string
  component: string
  executionRecordReviewed: boolean
  telemetrySampleReviewed: boolean
  rollbackReadinessVerified: boolean
  productionMutationGuardStillRetained: boolean
  productionForwardingUnchangedVerified: boolean
  leakRegressionAbsenceVerified: boolean
  verificationEvidenceArchived: boolean
  optInExecutionVerificationComplete: boolean
  verificationSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningOptInExecutionVerificationReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningOptInExecutionComplete: boolean
  optInExecutionVerification: RuntimeRustKernelRuntimeDataPlaneHardeningOptInExecutionVerificationReport
  finalVerificationDecision: boolean
  rustDataPlaneHardeningOptInExecutionVerificationComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutGuardReport {
  runtimeId: string
  component: string
  optInVerificationReviewed: boolean
  controlledRolloutScopeLocked: boolean
  canaryPopulationCapDefined: boolean
  healthRollbackTriggersDefined: boolean
  telemetryHoldWindowConfigured: boolean
  mihomoFallbackRetained: boolean
  productionMutationGuardRetained: boolean
  operatorRolloutGuardAcknowledged: boolean
  controlledRolloutGuardComplete: boolean
  guardedSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutGuardReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningOptInExecutionVerificationComplete: boolean
  controlledRolloutGuard: RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutGuardReport
  finalControlledRolloutGuardDecision: boolean
  rustDataPlaneHardeningControlledRolloutGuardComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutDryRunReport {
  runtimeId: string
  component: string
  guardReviewed: boolean
  dryRunManifestReplayed: boolean
  cappedCanarySimulationCompleted: boolean
  fallbackTriggerRehearsed: boolean
  telemetryHoldSampleReviewed: boolean
  rollbackSwitchRehearsed: boolean
  productionForwardingUnchangedVerified: boolean
  dryRunEvidenceArchived: boolean
  controlledRolloutDryRunComplete: boolean
  dryRunSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutDryRunReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningControlledRolloutGuardComplete: boolean
  controlledRolloutDryRun: RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutDryRunReport
  finalControlledRolloutDryRunDecision: boolean
  rustDataPlaneHardeningControlledRolloutDryRunComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutReadinessCloseoutReport {
  runtimeId: string
  component: string
  dryRunReviewed: boolean
  rolloutWindowApproved: boolean
  canaryPopulationCapEnforced: boolean
  automaticFallbackArmed: boolean
  telemetryWatchActive: boolean
  rollbackOwnerAcknowledged: boolean
  productionMutationGuardRetained: boolean
  closeoutEvidenceArchived: boolean
  controlledRolloutReadinessCloseoutComplete: boolean
  closeoutSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutReadinessCloseoutReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningControlledRolloutDryRunComplete: boolean
  controlledRolloutReadinessCloseout: RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutReadinessCloseoutReport
  finalControlledRolloutReadinessDecision: boolean
  rustDataPlaneHardeningControlledRolloutReadinessCloseoutComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryExecutionReport {
  runtimeId: string
  component: string
  readinessCloseoutReviewed: boolean
  executionManifestLocked: boolean
  canaryWindowStarted: boolean
  canaryPopulationCapEnforced: boolean
  healthTelemetryActive: boolean
  automaticFallbackArmed: boolean
  mihomoFallbackRetained: boolean
  productionMutationGuardRetained: boolean
  operatorCanaryExecutionAcknowledged: boolean
  controlledRolloutCanaryExecutionComplete: boolean
  executionSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryExecutionReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningControlledRolloutReadinessCloseoutComplete: boolean
  controlledRolloutCanaryExecution: RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryExecutionReport
  finalControlledRolloutCanaryExecutionDecision: boolean
  rustDataPlaneHardeningControlledRolloutCanaryExecutionComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryVerificationReport {
  runtimeId: string
  component: string
  executionRecordReviewed: boolean
  healthTelemetrySampleReviewed: boolean
  automaticFallbackResultReviewed: boolean
  unsupportedTrafficFallbackVerified: boolean
  leakRegressionAbsenceVerified: boolean
  rollbackReadinessVerified: boolean
  productionMutationGuardStillRetained: boolean
  verificationEvidenceArchived: boolean
  controlledRolloutCanaryVerificationComplete: boolean
  verificationSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryVerificationReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningControlledRolloutCanaryExecutionComplete: boolean
  controlledRolloutCanaryVerification: RuntimeRustKernelRuntimeDataPlaneHardeningControlledRolloutCanaryVerificationReport
  finalControlledRolloutCanaryVerificationDecision: boolean
  rustDataPlaneHardeningControlledRolloutCanaryVerificationComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionGuardReport {
  runtimeId: string
  component: string
  canaryVerificationReviewed: boolean
  supportedProfileScopeLocked: boolean
  fallbackMatrixRetained: boolean
  rollbackSwitchVerified: boolean
  telemetrySoakWindowDefined: boolean
  releaseBlockerReviewed: boolean
  productionMutationGuardRetained: boolean
  operatorPromotionAcknowledged: boolean
  supportedDefaultPromotionGuardComplete: boolean
  guardSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionGuardReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningControlledRolloutCanaryVerificationComplete: boolean
  supportedDefaultPromotionGuard: RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionGuardReport
  finalSupportedDefaultPromotionGuardDecision: boolean
  rustDataPlaneHardeningSupportedDefaultPromotionGuardComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionDryRunReport {
  runtimeId: string
  component: string
  guardReviewed: boolean
  defaultSelectionManifestReplayed: boolean
  supportedProfileSimulationCompleted: boolean
  fallbackDecisionRehearsed: boolean
  rollbackRehearsed: boolean
  productionForwardingUnchangedVerified: boolean
  dryRunEvidenceArchived: boolean
  supportedDefaultPromotionDryRunComplete: boolean
  dryRunSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionDryRunReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningSupportedDefaultPromotionGuardComplete: boolean
  supportedDefaultPromotionDryRun: RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultPromotionDryRunReport
  finalSupportedDefaultPromotionDryRunDecision: boolean
  rustDataPlaneHardeningSupportedDefaultPromotionDryRunComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverReport {
  runtimeId: string
  component: string
  dryRunReviewed: boolean
  cutoverManifestLocked: boolean
  supportedProfileDefaultSelectionConfirmed: boolean
  unsupportedPathsBoundToMihomoFallback: boolean
  rollbackSwitchArmed: boolean
  telemetrySoakWatchActive: boolean
  operatorCutoverAcknowledged: boolean
  productionMutationGuardTransitionRecorded: boolean
  supportedDefaultCutoverComplete: boolean
  cutoverSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningSupportedDefaultPromotionDryRunComplete: boolean
  supportedDefaultCutover: RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverReport
  finalSupportedDefaultCutoverDecision: boolean
  rustDataPlaneHardeningSupportedDefaultCutoverComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverVerificationReport {
  runtimeId: string
  component: string
  cutoverRecordReviewed: boolean
  supportedProfileTrafficSampleReviewed: boolean
  unsupportedPathFallbackVerified: boolean
  rollbackSwitchVerified: boolean
  telemetrySoakSampleReviewed: boolean
  leakRegressionAbsenceVerified: boolean
  mutationAuditRecordArchived: boolean
  cutoverVerificationComplete: boolean
  verificationSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverVerificationReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningSupportedDefaultCutoverComplete: boolean
  supportedDefaultCutoverVerification: RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverVerificationReport
  finalSupportedDefaultCutoverVerificationDecision: boolean
  rustDataPlaneHardeningSupportedDefaultCutoverVerificationComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport {
  runtimeId: string
  component: string
  verificationReviewed: boolean
  soakWindowElapsed: boolean
  healthBudgetSatisfied: boolean
  fallbackIncidentsReviewed: boolean
  rollbackSwitchStillArmed: boolean
  mihomoFallbackStillRetained: boolean
  holdWindowEvidenceArchived: boolean
  cutoverHoldWindowComplete: boolean
  holdSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningSupportedDefaultCutoverVerificationComplete: boolean
  supportedDefaultCutoverHoldWindow: RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport
  finalSupportedDefaultCutoverHoldWindowDecision: boolean
  rustDataPlaneHardeningSupportedDefaultCutoverHoldWindowComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverCloseoutReport {
  runtimeId: string
  component: string
  holdWindowReviewed: boolean
  supportedDefaultStateDocumented: boolean
  rollbackOwnerAcknowledged: boolean
  fallbackRetirementBoundaryRetained: boolean
  releaseNotesUpdated: boolean
  closeoutEvidenceArchived: boolean
  supportedDefaultCutoverCloseoutComplete: boolean
  closeoutSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverCloseoutReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningSupportedDefaultCutoverHoldWindowComplete: boolean
  supportedDefaultCutoverCloseout: RuntimeRustKernelRuntimeDataPlaneHardeningSupportedDefaultCutoverCloseoutReport
  finalSupportedDefaultCutoverCloseoutDecision: boolean
  rustDataPlaneHardeningSupportedDefaultCutoverCloseoutComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutGuardReport {
  runtimeId: string
  component: string
  cutoverCloseoutReviewed: boolean
  expandedScopeLocked: boolean
  rolloutCapDefined: boolean
  fallbackMatrixRetained: boolean
  rollbackSwitchVerified: boolean
  telemetrySoakPlanDefined: boolean
  unsupportedPathBoundaryRetained: boolean
  operatorRolloutAcknowledged: boolean
  expandedDefaultRolloutGuardComplete: boolean
  guardSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutGuardReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningSupportedDefaultCutoverCloseoutComplete: boolean
  expandedDefaultRolloutGuard: RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutGuardReport
  finalExpandedDefaultRolloutGuardDecision: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutGuardComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutDryRunReport {
  runtimeId: string
  component: string
  guardReviewed: boolean
  expandedManifestReplayed: boolean
  representativeProfileSimulationCompleted: boolean
  fallbackRoutingRehearsed: boolean
  rollbackRehearsed: boolean
  telemetrySoakSampleReviewed: boolean
  dryRunEvidenceArchived: boolean
  expandedDefaultRolloutDryRunComplete: boolean
  dryRunSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutDryRunReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutGuardComplete: boolean
  expandedDefaultRolloutDryRun: RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutDryRunReport
  finalExpandedDefaultRolloutDryRunDecision: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutDryRunComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutExecutionReport {
  runtimeId: string
  component: string
  dryRunReviewed: boolean
  executionManifestLocked: boolean
  rolloutWindowStarted: boolean
  expandedProfileCapEnforced: boolean
  activeTelemetryWatch: boolean
  rollbackSwitchArmed: boolean
  mihomoFallbackRetained: boolean
  operatorExecutionAcknowledged: boolean
  expandedDefaultRolloutExecutionComplete: boolean
  executionSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutExecutionReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutDryRunComplete: boolean
  expandedDefaultRolloutExecution: RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutExecutionReport
  finalExpandedDefaultRolloutExecutionDecision: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutExecutionComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutVerificationReport {
  runtimeId: string
  component: string
  executionRecordReviewed: boolean
  expandedProfileTrafficSampleReviewed: boolean
  fallbackPathSampleVerified: boolean
  rollbackSwitchVerified: boolean
  telemetryHealthBudgetVerified: boolean
  leakRegressionAbsenceVerified: boolean
  verificationEvidenceArchived: boolean
  expandedDefaultRolloutVerificationComplete: boolean
  verificationSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutVerificationReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutExecutionComplete: boolean
  expandedDefaultRolloutVerification: RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutVerificationReport
  finalExpandedDefaultRolloutVerificationDecision: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutVerificationComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutCloseoutReport {
  runtimeId: string
  component: string
  verificationReviewed: boolean
  expandedRolloutStateDocumented: boolean
  rollbackOwnerAcknowledged: boolean
  fallbackMatrixRetained: boolean
  unsupportedPathBoundaryRetained: boolean
  releaseNotesUpdated: boolean
  closeoutEvidenceArchived: boolean
  expandedDefaultRolloutCloseoutComplete: boolean
  closeoutSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutCloseoutReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutVerificationComplete: boolean
  expandedDefaultRolloutCloseout: RuntimeRustKernelRuntimeDataPlaneHardeningExpandedDefaultRolloutCloseoutReport
  finalExpandedDefaultRolloutCloseoutDecision: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutCloseoutComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementGuardReport {
  runtimeId: string
  component: string
  expandedRolloutCloseoutReviewed: boolean
  protocolParityScopeLocked: boolean
  tunParityScopeLocked: boolean
  adapterParityScopeLocked: boolean
  dnsParityScopeLocked: boolean
  emergencyRollbackRetained: boolean
  crossPlatformDrillPlanDefined: boolean
  operatorRetirementAcknowledged: boolean
  mihomoFallbackRetirementGuardComplete: boolean
  guardSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementGuardReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningExpandedDefaultRolloutCloseoutComplete: boolean
  mihomoFallbackRetirementGuard: RuntimeRustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementGuardReport
  finalMihomoFallbackRetirementGuardDecision: boolean
  rustDataPlaneHardeningMihomoFallbackRetirementGuardComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementDryRunReport {
  runtimeId: string
  component: string
  guardReviewed: boolean
  parityManifestReplayed: boolean
  crossPlatformRollbackRehearsed: boolean
  fallbackDependencyInventoryReplayed: boolean
  emergencyRecoveryRehearsed: boolean
  productionForwardingUnchangedVerified: boolean
  dryRunEvidenceArchived: boolean
  mihomoFallbackRetirementDryRunComplete: boolean
  dryRunSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementDryRunReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningMihomoFallbackRetirementGuardComplete: boolean
  mihomoFallbackRetirementDryRun: RuntimeRustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementDryRunReport
  finalMihomoFallbackRetirementDryRunDecision: boolean
  rustDataPlaneHardeningMihomoFallbackRetirementDryRunComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RuntimeRustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementReadinessReport {
  runtimeId: string
  component: string
  dryRunReviewed: boolean
  protocolParityEvidenceArchived: boolean
  tunParityEvidenceArchived: boolean
  adapterParityEvidenceArchived: boolean
  dnsParityEvidenceArchived: boolean
  soakEvidenceArchived: boolean
  emergencyRollbackOwnerAcknowledged: boolean
  mihomoFallbackRetirementReadinessComplete: boolean
  readinessSurfaces: string[]
  blockers: string[]
  facts: string[]
}

export interface RuntimeKernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementReadinessReport {
  runtimeId: string
  component: string
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  productionDataPlaneMutationAllowed: boolean
  rustDataPlaneHardeningMihomoFallbackRetirementDryRunComplete: boolean
  mihomoFallbackRetirementReadiness: RuntimeRustKernelRuntimeDataPlaneHardeningMihomoFallbackRetirementReadinessReport
  finalMihomoFallbackRetirementReadinessDecision: boolean
  rustDataPlaneHardeningMihomoFallbackRetirementReadinessComplete: boolean
  selectedRuntimeKind: RuntimeKernelRuntimeKind
  rollbackRuntimeKind: RuntimeKernelRuntimeKind
  checks: RuntimeKernelLoopbackR4ExpandedOptInLimitedRolloutGateCheck[]
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

export type RustProtocolForwardingSubsetStatus =
  | 'ready'
  | 'running'
  | 'stopped'
  | 'blocked'

export interface RustProtocolForwardingSubsetAccounting {
  acceptedConnections: number
  completedConnections: number
  failedConnections: number
  bytesFromClient: number
  bytesFromTarget: number
  lastError?: string | null
}

export interface RustProtocolForwardingSubsetPreflightReport {
  runtimeId: string
  component: string
  kernelArea: string
  status: RustProtocolForwardingSubsetStatus
  reason: string
  listenerHost: string
  listenerPort: number
  targetHost: string
  targetPort: number
  canStartAfterOptIn: boolean
  explicitOptInRequired: boolean
  loopbackOnly: boolean
  supportedProtocols: string[]
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RustProtocolForwardingSubsetStatusReport {
  runtimeId: string
  component: string
  kernelArea: string
  status: RustProtocolForwardingSubsetStatus
  reason: string
  running: boolean
  listenerHost: string
  listenerPort?: number | null
  targetHost?: string | null
  targetPort?: number | null
  startedAtEpochMs?: number | null
  accounting: RustProtocolForwardingSubsetAccounting
  loopbackOnly: boolean
  supportedProtocols: string[]
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RustProtocolForwardingSubsetStartReport {
  preflight: RustProtocolForwardingSubsetPreflightReport
  status: RustProtocolForwardingSubsetStatusReport
  explicitOptIn: boolean
  started: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface RustProtocolForwardingSubsetStopReport {
  status: RustProtocolForwardingSubsetStatus
  reason: string
  stopped: boolean
  previousStatus: RustProtocolForwardingSubsetStatusReport
  afterStatus: RustProtocolForwardingSubsetStatusReport
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface RustProtocolForwardingSubsetSmokeEvidenceReport {
  runtimeId: string
  component: string
  kernelArea: string
  status: RustProtocolForwardingSubsetStatus
  listenerPort: number
  targetPort: number
  targetReceived: boolean
  responseStatus?: string | null
  accounting: RustProtocolForwardingSubsetAccounting
  stopReport?: RustProtocolForwardingSubsetStopReport | null
  passed: boolean
  mutatesRuntime: boolean
  liveExecutionAllowed: boolean
  defaultRoute: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export type RustTunSystemProxyMode = 'off' | 'systemProxy' | 'tun'

export type RustTunSystemProxyParityStatus =
  | 'ready'
  | 'applied'
  | 'restored'
  | 'blocked'

export interface RustTunSystemProxyRouteSnapshot {
  enableSystemProxy: boolean
  enableTunMode: boolean
  proxyAutoConfig: boolean
  proxyHost?: string | null
  mixedPort: number
  systemProxyBypass?: string | null
  useDefaultBypass: boolean
  osSystemProxyEnabled?: boolean | null
  osSystemProxyServer?: string | null
  clashTunEnabled?: boolean | null
}

export interface RustTunSystemProxyRoutePatch {
  enableSystemProxy: boolean
  enableTunMode: boolean
}

export interface RustTunSystemProxyParityPreflightReport {
  runtimeId: string
  component: string
  kernelArea: string
  status: RustTunSystemProxyParityStatus
  reason: string
  requestedMode: RustTunSystemProxyMode
  currentSnapshot: RustTunSystemProxyRouteSnapshot
  routePatch: RustTunSystemProxyRoutePatch
  explicitOptInRequired: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  systemProxyOsApply: boolean
  tunRuntimeApply: boolean
  mihomoFallback: boolean
  rollbackSupported: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RustTunSystemProxyParityApplyReport {
  status: RustTunSystemProxyParityStatus
  reason: string
  requestedMode: RustTunSystemProxyMode
  preflight: RustTunSystemProxyParityPreflightReport
  previousSnapshot: RustTunSystemProxyRouteSnapshot
  appliedSnapshot: RustTunSystemProxyRouteSnapshot
  rollbackRecordPath?: string | null
  explicitOptIn: boolean
  mutatesRuntime: boolean
  reloadMihomo: boolean
  systemProxyOsApply: boolean
  tunRuntimeApply: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export interface RustTunSystemProxyParityRollbackReport {
  status: RustTunSystemProxyParityStatus
  reason: string
  restoredSnapshot: RustTunSystemProxyRouteSnapshot
  rollbackRecordPath?: string | null
  mutatesRuntime: boolean
  reloadMihomo: boolean
  systemProxyOsApply: boolean
  tunRuntimeApply: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export type RustFallbackRetirementReadinessStatus =
  | 'ready'
  | 'locked'
  | 'blocked'

export interface RustFallbackRetirementScopeArea {
  area: string
  rustOwnedCapability: string
  mihomoFallbackScope: string
  rollbackRecordPath?: string | null
  rollbackRecordPresent: boolean
  canaryEvidenceRequired: boolean
  fallbackRetirementAllowed: boolean
  blockers: string[]
  warnings: string[]
}

export interface RustFallbackRetirementReadinessManifest {
  runtimeId: string
  component: string
  kernelArea: string
  status: RustFallbackRetirementReadinessStatus
  generatedAtEpochSeconds: number
  supportedScope: RustFallbackRetirementScopeArea[]
  unsupportedFallbackScope: string[]
  emergencyRollbackPaths: string[]
  manifestPath?: string | null
  fallbackRetirementExecutionAllowed: boolean
  mutatesRuntime: boolean
  removesMihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export interface RustFallbackRetirementReadinessLockReport {
  status: RustFallbackRetirementReadinessStatus
  reason: string
  manifest: RustFallbackRetirementReadinessManifest
  explicitOptIn: boolean
  manifestPath?: string | null
  mutatesRuntime: boolean
  removesMihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
}

export type RustRuntimeRealCanaryStatus = 'passed' | 'failed' | 'blocked'

export interface RustRuntimeRealCanaryEvidenceReport {
  runtimeId: string
  component: string
  kernelArea: string
  status: RustRuntimeRealCanaryStatus
  reason: string
  canaryProfile: string
  startedAtEpochSeconds: number
  explicitOptIn: boolean
  dnsSmokeEvidence?: RuntimeKernelLoopbackDnsSmokeEvidenceReport | null
  protocolForwardingEvidence?: RustProtocolForwardingSubsetSmokeEvidenceReport | null
  tunSystemProxyPreflight?: RustTunSystemProxyParityPreflightReport | null
  fallbackReadinessManifest?: RustFallbackRetirementReadinessManifest | null
  evidencePath?: string | null
  mutatesRuntime: boolean
  writesEvidenceArtifact: boolean
  removesMihomoFallback: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export type MihomoFallbackRetirementExecutionStatus =
  | 'planned'
  | 'executed'
  | 'restored'
  | 'blocked'

export interface MihomoFallbackRetirementExecutionScope {
  scope: string
  rustOwnedPath: string
  fallbackRetiredForScope: boolean
  mihomoFallbackRetainedFor: string[]
  evidence: string[]
}

export interface MihomoFallbackRetirementEmergencyCheckpoint {
  checkpointPath?: string | null
  canaryEvidencePath?: string | null
  previousExecutionManifestPath?: string | null
  retainedFallbackScope: string[]
  createdAtEpochSeconds: number
}

export interface MihomoFallbackRetirementExecutionReport {
  runtimeId: string
  component: string
  kernelArea: string
  status: MihomoFallbackRetirementExecutionStatus
  reason: string
  explicitOptIn: boolean
  supportedScope: MihomoFallbackRetirementExecutionScope[]
  emergencyCheckpoint: MihomoFallbackRetirementEmergencyCheckpoint
  executionManifestPath?: string | null
  mutatesRuntime: boolean
  writesExecutionManifest: boolean
  retiresSupportedFallback: boolean
  removesMihomoFallbackBinary: boolean
  unsupportedMihomoFallbackRetained: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export type RustProtocolAdapterForwardingStatus =
  | 'passed'
  | 'failed'
  | 'blocked'

export type RustProtocolAdapterForwardingAdapterKind =
  | 'direct'
  | 'reject'
  | 'mihomoFallback'

export interface RustProtocolAdapterForwardingDecisionEvidence {
  adapterKind: RustProtocolAdapterForwardingAdapterKind
  listenerPort: number
  targetPort?: number | null
  targetReceived: boolean
  responseStatus?: string | null
  acceptedConnections: number
  bytesFromClient: number
  bytesFromTarget: number
  passed: boolean
  blockers: string[]
}

export interface RustProtocolAdapterForwardingExpansionReport {
  runtimeId: string
  component: string
  kernelArea: string
  status: RustProtocolAdapterForwardingStatus
  reason: string
  explicitOptIn: boolean
  directEvidence?: RustProtocolAdapterForwardingDecisionEvidence | null
  rejectEvidence?: RustProtocolAdapterForwardingDecisionEvidence | null
  evidencePath?: string | null
  loopbackOnly: boolean
  mutatesRuntime: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  writesEvidenceArtifact: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export type RustRemoteAdapterTransportStatus =
  | 'passed'
  | 'failed'
  | 'blocked'

export type RustRemoteAdapterTransportKind =
  | 'tcpConnect'
  | 'unsupportedProxyProtocol'

export interface RustRemoteAdapterTransportEvidence {
  transportKind: RustRemoteAdapterTransportKind
  adapterName: string
  controlPort?: number | null
  targetPort?: number | null
  targetReceived: boolean
  responseStatus?: string | null
  bytesToRemote: number
  bytesFromRemote: number
  fallbackRetained: boolean
  passed: boolean
  blockers: string[]
}

export interface RustRemoteAdapterTransportExpansionReport {
  runtimeId: string
  component: string
  kernelArea: string
  status: RustRemoteAdapterTransportStatus
  reason: string
  explicitOptIn: boolean
  tcpConnectEvidence?: RustRemoteAdapterTransportEvidence | null
  unsupportedProtocolEvidence?: RustRemoteAdapterTransportEvidence | null
  evidencePath?: string | null
  loopbackRemoteOnly: boolean
  mutatesRuntime: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  writesEvidenceArtifact: boolean
  mihomoFallback: boolean
  blockers: string[]
  warnings: string[]
  facts: string[]
  nextSafeBatch: string
}

export type RustHttpConnectProxyAdapterStatus =
  | 'passed'
  | 'failed'
  | 'blocked'

export interface RustHttpConnectProxyAdapterEvidence {
  adapterName: string
  listenerPort: number
  targetPort: number
  connectAuthority: string
  connectEstablished: boolean
  targetReceived: boolean
  responseStatus?: string | null
  bytesFromClient: number
  bytesFromTarget: number
  passed: boolean
  blockers: string[]
}

export interface RustHttpConnectProxyAdapterReport {
  runtimeId: string
  component: string
  kernelArea: string
  status: RustHttpConnectProxyAdapterStatus
  reason: string
  explicitOptIn: boolean
  connectEvidence?: RustHttpConnectProxyAdapterEvidence | null
  unsupportedProtocols: string[]
  evidencePath?: string | null
  loopbackRemoteOnly: boolean
  mutatesRuntime: boolean
  forwardsTraffic: boolean
  outboundAdaptersUsed: boolean
  writesEvidenceArtifact: boolean
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

export async function getRuntimeKernelRustProtocolForwardingSubsetPreflight(
  listenerPort?: number,
  targetHost?: string,
  targetPort?: number,
) {
  return invoke<RustProtocolForwardingSubsetPreflightReport>(
    'get_runtime_kernel_rust_protocol_forwarding_subset_preflight',
    { listenerPort, targetHost, targetPort },
  )
}

export async function startRuntimeKernelRustProtocolForwardingSubset(
  listenerPort?: number,
  targetHost?: string,
  targetPort?: number,
  explicitOptIn = false,
) {
  return invoke<RustProtocolForwardingSubsetStartReport>(
    'start_runtime_kernel_rust_protocol_forwarding_subset',
    { listenerPort, targetHost, targetPort, explicitOptIn },
  )
}

export async function getRuntimeKernelRustProtocolForwardingSubsetStatus() {
  return invoke<RustProtocolForwardingSubsetStatusReport>(
    'get_runtime_kernel_rust_protocol_forwarding_subset_status',
  )
}

export async function stopRuntimeKernelRustProtocolForwardingSubset() {
  return invoke<RustProtocolForwardingSubsetStopReport>(
    'stop_runtime_kernel_rust_protocol_forwarding_subset',
  )
}

export async function getRuntimeKernelRustProtocolForwardingSubsetSmokeEvidence(
  listenerPort?: number,
  targetPort?: number,
) {
  return invoke<RustProtocolForwardingSubsetSmokeEvidenceReport>(
    'get_runtime_kernel_rust_protocol_forwarding_subset_smoke_evidence',
    { listenerPort, targetPort },
  )
}

export async function getRuntimeKernelRustTunSystemProxyParityPreflight(
  requestedMode?: RustTunSystemProxyMode,
) {
  return invoke<RustTunSystemProxyParityPreflightReport>(
    'get_runtime_kernel_rust_tun_system_proxy_parity_preflight',
    { requestedMode },
  )
}

export async function applyRuntimeKernelRustTunSystemProxyParity(
  requestedMode?: RustTunSystemProxyMode,
  explicitOptIn = false,
) {
  return invoke<RustTunSystemProxyParityApplyReport>(
    'apply_runtime_kernel_rust_tun_system_proxy_parity',
    { requestedMode, explicitOptIn },
  )
}

export async function rollbackRuntimeKernelRustTunSystemProxyParity() {
  return invoke<RustTunSystemProxyParityRollbackReport>(
    'rollback_runtime_kernel_rust_tun_system_proxy_parity',
  )
}

export async function getRuntimeKernelRustFallbackRetirementReadinessManifest() {
  return invoke<RustFallbackRetirementReadinessManifest>(
    'get_runtime_kernel_rust_fallback_retirement_readiness_manifest',
  )
}

export async function lockRuntimeKernelRustFallbackRetirementReadiness(
  explicitOptIn = false,
) {
  return invoke<RustFallbackRetirementReadinessLockReport>(
    'lock_runtime_kernel_rust_fallback_retirement_readiness',
    { explicitOptIn },
  )
}

export async function runRuntimeKernelRustRuntimeRealCanary(
  canaryProfile?: string,
  explicitOptIn = false,
) {
  return invoke<RustRuntimeRealCanaryEvidenceReport>(
    'run_runtime_kernel_rust_runtime_real_canary',
    { canaryProfile, explicitOptIn },
  )
}

export async function runRuntimeKernelRustProtocolAdapterForwardingExpansion(
  explicitOptIn = false,
) {
  return invoke<RustProtocolAdapterForwardingExpansionReport>(
    'run_runtime_kernel_rust_protocol_adapter_forwarding_expansion',
    { explicitOptIn },
  )
}

export async function runRuntimeKernelRustRemoteAdapterTransportExpansion(
  explicitOptIn = false,
) {
  return invoke<RustRemoteAdapterTransportExpansionReport>(
    'run_runtime_kernel_rust_remote_adapter_transport_expansion',
    { explicitOptIn },
  )
}

export async function runRuntimeKernelRustHttpConnectProxyAdapter(
  explicitOptIn = false,
) {
  return invoke<RustHttpConnectProxyAdapterReport>(
    'run_runtime_kernel_rust_http_connect_proxy_adapter',
    { explicitOptIn },
  )
}

export async function getRuntimeKernelMihomoFallbackRetirementExecutionPlan() {
  return invoke<MihomoFallbackRetirementExecutionReport>(
    'get_runtime_kernel_mihomo_fallback_retirement_execution_plan',
  )
}

export async function executeRuntimeKernelMihomoFallbackRetirement(
  explicitOptIn = false,
  runCanary = false,
) {
  return invoke<MihomoFallbackRetirementExecutionReport>(
    'execute_runtime_kernel_mihomo_fallback_retirement',
    { explicitOptIn, runCanary },
  )
}

export async function rollbackRuntimeKernelMihomoFallbackRetirementExecution() {
  return invoke<MihomoFallbackRetirementExecutionReport>(
    'rollback_runtime_kernel_mihomo_fallback_retirement_execution',
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

export async function getRuntimeKernelRustRuntimeCandidate() {
  return invoke<RuntimeRustKernelRuntimeCandidateReport>(
    'get_runtime_kernel_rust_runtime_candidate',
  )
}

export async function getRuntimeKernelRuntimeSelectionScaffold(
  requestedRuntimeKind?: RuntimeKernelRuntimeKind,
  rustRuntimeOptInDecision?: boolean,
) {
  return invoke<RuntimeKernelRuntimeSelectionScaffoldReport>(
    'get_runtime_kernel_runtime_selection_scaffold',
    {
      requestedRuntimeKind,
      rustRuntimeOptInDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5DefaultCutoverCloseoutReport(
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
  r5CloseoutReportDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5DefaultCutoverCloseoutReport>(
    'get_runtime_kernel_loopback_r5_default_cutover_closeout_report',
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
      r5CloseoutReportDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR5CloseoutR6RustRuntimeScaffold(
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
  r5CloseoutReportDecision?: boolean,
  requestedRuntimeKind?: RuntimeKernelRuntimeKind,
  rustRuntimeOptInDecision?: boolean,
  rustRuntimeScaffoldDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR5CloseoutR6RustRuntimeScaffoldReport>(
    'get_runtime_kernel_loopback_r5_closeout_r6_rust_runtime_scaffold',
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
      r5CloseoutReportDecision,
      requestedRuntimeKind,
      rustRuntimeOptInDecision,
      rustRuntimeScaffoldDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR6OptInRustRuntimeMvp(
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
  r5CloseoutReportDecision?: boolean,
  requestedRuntimeKind?: RuntimeKernelRuntimeKind,
  rustRuntimeOptInDecision?: boolean,
  rustRuntimeScaffoldDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR6OptInRustRuntimeMvpReport>(
    'get_runtime_kernel_loopback_r6_opt_in_rust_runtime_mvp',
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
      r5CloseoutReportDecision,
      requestedRuntimeKind,
      rustRuntimeOptInDecision,
      rustRuntimeScaffoldDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackR6RustDefaultCanary(
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
  r5CloseoutReportDecision?: boolean,
  requestedRuntimeKind?: RuntimeKernelRuntimeKind,
  rustRuntimeOptInDecision?: boolean,
  rustRuntimeScaffoldDecision?: boolean,
  canaryDefaultDecision?: boolean,
  healthCheckPassed?: boolean,
  rollbackTriggered?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR6RustDefaultCanaryReport>(
    'get_runtime_kernel_loopback_r6_rust_default_canary',
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
      r5CloseoutReportDecision,
      requestedRuntimeKind,
      rustRuntimeOptInDecision,
      rustRuntimeScaffoldDecision,
      canaryDefaultDecision,
      healthCheckPassed,
      rollbackTriggered,
    },
  )
}

export async function getRuntimeKernelLoopbackR7RustDefaultCutover(
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
  r5CloseoutReportDecision?: boolean,
  requestedRuntimeKind?: RuntimeKernelRuntimeKind,
  rustRuntimeOptInDecision?: boolean,
  rustRuntimeScaffoldDecision?: boolean,
  canaryDefaultDecision?: boolean,
  healthCheckPassed?: boolean,
  rollbackTriggered?: boolean,
  r7CutoverDecision?: boolean,
  rollbackHoldDecision?: boolean,
  rollbackSwitchRequested?: boolean,
  profileScope?: string,
) {
  return invoke<RuntimeKernelLoopbackR7RustDefaultCutoverReport>(
    'get_runtime_kernel_loopback_r7_rust_default_cutover',
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
      r5CloseoutReportDecision,
      requestedRuntimeKind,
      rustRuntimeOptInDecision,
      rustRuntimeScaffoldDecision,
      canaryDefaultDecision,
      healthCheckPassed,
      rollbackTriggered,
      r7CutoverDecision,
      rollbackHoldDecision,
      rollbackSwitchRequested,
      profileScope,
    },
  )
}

export async function getRuntimeKernelLoopbackR7MihomoFallbackRetirement(
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
  r5CloseoutReportDecision?: boolean,
  requestedRuntimeKind?: RuntimeKernelRuntimeKind,
  rustRuntimeOptInDecision?: boolean,
  rustRuntimeScaffoldDecision?: boolean,
  canaryDefaultDecision?: boolean,
  healthCheckPassed?: boolean,
  rollbackTriggered?: boolean,
  r7CutoverDecision?: boolean,
  rollbackHoldDecision?: boolean,
  rollbackSwitchRequested?: boolean,
  profileScope?: string,
  protocolParityDecision?: boolean,
  tunParityDecision?: boolean,
  adapterParityDecision?: boolean,
  dnsRuntimeParityDecision?: boolean,
  crossPlatformRollbackDecision?: boolean,
  soakEvidenceDecision?: boolean,
  fallbackRetirementDecision?: boolean,
  emergencyRollbackDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackR7MihomoFallbackRetirementReport>(
    'get_runtime_kernel_loopback_r7_mihomo_fallback_retirement',
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
      r5CloseoutReportDecision,
      requestedRuntimeKind,
      rustRuntimeOptInDecision,
      rustRuntimeScaffoldDecision,
      canaryDefaultDecision,
      healthCheckPassed,
      rollbackTriggered,
      r7CutoverDecision,
      rollbackHoldDecision,
      rollbackSwitchRequested,
      profileScope,
      protocolParityDecision,
      tunParityDecision,
      adapterParityDecision,
      dnsRuntimeParityDecision,
      crossPlatformRollbackDecision,
      soakEvidenceDecision,
      fallbackRetirementDecision,
      emergencyRollbackDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackFullRustRuntimeHardening(
  r7FallbackRetirementPassed?: boolean,
  observedSoakHours?: number,
  healthRegressionCount?: number,
  rollbackTriggerCount?: number,
  rollbackEventCount?: number,
  lastRollbackEventTs?: number,
  rollbackTelemetryDecision?: boolean,
  emergencyRollbackDecision?: boolean,
  windowsServiceHardeningDecision?: boolean,
  macosServiceHardeningDecision?: boolean,
  linuxServiceHardeningDecision?: boolean,
  finalHardeningDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackFullRustRuntimeHardeningReport>(
    'get_runtime_kernel_loopback_full_rust_runtime_hardening',
    {
      r7FallbackRetirementPassed,
      observedSoakHours,
      healthRegressionCount,
      rollbackTriggerCount,
      rollbackEventCount,
      lastRollbackEventTs,
      rollbackTelemetryDecision,
      emergencyRollbackDecision,
      windowsServiceHardeningDecision,
      macosServiceHardeningDecision,
      linuxServiceHardeningDecision,
      finalHardeningDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementAudit(
  fullRustRuntimeHardenedDecision?: boolean,
  sidecarSourceAuditDecision?: boolean,
  bundledMihomoAuditDecision?: boolean,
  ipcFallbackAuditDecision?: boolean,
  docsAuditDecision?: boolean,
  emergencyRollbackRetained?: boolean,
  finalRetirementAuditDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementAuditReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_audit',
    {
      fullRustRuntimeHardenedDecision,
      sidecarSourceAuditDecision,
      bundledMihomoAuditDecision,
      ipcFallbackAuditDecision,
      docsAuditDecision,
      emergencyRollbackRetained,
      finalRetirementAuditDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementPlan(
  goMihomoRetirementAuditCompleteDecision?: boolean,
  sidecarSourceRemovalPlanDecision?: boolean,
  bundledArtifactDeprecationPlanDecision?: boolean,
  ipcFallbackReplacementPlanDecision?: boolean,
  emergencyRollbackPreservationPlanDecision?: boolean,
  releaseRolloutPlanDecision?: boolean,
  finalRetirementPlanDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementPlanReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_plan',
    {
      goMihomoRetirementAuditCompleteDecision,
      sidecarSourceRemovalPlanDecision,
      bundledArtifactDeprecationPlanDecision,
      ipcFallbackReplacementPlanDecision,
      emergencyRollbackPreservationPlanDecision,
      releaseRolloutPlanDecision,
      finalRetirementPlanDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementExecutionGuard(
  goMihomoRetirementPlanCompleteDecision?: boolean,
  removalManifestDecision?: boolean,
  abortPlanDecision?: boolean,
  stagedRolloutGuardDecision?: boolean,
  emergencyRollbackDrillDecision?: boolean,
  operatorAcknowledgementDecision?: boolean,
  finalExecutionGuardDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementExecutionGuardReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_execution_guard',
    {
      goMihomoRetirementPlanCompleteDecision,
      removalManifestDecision,
      abortPlanDecision,
      stagedRolloutGuardDecision,
      emergencyRollbackDrillDecision,
      operatorAcknowledgementDecision,
      finalExecutionGuardDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementDryRun(
  goMihomoRetirementExecutionGuardCompleteDecision?: boolean,
  dryRunManifestReplayDecision?: boolean,
  noSourceMutationsDecision?: boolean,
  noBundledArtifactMutationsDecision?: boolean,
  rollbackRehearsalDecision?: boolean,
  dryRunReportArchivedDecision?: boolean,
  finalDryRunDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementDryRunReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_dry_run',
    {
      goMihomoRetirementExecutionGuardCompleteDecision,
      dryRunManifestReplayDecision,
      noSourceMutationsDecision,
      noBundledArtifactMutationsDecision,
      rollbackRehearsalDecision,
      dryRunReportArchivedDecision,
      finalDryRunDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementCloseout(
  goMihomoRetirementDryRunCompleteDecision?: boolean,
  dryRunEvidenceReviewDecision?: boolean,
  closeoutReportArchivedDecision?: boolean,
  rollbackCheckpointVerifiedDecision?: boolean,
  artifactInventoryFrozenDecision?: boolean,
  noRemovalMutationsDecision?: boolean,
  finalCloseoutDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementCloseoutReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_closeout',
    {
      goMihomoRetirementDryRunCompleteDecision,
      dryRunEvidenceReviewDecision,
      closeoutReportArchivedDecision,
      rollbackCheckpointVerifiedDecision,
      artifactInventoryFrozenDecision,
      noRemovalMutationsDecision,
      finalCloseoutDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementFinalRemovalGate(
  goMihomoRetirementCloseoutCompleteDecision?: boolean,
  closeoutEvidenceAcceptanceDecision?: boolean,
  rollbackBoundaryLockDecision?: boolean,
  removalScopeLockDecision?: boolean,
  releaseBlockerReviewDecision?: boolean,
  finalOperatorApprovalDecision?: boolean,
  finalRemovalDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementFinalRemovalGateReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_final_removal_gate',
    {
      goMihomoRetirementCloseoutCompleteDecision,
      closeoutEvidenceAcceptanceDecision,
      rollbackBoundaryLockDecision,
      removalScopeLockDecision,
      releaseBlockerReviewDecision,
      finalOperatorApprovalDecision,
      finalRemovalDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementExecution(
  goMihomoRetirementFinalRemovalGateCompleteDecision?: boolean,
  rollbackCheckpointCreatedDecision?: boolean,
  executionManifestApplicationDecision?: boolean,
  sourceRemovalRecordDecision?: boolean,
  artifactRemovalRecordDecision?: boolean,
  postExecutionValidationDecision?: boolean,
  finalExecutionDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementExecutionReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_execution',
    {
      goMihomoRetirementFinalRemovalGateCompleteDecision,
      rollbackCheckpointCreatedDecision,
      executionManifestApplicationDecision,
      sourceRemovalRecordDecision,
      artifactRemovalRecordDecision,
      postExecutionValidationDecision,
      finalExecutionDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementPostExecutionVerification(
  goMihomoRetirementExecutionCompleteDecision?: boolean,
  rustOnlyBoundaryVerificationDecision?: boolean,
  rollbackCheckpointRetentionDecision?: boolean,
  sourceRemovalVerificationDecision?: boolean,
  artifactRemovalVerificationDecision?: boolean,
  fallbackIpcAbsenceVerificationDecision?: boolean,
  finalVerificationDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementPostExecutionVerificationReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_post_execution_verification',
    {
      goMihomoRetirementExecutionCompleteDecision,
      rustOnlyBoundaryVerificationDecision,
      rollbackCheckpointRetentionDecision,
      sourceRemovalVerificationDecision,
      artifactRemovalVerificationDecision,
      fallbackIpcAbsenceVerificationDecision,
      finalVerificationDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementRollbackSurfaceRetirement(
  goMihomoRetirementPostExecutionVerificationCompleteDecision?: boolean,
  postExecutionVerificationReviewDecision?: boolean,
  replacementRecoveryPathVerificationDecision?: boolean,
  rollbackSurfaceInventoryLockDecision?: boolean,
  rollbackSurfaceRetirementPlanArchiveDecision?: boolean,
  emergencyRecoveryDrillDecision?: boolean,
  finalRollbackSurfaceRetirementDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementRollbackSurfaceRetirementReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_rollback_surface_retirement',
    {
      goMihomoRetirementPostExecutionVerificationCompleteDecision,
      postExecutionVerificationReviewDecision,
      replacementRecoveryPathVerificationDecision,
      rollbackSurfaceInventoryLockDecision,
      rollbackSurfaceRetirementPlanArchiveDecision,
      emergencyRecoveryDrillDecision,
      finalRollbackSurfaceRetirementDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackGoMihomoRetirementCompletionCloseout(
  goMihomoRetirementRollbackSurfaceRetirementCompleteDecision?: boolean,
  rollbackSurfaceRetirementReviewDecision?: boolean,
  recoveryBoundaryEvidenceRetentionDecision?: boolean,
  completionReportArchiveDecision?: boolean,
  releaseNotesUpdateDecision?: boolean,
  migrationStateFreezeDecision?: boolean,
  finalCompletionDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackGoMihomoRetirementCompletionCloseoutReport>(
    'get_runtime_kernel_loopback_go_mihomo_retirement_completion_closeout',
    {
      goMihomoRetirementRollbackSurfaceRetirementCompleteDecision,
      rollbackSurfaceRetirementReviewDecision,
      recoveryBoundaryEvidenceRetentionDecision,
      completionReportArchiveDecision,
      releaseNotesUpdateDecision,
      migrationStateFreezeDecision,
      finalCompletionDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningPreflight(
  goMihomoRetirementCompleteDecision?: boolean,
  protocolParityInventoryDecision?: boolean,
  tunBoundaryInventoryDecision?: boolean,
  adapterCompatibilityMatrixDecision?: boolean,
  dnsLeakVerificationPlanDecision?: boolean,
  rollbackDrillPlanDecision?: boolean,
  optInExecutionBoundaryDecision?: boolean,
  finalPreflightDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningPreflightReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_preflight',
    {
      goMihomoRetirementCompleteDecision,
      protocolParityInventoryDecision,
      tunBoundaryInventoryDecision,
      adapterCompatibilityMatrixDecision,
      dnsLeakVerificationPlanDecision,
      rollbackDrillPlanDecision,
      optInExecutionBoundaryDecision,
      finalPreflightDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningBoundaryAudit(
  rustDataPlaneHardeningPreflightCompleteDecision?: boolean,
  preflightReviewDecision?: boolean,
  protocolBoundaryAuditDecision?: boolean,
  tunBoundaryAuditDecision?: boolean,
  adapterBoundaryAuditDecision?: boolean,
  dnsLeakBoundaryAuditDecision?: boolean,
  rollbackBoundaryAuditDecision?: boolean,
  optInBoundaryAuditDecision?: boolean,
  finalBoundaryAuditDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningBoundaryAuditReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_boundary_audit',
    {
      rustDataPlaneHardeningPreflightCompleteDecision,
      preflightReviewDecision,
      protocolBoundaryAuditDecision,
      tunBoundaryAuditDecision,
      adapterBoundaryAuditDecision,
      dnsLeakBoundaryAuditDecision,
      rollbackBoundaryAuditDecision,
      optInBoundaryAuditDecision,
      finalBoundaryAuditDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningOptInExecutionGuard(
  rustDataPlaneHardeningBoundaryAuditCompleteDecision?: boolean,
  boundaryAuditReviewDecision?: boolean,
  optInScopeLockDecision?: boolean,
  rolloutGuardDefinitionDecision?: boolean,
  abortPlanApprovalDecision?: boolean,
  telemetryWatchConfigurationDecision?: boolean,
  rollbackSwitchVerificationDecision?: boolean,
  operatorAcknowledgementDecision?: boolean,
  finalExecutionGuardDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningOptInExecutionGuardReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_opt_in_execution_guard',
    {
      rustDataPlaneHardeningBoundaryAuditCompleteDecision,
      boundaryAuditReviewDecision,
      optInScopeLockDecision,
      rolloutGuardDefinitionDecision,
      abortPlanApprovalDecision,
      telemetryWatchConfigurationDecision,
      rollbackSwitchVerificationDecision,
      operatorAcknowledgementDecision,
      finalExecutionGuardDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningOptInDryRun(
  rustDataPlaneHardeningOptInExecutionGuardCompleteDecision?: boolean,
  executionGuardReviewDecision?: boolean,
  dryRunScopeLockDecision?: boolean,
  manifestReplayDecision?: boolean,
  syntheticFlowPlanDecision?: boolean,
  leakWatchPlanVerificationDecision?: boolean,
  rollbackRehearsalDecision?: boolean,
  productionForwardingUnchangedVerificationDecision?: boolean,
  dryRunEvidenceArchiveDecision?: boolean,
  finalDryRunDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningOptInDryRunReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_opt_in_dry_run',
    {
      rustDataPlaneHardeningOptInExecutionGuardCompleteDecision,
      executionGuardReviewDecision,
      dryRunScopeLockDecision,
      manifestReplayDecision,
      syntheticFlowPlanDecision,
      leakWatchPlanVerificationDecision,
      rollbackRehearsalDecision,
      productionForwardingUnchangedVerificationDecision,
      dryRunEvidenceArchiveDecision,
      finalDryRunDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningOptInExecution(
  rustDataPlaneHardeningOptInDryRunCompleteDecision?: boolean,
  dryRunReviewDecision?: boolean,
  executionManifestLockDecision?: boolean,
  stagedOptInWindowDecision?: boolean,
  telemetryWatchActivationDecision?: boolean,
  rollbackSwitchArmDecision?: boolean,
  productionMutationGuardRetentionDecision?: boolean,
  operatorExecutionAcknowledgementDecision?: boolean,
  finalExecutionDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningOptInExecutionReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_opt_in_execution',
    {
      rustDataPlaneHardeningOptInDryRunCompleteDecision,
      dryRunReviewDecision,
      executionManifestLockDecision,
      stagedOptInWindowDecision,
      telemetryWatchActivationDecision,
      rollbackSwitchArmDecision,
      productionMutationGuardRetentionDecision,
      operatorExecutionAcknowledgementDecision,
      finalExecutionDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningOptInExecutionVerification(
  rustDataPlaneHardeningOptInExecutionCompleteDecision?: boolean,
  executionRecordReviewDecision?: boolean,
  telemetrySampleReviewDecision?: boolean,
  rollbackReadinessVerificationDecision?: boolean,
  productionMutationGuardRetentionVerificationDecision?: boolean,
  productionForwardingUnchangedVerificationDecision?: boolean,
  leakRegressionAbsenceVerificationDecision?: boolean,
  verificationEvidenceArchiveDecision?: boolean,
  finalVerificationDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningOptInExecutionVerificationReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_opt_in_execution_verification',
    {
      rustDataPlaneHardeningOptInExecutionCompleteDecision,
      executionRecordReviewDecision,
      telemetrySampleReviewDecision,
      rollbackReadinessVerificationDecision,
      productionMutationGuardRetentionVerificationDecision,
      productionForwardingUnchangedVerificationDecision,
      leakRegressionAbsenceVerificationDecision,
      verificationEvidenceArchiveDecision,
      finalVerificationDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutGuard(
  rustDataPlaneHardeningOptInExecutionVerificationCompleteDecision?: boolean,
  optInVerificationReviewDecision?: boolean,
  controlledRolloutScopeLockDecision?: boolean,
  canaryPopulationCapDefinitionDecision?: boolean,
  healthRollbackTriggerDefinitionDecision?: boolean,
  telemetryHoldWindowConfigurationDecision?: boolean,
  mihomoFallbackRetentionDecision?: boolean,
  productionMutationGuardRetentionDecision?: boolean,
  operatorRolloutGuardAcknowledgementDecision?: boolean,
  finalControlledRolloutGuardDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutGuardReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_guard',
    {
      rustDataPlaneHardeningOptInExecutionVerificationCompleteDecision,
      optInVerificationReviewDecision,
      controlledRolloutScopeLockDecision,
      canaryPopulationCapDefinitionDecision,
      healthRollbackTriggerDefinitionDecision,
      telemetryHoldWindowConfigurationDecision,
      mihomoFallbackRetentionDecision,
      productionMutationGuardRetentionDecision,
      operatorRolloutGuardAcknowledgementDecision,
      finalControlledRolloutGuardDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutDryRun(
  rustDataPlaneHardeningControlledRolloutGuardCompleteDecision?: boolean,
  guardReviewDecision?: boolean,
  dryRunManifestReplayDecision?: boolean,
  cappedCanarySimulationDecision?: boolean,
  fallbackTriggerRehearsalDecision?: boolean,
  telemetryHoldSampleReviewDecision?: boolean,
  rollbackSwitchRehearsalDecision?: boolean,
  productionForwardingUnchangedVerificationDecision?: boolean,
  dryRunEvidenceArchiveDecision?: boolean,
  finalControlledRolloutDryRunDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutDryRunReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_dry_run',
    {
      rustDataPlaneHardeningControlledRolloutGuardCompleteDecision,
      guardReviewDecision,
      dryRunManifestReplayDecision,
      cappedCanarySimulationDecision,
      fallbackTriggerRehearsalDecision,
      telemetryHoldSampleReviewDecision,
      rollbackSwitchRehearsalDecision,
      productionForwardingUnchangedVerificationDecision,
      dryRunEvidenceArchiveDecision,
      finalControlledRolloutDryRunDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutReadinessCloseout(
  rustDataPlaneHardeningControlledRolloutDryRunCompleteDecision?: boolean,
  dryRunReviewDecision?: boolean,
  rolloutWindowApprovalDecision?: boolean,
  canaryPopulationCapEnforcementDecision?: boolean,
  automaticFallbackArmDecision?: boolean,
  telemetryWatchActivationDecision?: boolean,
  rollbackOwnerAcknowledgementDecision?: boolean,
  productionMutationGuardRetentionDecision?: boolean,
  closeoutEvidenceArchiveDecision?: boolean,
  finalControlledRolloutReadinessDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutReadinessCloseoutReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_readiness_closeout',
    {
      rustDataPlaneHardeningControlledRolloutDryRunCompleteDecision,
      dryRunReviewDecision,
      rolloutWindowApprovalDecision,
      canaryPopulationCapEnforcementDecision,
      automaticFallbackArmDecision,
      telemetryWatchActivationDecision,
      rollbackOwnerAcknowledgementDecision,
      productionMutationGuardRetentionDecision,
      closeoutEvidenceArchiveDecision,
      finalControlledRolloutReadinessDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryExecution(
  rustDataPlaneHardeningControlledRolloutReadinessCloseoutCompleteDecision?: boolean,
  readinessCloseoutReviewDecision?: boolean,
  executionManifestLockDecision?: boolean,
  canaryWindowStartDecision?: boolean,
  canaryPopulationCapEnforcementDecision?: boolean,
  healthTelemetryActivationDecision?: boolean,
  automaticFallbackArmDecision?: boolean,
  mihomoFallbackRetentionDecision?: boolean,
  productionMutationGuardRetentionDecision?: boolean,
  operatorCanaryExecutionAcknowledgementDecision?: boolean,
  finalControlledRolloutCanaryExecutionDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryExecutionReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_canary_execution',
    {
      rustDataPlaneHardeningControlledRolloutReadinessCloseoutCompleteDecision,
      readinessCloseoutReviewDecision,
      executionManifestLockDecision,
      canaryWindowStartDecision,
      canaryPopulationCapEnforcementDecision,
      healthTelemetryActivationDecision,
      automaticFallbackArmDecision,
      mihomoFallbackRetentionDecision,
      productionMutationGuardRetentionDecision,
      operatorCanaryExecutionAcknowledgementDecision,
      finalControlledRolloutCanaryExecutionDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryVerification(
  rustDataPlaneHardeningControlledRolloutCanaryExecutionCompleteDecision?: boolean,
  executionRecordReviewDecision?: boolean,
  healthTelemetrySampleReviewDecision?: boolean,
  automaticFallbackResultReviewDecision?: boolean,
  unsupportedTrafficFallbackVerificationDecision?: boolean,
  leakRegressionAbsenceVerificationDecision?: boolean,
  rollbackReadinessVerificationDecision?: boolean,
  productionMutationGuardRetentionVerificationDecision?: boolean,
  verificationEvidenceArchiveDecision?: boolean,
  finalControlledRolloutCanaryVerificationDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningControlledRolloutCanaryVerificationReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_controlled_rollout_canary_verification',
    {
      rustDataPlaneHardeningControlledRolloutCanaryExecutionCompleteDecision,
      executionRecordReviewDecision,
      healthTelemetrySampleReviewDecision,
      automaticFallbackResultReviewDecision,
      unsupportedTrafficFallbackVerificationDecision,
      leakRegressionAbsenceVerificationDecision,
      rollbackReadinessVerificationDecision,
      productionMutationGuardRetentionVerificationDecision,
      verificationEvidenceArchiveDecision,
      finalControlledRolloutCanaryVerificationDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionGuard(
  rustDataPlaneHardeningControlledRolloutCanaryVerificationCompleteDecision?: boolean,
  canaryVerificationReviewDecision?: boolean,
  supportedProfileScopeLockDecision?: boolean,
  fallbackMatrixRetentionDecision?: boolean,
  rollbackSwitchVerificationDecision?: boolean,
  telemetrySoakWindowDefinitionDecision?: boolean,
  releaseBlockerReviewDecision?: boolean,
  productionMutationGuardRetentionDecision?: boolean,
  operatorPromotionAcknowledgementDecision?: boolean,
  finalSupportedDefaultPromotionGuardDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionGuardReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_promotion_guard',
    {
      rustDataPlaneHardeningControlledRolloutCanaryVerificationCompleteDecision,
      canaryVerificationReviewDecision,
      supportedProfileScopeLockDecision,
      fallbackMatrixRetentionDecision,
      rollbackSwitchVerificationDecision,
      telemetrySoakWindowDefinitionDecision,
      releaseBlockerReviewDecision,
      productionMutationGuardRetentionDecision,
      operatorPromotionAcknowledgementDecision,
      finalSupportedDefaultPromotionGuardDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionDryRun(
  rustDataPlaneHardeningSupportedDefaultPromotionGuardCompleteDecision?: boolean,
  guardReviewDecision?: boolean,
  defaultSelectionManifestReplayDecision?: boolean,
  supportedProfileSimulationDecision?: boolean,
  fallbackDecisionRehearsalDecision?: boolean,
  rollbackRehearsalDecision?: boolean,
  productionForwardingUnchangedVerificationDecision?: boolean,
  dryRunEvidenceArchiveDecision?: boolean,
  finalSupportedDefaultPromotionDryRunDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultPromotionDryRunReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_promotion_dry_run',
    {
      rustDataPlaneHardeningSupportedDefaultPromotionGuardCompleteDecision,
      guardReviewDecision,
      defaultSelectionManifestReplayDecision,
      supportedProfileSimulationDecision,
      fallbackDecisionRehearsalDecision,
      rollbackRehearsalDecision,
      productionForwardingUnchangedVerificationDecision,
      dryRunEvidenceArchiveDecision,
      finalSupportedDefaultPromotionDryRunDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutover(
  rustDataPlaneHardeningSupportedDefaultPromotionDryRunCompleteDecision?: boolean,
  dryRunReviewDecision?: boolean,
  cutoverManifestLockDecision?: boolean,
  supportedProfileDefaultSelectionConfirmationDecision?: boolean,
  unsupportedPathsMihomoFallbackBindingDecision?: boolean,
  rollbackSwitchArmDecision?: boolean,
  telemetrySoakWatchActivationDecision?: boolean,
  operatorCutoverAcknowledgementDecision?: boolean,
  productionMutationGuardTransitionRecordDecision?: boolean,
  finalSupportedDefaultCutoverDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_cutover',
    {
      rustDataPlaneHardeningSupportedDefaultPromotionDryRunCompleteDecision,
      dryRunReviewDecision,
      cutoverManifestLockDecision,
      supportedProfileDefaultSelectionConfirmationDecision,
      unsupportedPathsMihomoFallbackBindingDecision,
      rollbackSwitchArmDecision,
      telemetrySoakWatchActivationDecision,
      operatorCutoverAcknowledgementDecision,
      productionMutationGuardTransitionRecordDecision,
      finalSupportedDefaultCutoverDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverVerification(
  rustDataPlaneHardeningSupportedDefaultCutoverCompleteDecision?: boolean,
  cutoverRecordReviewDecision?: boolean,
  supportedProfileTrafficSampleReviewDecision?: boolean,
  unsupportedPathFallbackVerificationDecision?: boolean,
  rollbackSwitchVerificationDecision?: boolean,
  telemetrySoakSampleReviewDecision?: boolean,
  leakRegressionAbsenceVerificationDecision?: boolean,
  mutationAuditRecordArchiveDecision?: boolean,
  finalSupportedDefaultCutoverVerificationDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverVerificationReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_cutover_verification',
    {
      rustDataPlaneHardeningSupportedDefaultCutoverCompleteDecision,
      cutoverRecordReviewDecision,
      supportedProfileTrafficSampleReviewDecision,
      unsupportedPathFallbackVerificationDecision,
      rollbackSwitchVerificationDecision,
      telemetrySoakSampleReviewDecision,
      leakRegressionAbsenceVerificationDecision,
      mutationAuditRecordArchiveDecision,
      finalSupportedDefaultCutoverVerificationDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverHoldWindow(
  rustDataPlaneHardeningSupportedDefaultCutoverVerificationCompleteDecision?: boolean,
  verificationReviewDecision?: boolean,
  soakWindowElapsedDecision?: boolean,
  healthBudgetSatisfiedDecision?: boolean,
  fallbackIncidentReviewDecision?: boolean,
  rollbackSwitchStillArmedDecision?: boolean,
  mihomoFallbackRetentionDecision?: boolean,
  holdWindowEvidenceArchiveDecision?: boolean,
  finalSupportedDefaultCutoverHoldWindowDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverHoldWindowReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_cutover_hold_window',
    {
      rustDataPlaneHardeningSupportedDefaultCutoverVerificationCompleteDecision,
      verificationReviewDecision,
      soakWindowElapsedDecision,
      healthBudgetSatisfiedDecision,
      fallbackIncidentReviewDecision,
      rollbackSwitchStillArmedDecision,
      mihomoFallbackRetentionDecision,
      holdWindowEvidenceArchiveDecision,
      finalSupportedDefaultCutoverHoldWindowDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverCloseout(
  rustDataPlaneHardeningSupportedDefaultCutoverHoldWindowCompleteDecision?: boolean,
  holdWindowReviewDecision?: boolean,
  supportedDefaultStateDocumentationDecision?: boolean,
  rollbackOwnerAcknowledgementDecision?: boolean,
  fallbackRetirementBoundaryRetentionDecision?: boolean,
  releaseNotesUpdateDecision?: boolean,
  closeoutEvidenceArchiveDecision?: boolean,
  finalSupportedDefaultCutoverCloseoutDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningSupportedDefaultCutoverCloseoutReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_supported_default_cutover_closeout',
    {
      rustDataPlaneHardeningSupportedDefaultCutoverHoldWindowCompleteDecision,
      holdWindowReviewDecision,
      supportedDefaultStateDocumentationDecision,
      rollbackOwnerAcknowledgementDecision,
      fallbackRetirementBoundaryRetentionDecision,
      releaseNotesUpdateDecision,
      closeoutEvidenceArchiveDecision,
      finalSupportedDefaultCutoverCloseoutDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutGuard(
  rustDataPlaneHardeningSupportedDefaultCutoverCloseoutCompleteDecision?: boolean,
  cutoverCloseoutReviewDecision?: boolean,
  expandedScopeLockDecision?: boolean,
  rolloutCapDefinitionDecision?: boolean,
  fallbackMatrixRetentionDecision?: boolean,
  rollbackSwitchVerificationDecision?: boolean,
  telemetrySoakPlanDefinitionDecision?: boolean,
  unsupportedPathBoundaryRetentionDecision?: boolean,
  operatorRolloutAcknowledgementDecision?: boolean,
  finalExpandedDefaultRolloutGuardDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutGuardReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_guard',
    {
      rustDataPlaneHardeningSupportedDefaultCutoverCloseoutCompleteDecision,
      cutoverCloseoutReviewDecision,
      expandedScopeLockDecision,
      rolloutCapDefinitionDecision,
      fallbackMatrixRetentionDecision,
      rollbackSwitchVerificationDecision,
      telemetrySoakPlanDefinitionDecision,
      unsupportedPathBoundaryRetentionDecision,
      operatorRolloutAcknowledgementDecision,
      finalExpandedDefaultRolloutGuardDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutDryRun(
  rustDataPlaneHardeningExpandedDefaultRolloutGuardCompleteDecision?: boolean,
  guardReviewDecision?: boolean,
  expandedManifestReplayDecision?: boolean,
  representativeProfileSimulationDecision?: boolean,
  fallbackRoutingRehearsalDecision?: boolean,
  rollbackRehearsalDecision?: boolean,
  telemetrySoakSampleReviewDecision?: boolean,
  dryRunEvidenceArchiveDecision?: boolean,
  finalExpandedDefaultRolloutDryRunDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutDryRunReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_dry_run',
    {
      rustDataPlaneHardeningExpandedDefaultRolloutGuardCompleteDecision,
      guardReviewDecision,
      expandedManifestReplayDecision,
      representativeProfileSimulationDecision,
      fallbackRoutingRehearsalDecision,
      rollbackRehearsalDecision,
      telemetrySoakSampleReviewDecision,
      dryRunEvidenceArchiveDecision,
      finalExpandedDefaultRolloutDryRunDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutExecution(
  rustDataPlaneHardeningExpandedDefaultRolloutDryRunCompleteDecision?: boolean,
  dryRunReviewDecision?: boolean,
  executionManifestLockDecision?: boolean,
  rolloutWindowStartDecision?: boolean,
  expandedProfileCapEnforcementDecision?: boolean,
  activeTelemetryWatchDecision?: boolean,
  rollbackSwitchArmDecision?: boolean,
  mihomoFallbackRetentionDecision?: boolean,
  operatorExecutionAcknowledgementDecision?: boolean,
  finalExpandedDefaultRolloutExecutionDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutExecutionReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_execution',
    {
      rustDataPlaneHardeningExpandedDefaultRolloutDryRunCompleteDecision,
      dryRunReviewDecision,
      executionManifestLockDecision,
      rolloutWindowStartDecision,
      expandedProfileCapEnforcementDecision,
      activeTelemetryWatchDecision,
      rollbackSwitchArmDecision,
      mihomoFallbackRetentionDecision,
      operatorExecutionAcknowledgementDecision,
      finalExpandedDefaultRolloutExecutionDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutVerification(
  rustDataPlaneHardeningExpandedDefaultRolloutExecutionCompleteDecision?: boolean,
  executionRecordReviewDecision?: boolean,
  expandedProfileTrafficSampleReviewDecision?: boolean,
  fallbackPathSampleVerificationDecision?: boolean,
  rollbackSwitchVerificationDecision?: boolean,
  telemetryHealthBudgetVerificationDecision?: boolean,
  leakRegressionAbsenceVerificationDecision?: boolean,
  verificationEvidenceArchiveDecision?: boolean,
  finalExpandedDefaultRolloutVerificationDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutVerificationReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_verification',
    {
      rustDataPlaneHardeningExpandedDefaultRolloutExecutionCompleteDecision,
      executionRecordReviewDecision,
      expandedProfileTrafficSampleReviewDecision,
      fallbackPathSampleVerificationDecision,
      rollbackSwitchVerificationDecision,
      telemetryHealthBudgetVerificationDecision,
      leakRegressionAbsenceVerificationDecision,
      verificationEvidenceArchiveDecision,
      finalExpandedDefaultRolloutVerificationDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutCloseout(
  rustDataPlaneHardeningExpandedDefaultRolloutVerificationCompleteDecision?: boolean,
  verificationReviewDecision?: boolean,
  expandedRolloutStateDocumentationDecision?: boolean,
  rollbackOwnerAcknowledgementDecision?: boolean,
  fallbackMatrixRetentionDecision?: boolean,
  unsupportedPathBoundaryRetentionDecision?: boolean,
  releaseNotesUpdateDecision?: boolean,
  closeoutEvidenceArchiveDecision?: boolean,
  finalExpandedDefaultRolloutCloseoutDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningExpandedDefaultRolloutCloseoutReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_expanded_default_rollout_closeout',
    {
      rustDataPlaneHardeningExpandedDefaultRolloutVerificationCompleteDecision,
      verificationReviewDecision,
      expandedRolloutStateDocumentationDecision,
      rollbackOwnerAcknowledgementDecision,
      fallbackMatrixRetentionDecision,
      unsupportedPathBoundaryRetentionDecision,
      releaseNotesUpdateDecision,
      closeoutEvidenceArchiveDecision,
      finalExpandedDefaultRolloutCloseoutDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementGuard(
  rustDataPlaneHardeningExpandedDefaultRolloutCloseoutCompleteDecision?: boolean,
  expandedRolloutCloseoutReviewDecision?: boolean,
  protocolParityScopeLockDecision?: boolean,
  tunParityScopeLockDecision?: boolean,
  adapterParityScopeLockDecision?: boolean,
  dnsParityScopeLockDecision?: boolean,
  emergencyRollbackRetentionDecision?: boolean,
  crossPlatformDrillPlanDefinitionDecision?: boolean,
  operatorRetirementAcknowledgementDecision?: boolean,
  finalMihomoFallbackRetirementGuardDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementGuardReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_mihomo_fallback_retirement_guard',
    {
      rustDataPlaneHardeningExpandedDefaultRolloutCloseoutCompleteDecision,
      expandedRolloutCloseoutReviewDecision,
      protocolParityScopeLockDecision,
      tunParityScopeLockDecision,
      adapterParityScopeLockDecision,
      dnsParityScopeLockDecision,
      emergencyRollbackRetentionDecision,
      crossPlatformDrillPlanDefinitionDecision,
      operatorRetirementAcknowledgementDecision,
      finalMihomoFallbackRetirementGuardDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementDryRun(
  rustDataPlaneHardeningMihomoFallbackRetirementGuardCompleteDecision?: boolean,
  guardReviewDecision?: boolean,
  parityManifestReplayDecision?: boolean,
  crossPlatformRollbackRehearsalDecision?: boolean,
  fallbackDependencyInventoryReplayDecision?: boolean,
  emergencyRecoveryRehearsalDecision?: boolean,
  productionForwardingUnchangedVerificationDecision?: boolean,
  dryRunEvidenceArchiveDecision?: boolean,
  finalMihomoFallbackRetirementDryRunDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementDryRunReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_mihomo_fallback_retirement_dry_run',
    {
      rustDataPlaneHardeningMihomoFallbackRetirementGuardCompleteDecision,
      guardReviewDecision,
      parityManifestReplayDecision,
      crossPlatformRollbackRehearsalDecision,
      fallbackDependencyInventoryReplayDecision,
      emergencyRecoveryRehearsalDecision,
      productionForwardingUnchangedVerificationDecision,
      dryRunEvidenceArchiveDecision,
      finalMihomoFallbackRetirementDryRunDecision,
    },
  )
}

export async function getRuntimeKernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementReadiness(
  rustDataPlaneHardeningMihomoFallbackRetirementDryRunCompleteDecision?: boolean,
  dryRunReviewDecision?: boolean,
  protocolParityEvidenceArchiveDecision?: boolean,
  tunParityEvidenceArchiveDecision?: boolean,
  adapterParityEvidenceArchiveDecision?: boolean,
  dnsParityEvidenceArchiveDecision?: boolean,
  soakEvidenceArchiveDecision?: boolean,
  emergencyRollbackOwnerAcknowledgementDecision?: boolean,
  finalMihomoFallbackRetirementReadinessDecision?: boolean,
) {
  return invoke<RuntimeKernelLoopbackRustDataPlaneHardeningMihomoFallbackRetirementReadinessReport>(
    'get_runtime_kernel_loopback_rust_data_plane_hardening_mihomo_fallback_retirement_readiness',
    {
      rustDataPlaneHardeningMihomoFallbackRetirementDryRunCompleteDecision,
      dryRunReviewDecision,
      protocolParityEvidenceArchiveDecision,
      tunParityEvidenceArchiveDecision,
      adapterParityEvidenceArchiveDecision,
      dnsParityEvidenceArchiveDecision,
      soakEvidenceArchiveDecision,
      emergencyRollbackOwnerAcknowledgementDecision,
      finalMihomoFallbackRetirementReadinessDecision,
    },
  )
}
