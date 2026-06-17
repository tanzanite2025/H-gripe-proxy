import { useLockFn } from 'ahooks'
import { Activity, RefreshCw, Route } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import { Select } from '@/components/tailwind/Select'
import {
  activateAppRuntimeProjectionArtifact,
  acceptAppRuntimeDnsHandoff,
  applyAppRuntimeProjectionArtifactToRuntime,
  buildAppRuntimeDemoSeed,
  buildAppRuntimeProjectionArtifact,
  closeoutAppRuntimeStagedActivationLifecycle,
  completeAppRuntimeControlPlane,
  completeAppRuntimeStagedActivationLifecycle,
  decideAppRuntimeRuntimeApplyBoundary,
  deleteAppPolicyBinding,
  deleteAppRegistryEntry,
  deleteDnsProfile,
  deleteNodePool,
  deleteSecurityProfile,
  diagnoseAppRuntime,
  evaluateAppRuntimeSession,
  finishAppRuntimeSession,
  getAppRuntimeState,
  preflightAppRuntimeProjectionActivation,
  recordAppRuntimeSessionObservation,
  projectAppRuntimePlanToMihomo,
  rollbackAppRuntimeProjectionActivation,
  startAppRuntimeSession,
  upsertAppPolicyBinding,
  upsertAppRegistryEntry,
  upsertDnsProfile,
  upsertNodePool,
  upsertSecurityProfile,
  verifyAppRuntimeProjectionRuntimeApply,
  verifyAppRuntimeSessionLeak,
  type AppRuntimeControlPlaneCompletionReport,
  type AppRuntimeDnsHandoffReport,
  type AppRuntimeStagedActivationCloseoutReport,
  type AppRuntimeStagedActivationLifecycleReport,
  type AppPolicyBinding,
  type AppProcessMatcherKind,
  type AppRegistryEntry,
  type AppRoutingIntent,
  type AppRuntimeDiagnosticsReport,
  type AppRuntimeMihomoProjection,
  type AppRuntimePlan,
  type AppRuntimeProjectionActivationPreflightReport,
  type AppRuntimeProjectionArtifact,
  type AppRuntimeProjectionRuntimeVerificationReport,
  type AppRuntimeRuntimeApplyBoundaryDecision,
  type AppRuntimeRuntimeApplyBoundaryDecisionReport,
  type AppRuntimeSessionEvaluationReport,
  type AppRuntimeSessionLeakReport,
  type AppRuntimeStateDocument,
  type DnsProfile,
  type NodePool,
  type SecurityProfile,
} from '@/services/app-runtime'
import {
  dnsControlledRuntimeProbe,
  type DnsResolverRuntimeProbeReport,
} from '@/services/dns-api'
import { showNotice } from '@/services/notice-service'

import {
  AppRuntimeAggregateDiagnosticsPanel,
  type AggregateDiagnosticAction,
} from './app-runtime-aggregate-diagnostics-panel'
import { AppRuntimeAppRegistryForm } from './app-runtime-app-registry-form'
import { AppRuntimeDnsProfileForm } from './app-runtime-dns-profile-form'
import { AppRuntimeNodePoolForm } from './app-runtime-node-pool-form'
import { AppRuntimeOverviewPanel } from './app-runtime-overview-panel'
import { AppRuntimePlanningResultPanel } from './app-runtime-planning-result-panel'
import {
  collectionFor,
  createDnsProfileTemplate,
  createNodePoolTemplate,
  createSecurityProfileTemplate,
  emptyState,
  formatJson,
  newResourceValue,
  now,
  parseJsonObject,
  resourceIdFor,
  resourceNameFor,
  selectAppLabel,
  sortSessions,
  statusColor,
  templateFor,
  upsertSession,
  type FinishableSessionStatus,
  type RuntimeResourceKind,
} from './app-runtime-planning-utils'
import { AppRuntimePolicyBindingForm } from './app-runtime-policy-binding-form'
import { AppRuntimeResourceManagerPanel } from './app-runtime-resource-manager-panel'
import { AppRuntimeSecurityProfileForm } from './app-runtime-security-profile-form'
import { AppRuntimeSessionPanel } from './app-runtime-session-panel'

export function AppRuntimePlanningPanel() {
  const [state, setState] = useState<AppRuntimeStateDocument>(emptyState)
  const [selectedAppId, setSelectedAppId] = useState('')
  const [loading, setLoading] = useState(false)
  const [planning, setPlanning] = useState(false)
  const [sessionPending, setSessionPending] = useState(false)
  const [dnsProbePending, setDnsProbePending] = useState(false)
  const [dnsHandoffPending, setDnsHandoffPending] = useState(false)
  const [controlPlaneCompletionPending, setControlPlaneCompletionPending] =
    useState(false)
  const [
    stagedActivationLifecyclePending,
    setStagedActivationLifecyclePending,
  ] = useState(false)
  const [stagedActivationCloseoutPending, setStagedActivationCloseoutPending] =
    useState(false)
  const [
    runtimeApplyBoundaryDecisionPending,
    setRuntimeApplyBoundaryDecisionPending,
  ] = useState(false)
  const [artifactPending, setArtifactPending] = useState(false)
  const [activationPreflightPending, setActivationPreflightPending] =
    useState(false)
  const [activateMarkerPending, setActivateMarkerPending] = useState(false)
  const [runtimeApplyPending, setRuntimeApplyPending] = useState(false)
  const [runtimeVerificationPending, setRuntimeVerificationPending] =
    useState(false)
  const [activationRollbackPending, setActivationRollbackPending] =
    useState(false)
  const [plan, setPlan] = useState<AppRuntimePlan | null>(null)
  const [projection, setProjection] =
    useState<AppRuntimeMihomoProjection | null>(null)
  const [diagnostics, setDiagnostics] =
    useState<AppRuntimeDiagnosticsReport | null>(null)
  const [projectionArtifact, setProjectionArtifact] =
    useState<AppRuntimeProjectionArtifact | null>(null)
  const [activationPreflight, setActivationPreflight] =
    useState<AppRuntimeProjectionActivationPreflightReport | null>(null)
  const [runtimeVerification, setRuntimeVerification] =
    useState<AppRuntimeProjectionRuntimeVerificationReport | null>(null)
  const [selectedSessionId, setSelectedSessionId] = useState('')
  const [evaluation, setEvaluation] =
    useState<AppRuntimeSessionEvaluationReport | null>(null)
  const [leakReport, setLeakReport] =
    useState<AppRuntimeSessionLeakReport | null>(null)
  const [dnsProbeReport, setDnsProbeReport] =
    useState<DnsResolverRuntimeProbeReport | null>(null)
  const [dnsHandoffReport, setDnsHandoffReport] =
    useState<AppRuntimeDnsHandoffReport | null>(null)
  const [controlPlaneCompletionReport, setControlPlaneCompletionReport] =
    useState<AppRuntimeControlPlaneCompletionReport | null>(null)
  const [stagedActivationLifecycleReport, setStagedActivationLifecycleReport] =
    useState<AppRuntimeStagedActivationLifecycleReport | null>(null)
  const [stagedActivationCloseoutReport, setStagedActivationCloseoutReport] =
    useState<AppRuntimeStagedActivationCloseoutReport | null>(null)
  const [
    runtimeApplyBoundaryDecisionReport,
    setRuntimeApplyBoundaryDecisionReport,
  ] = useState<AppRuntimeRuntimeApplyBoundaryDecisionReport | null>(null)
  const [resourceKind, setResourceKind] = useState<RuntimeResourceKind>('apps')
  const [selectedResourceId, setSelectedResourceId] = useState(newResourceValue)
  const [resourceJson, setResourceJson] = useState('')
  const [bulkJson, setBulkJson] = useState('')
  const [overviewFilter, setOverviewFilter] = useState('')
  const [appDraft, setAppDraft] = useState({
    name: '',
    executablePath: '',
    bundleId: '',
    workingDirectory: '',
    matcherKind: 'process_name' as AppProcessMatcherKind,
    matcherPattern: '',
    tags: '',
  })
  const [nodePoolDraft, setNodePoolDraft] = useState({
    poolId: '',
    name: '',
    region: '',
    protocols: '',
    purpose: '',
    costTier: '',
    candidateNodeName: '',
    candidateProxyGroup: '',
    candidateTags: '',
    tags: '',
  })
  const [dnsProfileDraft, setDnsProfileDraft] = useState({
    profileId: '',
    name: '',
    testDomain: '',
    configYaml: '',
    tags: '',
  })
  const [securityProfileDraft, setSecurityProfileDraft] = useState({
    profileId: '',
    name: '',
    requireNodePool: 'true',
    requireDnsProfile: 'false',
    minRuntimeSupportedNameservers: '',
    allowedRoutingIntents: 'proxy, fallback',
    tags: '',
  })
  const [bindingDraft, setBindingDraft] = useState({
    nodePoolId: '',
    dnsProfileId: '',
    securityProfileId: '',
    routingIntent: 'proxy' as AppRoutingIntent,
    enabled: 'true',
  })
  const [resourcePending, setResourcePending] = useState(false)

  const selectedApp = useMemo(
    () => state.apps.find((app) => app.appId === selectedAppId) ?? null,
    [selectedAppId, state.apps],
  )

  useEffect(() => {
    if (!projectionArtifact) {
      setActivationPreflight(null)
      setRuntimeVerification(null)
    }
  }, [projectionArtifact])

  const latestRuntimeApplyAudit = useMemo(() => {
    if (!projectionArtifact) {
      return null
    }
    return (
      [
        ...state.runtimeApplyAudits.filter(
          (audit) => audit.artifactId === projectionArtifact.artifactId,
        ),
      ].sort(
        (left, right) =>
          right.appliedAt - left.appliedAt ||
          right.auditId.localeCompare(left.auditId),
      )[0] ?? null
    )
  }, [projectionArtifact, state.runtimeApplyAudits])

  const appOptions = useMemo(
    () =>
      state.apps.map((app) => ({
        value: app.appId,
        label: selectAppLabel(app),
      })),
    [state.apps],
  )

  const optionalNodePoolOptions = useMemo(
    () => [
      { value: '', label: '不绑定 node pool' },
      ...state.nodePools.map((nodePool) => ({
        value: nodePool.poolId,
        label: `${nodePool.name} (${nodePool.poolId})`,
      })),
    ],
    [state.nodePools],
  )

  const optionalDnsProfileOptions = useMemo(
    () => [
      { value: '', label: '不绑定 DNS profile' },
      ...state.dnsProfiles.map((profile) => ({
        value: profile.profileId,
        label: `${profile.name} (${profile.profileId})`,
      })),
    ],
    [state.dnsProfiles],
  )

  const optionalSecurityProfileOptions = useMemo(
    () => [
      { value: '', label: '不绑定 security profile' },
      ...state.securityProfiles.map((profile) => ({
        value: profile.profileId,
        label: `${profile.name} (${profile.profileId})`,
      })),
    ],
    [state.securityProfiles],
  )

  const appSessions = useMemo(
    () =>
      sortSessions(
        state.sessions.filter((session) => session.appId === selectedAppId),
      ),
    [selectedAppId, state.sessions],
  )

  const selectedSession = useMemo(
    () =>
      appSessions.find((session) => session.sessionId === selectedSessionId) ??
      appSessions[0] ??
      null,
    [appSessions, selectedSessionId],
  )

  const selectedBinding = useMemo(
    () =>
      state.policyBindings.find(
        (binding) => binding.appId === selectedAppId && binding.enabled,
      ) ??
      state.policyBindings.find((binding) => binding.appId === selectedAppId) ??
      null,
    [selectedAppId, state.policyBindings],
  )

  const selectedDnsProfile = useMemo(() => {
    const dnsProfileId =
      plan?.policyBinding?.dnsProfileId ?? selectedBinding?.dnsProfileId
    return (
      state.dnsProfiles.find((profile) => profile.profileId === dnsProfileId) ??
      null
    )
  }, [plan?.policyBinding?.dnsProfileId, selectedBinding, state.dnsProfiles])

  const selectedNodePool = useMemo(() => {
    const nodePoolId =
      plan?.policyBinding?.nodePoolId ?? selectedBinding?.nodePoolId
    return (
      state.nodePools.find((nodePool) => nodePool.poolId === nodePoolId) ?? null
    )
  }, [plan?.policyBinding?.nodePoolId, selectedBinding, state.nodePools])

  const selectedSecurityProfile = useMemo(() => {
    const securityProfileId =
      plan?.policyBinding?.securityProfileId ??
      selectedBinding?.securityProfileId
    return (
      state.securityProfiles.find(
        (profile) => profile.profileId === securityProfileId,
      ) ?? null
    )
  }, [
    plan?.policyBinding?.securityProfileId,
    selectedBinding,
    state.securityProfiles,
  ])

  const overviewRows = useMemo(
    () =>
      state.apps.map((app) => {
        const binding =
          state.policyBindings.find(
            (item) => item.appId === app.appId && item.enabled,
          ) ??
          state.policyBindings.find((item) => item.appId === app.appId) ??
          null
        const nodePool =
          state.nodePools.find((item) => item.poolId === binding?.nodePoolId) ??
          null
        const dnsProfile =
          state.dnsProfiles.find(
            (item) => item.profileId === binding?.dnsProfileId,
          ) ?? null
        const securityProfile =
          state.securityProfiles.find(
            (item) => item.profileId === binding?.securityProfileId,
          ) ?? null
        const sessions = state.sessions.filter(
          (session) => session.appId === app.appId,
        )
        const issues = [
          !binding ? 'missing binding' : '',
          binding?.enabled === false ? 'binding disabled' : '',
          binding?.nodePoolId && !nodePool
            ? `missing node pool: ${binding.nodePoolId}`
            : '',
          binding?.dnsProfileId && !dnsProfile
            ? `missing DNS profile: ${binding.dnsProfileId}`
            : '',
          binding?.securityProfileId && !securityProfile
            ? `missing security profile: ${binding.securityProfileId}`
            : '',
        ].filter(Boolean)
        return {
          app,
          binding,
          nodePool,
          dnsProfile,
          securityProfile,
          sessions,
          openSessions: sessions.filter((session) => !session.endedAt).length,
          issues,
        }
      }),
    [state],
  )

  const filteredOverviewRows = useMemo(() => {
    const query = overviewFilter.trim().toLowerCase()
    if (!query) {
      return overviewRows
    }
    return overviewRows.filter((row) =>
      [
        row.app.appId,
        row.app.name,
        row.binding?.bindingId ?? '',
        row.binding?.routingIntent ?? '',
        row.nodePool?.name ?? row.binding?.nodePoolId ?? '',
        row.dnsProfile?.name ?? row.binding?.dnsProfileId ?? '',
        row.securityProfile?.name ?? row.binding?.securityProfileId ?? '',
        ...row.app.tags,
        ...row.issues,
      ]
        .join(' ')
        .toLowerCase()
        .includes(query),
    )
  }, [overviewFilter, overviewRows])

  const selectedOverviewRow = useMemo(
    () => overviewRows.find((row) => row.app.appId === selectedAppId) ?? null,
    [overviewRows, selectedAppId],
  )

  const aggregateDiagnostics = useMemo(() => {
    if (!selectedApp) {
      return []
    }

    const dnsProbeStatus = !selectedDnsProfile
      ? 'skipped'
      : dnsProbeReport
        ? dnsProbeReport.summary.failedTargets > 0
          ? 'failed'
          : dnsProbeReport.warnings.length > 0
            ? 'warning'
            : 'passed'
        : 'skipped'

    return [
      {
        key: 'state',
        label: 'State references',
        status: selectedOverviewRow?.issues.length ? 'failed' : 'passed',
        detail: selectedOverviewRow?.issues.length
          ? selectedOverviewRow.issues.join('；')
          : 'App / binding / node / DNS / security references resolved.',
      },
      {
        key: 'diagnostics',
        label: 'Planning diagnostics',
        status: diagnostics?.status ?? 'skipped',
        detail: diagnostics
          ? `${diagnostics.summary.failed} failed / ${diagnostics.summary.warnings} warnings / ${diagnostics.summary.passed} passed`
          : 'Run planning diagnostics to populate Rust checks.',
      },
      {
        key: 'dns-probe',
        label: 'DNS controlled probe',
        status: dnsProbeStatus,
        detail: !selectedDnsProfile
          ? 'No DNS profile bound.'
          : dnsProbeReport
            ? `${dnsProbeReport.summary.healthyTargets}/${dnsProbeReport.summary.runtimeSupportedTargets} runtime-supported targets healthy.`
            : 'Probe is opt-in; run it before treating DNS runtime health as known.',
      },
      {
        key: 'runtime-boundary',
        label: 'Runtime boundary',
        status: projection
          ? projection.mutatesRuntime
            ? 'failed'
            : 'passed'
          : 'skipped',
        detail: projection
          ? projection.mutatesRuntime
            ? 'Projection reports runtime mutation.'
            : 'Projection remains planning-only.'
          : 'Generate projection through planning diagnostics.',
      },
      {
        key: 'dns-handoff',
        label: 'DNS handoff intake',
        status: dnsHandoffReport?.status ?? 'skipped',
        detail: dnsHandoffReport
          ? `${dnsHandoffReport.nextAppRuntimeStep}; phase8=${String(dnsHandoffReport.phase8Allowed)}`
          : 'Accept DNS expanded control-plane completion before next app-runtime follow-up.',
      },
      {
        key: 'control-plane-completion',
        label: 'Control-plane completion',
        status: controlPlaneCompletionReport?.status ?? 'skipped',
        detail: controlPlaneCompletionReport
          ? `${controlPlaneCompletionReport.nextAppRuntimeStep}; staged=${String(controlPlaneCompletionReport.readyForStagedActivation)}`
          : 'Complete DNS handoff, projection artifact, and staged activation preflight together.',
      },
      {
        key: 'staged-activation-lifecycle',
        label: 'Staged activation lifecycle',
        status: stagedActivationLifecycleReport?.status ?? 'skipped',
        detail: stagedActivationLifecycleReport
          ? `${stagedActivationLifecycleReport.nextAppRuntimeStep}; marker=${String(stagedActivationLifecycleReport.markerActivated)}`
          : 'Complete control-plane and activate the staged marker in one explicit step.',
      },
      {
        key: 'staged-activation-closeout',
        label: 'Runtime-apply boundary closeout',
        status: stagedActivationCloseoutReport?.status ?? 'skipped',
        detail: stagedActivationCloseoutReport
          ? `${stagedActivationCloseoutReport.nextAppRuntimeStep}; manifest=${String(stagedActivationCloseoutReport.boundaryManifestPersisted)}`
          : 'Close out staged activation with a persisted runtime-apply boundary manifest.',
      },
    ]
  }, [
    controlPlaneCompletionReport,
    diagnostics,
    dnsHandoffReport,
    dnsProbeReport,
    projection,
    selectedApp,
    selectedDnsProfile,
    selectedOverviewRow,
    stagedActivationCloseoutReport,
    stagedActivationLifecycleReport,
  ])

  const aggregateDiagnosticActions = useMemo(() => {
    if (!selectedApp) {
      return []
    }

    const actions: AggregateDiagnosticAction[] = []

    for (const issue of selectedOverviewRow?.issues ?? []) {
      actions.push({
        key: `state-${issue}`,
        scope: 'State',
        status: 'failed',
        message: issue,
        detail: 'Use the quick forms or JSON editor to repair this reference.',
        action: 'focus-state',
        actionLabel: '定位',
      })
    }

    if (!diagnostics) {
      actions.push({
        key: 'diagnostics-run',
        scope: 'Diagnostics',
        status: 'skipped',
        message: 'Planning diagnostics not run',
        detail:
          'Run planning diagnostics before reviewing projection readiness.',
        action: 'run-diagnostics',
        actionLabel: '运行',
      })
    } else {
      for (const check of diagnostics.checks.filter(
        (item) => item.status === 'failed' || item.status === 'warning',
      )) {
        actions.push({
          key: `diagnostics-${check.checkId}`,
          scope: check.category,
          status: check.status,
          message: check.message,
          detail: check.details.join('；') || check.severity,
        })
      }
    }

    if (selectedDnsProfile && !dnsProbeReport) {
      actions.push({
        key: 'dns-probe-run',
        scope: 'DNS',
        status: 'skipped',
        message: 'DNS controlled probe not run',
        detail:
          'Run the opt-in probe before treating DNS runtime health as known.',
        action: 'run-dns-probe',
        actionLabel: '探测',
      })
    }

    if (dnsProbeReport) {
      for (const target of dnsProbeReport.targets.filter(
        (item) => item.runtimeSupported && !item.healthy,
      )) {
        actions.push({
          key: `dns-target-${target.protocol}-${target.server}`,
          scope: 'DNS',
          status: 'failed',
          message: `${target.server} · ${target.providerLabel ?? target.protocol}`,
          detail: target.message,
          action: 'run-dns-probe',
          actionLabel: '重试',
        })
      }

      for (const warning of dnsProbeReport.warnings) {
        actions.push({
          key: `dns-warning-${warning}`,
          scope: 'DNS',
          status: 'warning',
          message: warning,
          detail:
            'Probe warning remains informational unless promoted by policy.',
          action: 'run-dns-probe',
          actionLabel: '重试',
        })
      }
    }

    if (projection?.mutatesRuntime) {
      actions.push({
        key: 'runtime-boundary-mutates',
        scope: 'Runtime',
        status: 'failed',
        message: 'Projection reports runtime mutation',
        detail: 'App-runtime UI should stay planning-only at this phase.',
      })
    }

    if (!dnsHandoffReport) {
      actions.push({
        key: 'dns-handoff-accept',
        scope: 'DNS handoff',
        status: 'skipped',
        message: 'DNS expanded handoff not accepted',
        detail:
          'Accept the DNS control-plane completion manifest before app-runtime follow-up.',
        action: 'accept-dns-handoff',
        actionLabel: '接收',
      })
    } else if (dnsHandoffReport.status !== 'accepted') {
      actions.push({
        key: 'dns-handoff-status',
        scope: 'DNS handoff',
        status: dnsHandoffReport.status,
        message: dnsHandoffReport.reason,
        detail:
          dnsHandoffReport.blockers.join('；') ||
          dnsHandoffReport.nextAppRuntimeStep,
        action: 'accept-dns-handoff',
        actionLabel: '重试',
      })
    }

    if (!controlPlaneCompletionReport) {
      actions.push({
        key: 'control-plane-completion-run',
        scope: 'Completion',
        status: 'skipped',
        message: 'App runtime control-plane completion not run',
        detail:
          'Run the combined DNS handoff + projection artifact + preflight bundle.',
        action: 'complete-control-plane',
        actionLabel: '完成',
      })
    } else if (controlPlaneCompletionReport.status === 'blocked') {
      actions.push({
        key: 'control-plane-completion-blocked',
        scope: 'Completion',
        status: 'blocked',
        message: controlPlaneCompletionReport.reason,
        detail:
          controlPlaneCompletionReport.blockers.join('；') ||
          controlPlaneCompletionReport.nextAppRuntimeStep,
        action: 'complete-control-plane',
        actionLabel: '重试',
      })
    }

    if (!stagedActivationLifecycleReport) {
      actions.push({
        key: 'staged-activation-lifecycle-run',
        scope: 'Staged activation',
        status: 'skipped',
        message: 'Staged activation lifecycle not run',
        detail:
          'Run the combined control-plane completion and staged marker activation bundle.',
        action: 'complete-staged-activation-lifecycle',
        actionLabel: '激活',
      })
    } else if (stagedActivationLifecycleReport.status === 'blocked') {
      actions.push({
        key: 'staged-activation-lifecycle-blocked',
        scope: 'Staged activation',
        status: 'blocked',
        message: stagedActivationLifecycleReport.reason,
        detail:
          stagedActivationLifecycleReport.blockers.join('；') ||
          stagedActivationLifecycleReport.nextAppRuntimeStep,
        action: 'complete-staged-activation-lifecycle',
        actionLabel: '重试',
      })
    }

    if (!stagedActivationCloseoutReport) {
      actions.push({
        key: 'staged-activation-closeout-run',
        scope: 'Closeout',
        status: 'skipped',
        message: 'Runtime-apply boundary closeout not run',
        detail:
          'Persist the staged activation closeout manifest before any runtime-apply discussion.',
        action: 'closeout-staged-activation',
        actionLabel: '收口',
      })
    } else if (stagedActivationCloseoutReport.status === 'blocked') {
      actions.push({
        key: 'staged-activation-closeout-blocked',
        scope: 'Closeout',
        status: 'blocked',
        message: stagedActivationCloseoutReport.reason,
        detail:
          stagedActivationCloseoutReport.blockers.join('；') ||
          stagedActivationCloseoutReport.nextAppRuntimeStep,
        action: 'closeout-staged-activation',
        actionLabel: '重试',
      })
    }

    if (actions.length === 0) {
      actions.push({
        key: 'no-actions',
        scope: 'Summary',
        status: 'passed',
        message: 'No aggregate actions pending',
        detail:
          'State references, diagnostics, DNS probe, and runtime boundary are clear.',
      })
    }

    return actions
  }, [
    controlPlaneCompletionReport,
    diagnostics,
    dnsHandoffReport,
    dnsProbeReport,
    projection?.mutatesRuntime,
    selectedApp,
    selectedDnsProfile,
    selectedOverviewRow,
    stagedActivationCloseoutReport,
    stagedActivationLifecycleReport,
  ])

  const resources = useMemo(
    () => collectionFor(state, resourceKind),
    [resourceKind, state],
  )

  const resourceOptions = useMemo(
    () => [
      { value: newResourceValue, label: '新建资源' },
      ...resources.map((resource) => {
        const resourceId = resourceIdFor(resourceKind, resource)
        return {
          value: resourceId,
          label: `${resourceNameFor(resourceKind, resource)} (${resourceId})`,
        }
      }),
    ],
    [resourceKind, resources],
  )

  const selectAppForDiagnostics = (appId: string) => {
    setSelectedAppId(appId)
    setSelectedSessionId('')
    setPlan(null)
    setProjection(null)
    setDiagnostics(null)
    setEvaluation(null)
    setLeakReport(null)
    setDnsProbeReport(null)
    setProjectionArtifact(null)
  }

  const loadState = useLockFn(async () => {
    setLoading(true)
    try {
      const nextState = await getAppRuntimeState()
      setState(nextState)
      setSelectedAppId((current) => current || nextState.apps[0]?.appId || '')
      setSelectedSessionId(
        (current) => current || nextState.sessions[0]?.sessionId || '',
      )
    } catch (error) {
      showNotice.error(error)
    } finally {
      setLoading(false)
    }
  })

  const runPlanningDiagnostics = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setPlanning(true)
    try {
      const request = { appId: selectedAppId }
      const [nextDiagnostics, nextProjection] = await Promise.all([
        diagnoseAppRuntime(request),
        projectAppRuntimePlanToMihomo(request),
      ])
      setPlan(nextDiagnostics.plan)
      setProjection(nextProjection)
      setDiagnostics(nextDiagnostics)
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('应用运行时规划诊断已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPlanning(false)
    }
  })

  const handleProbeSelectedDnsProfile = useLockFn(async () => {
    if (!selectedDnsProfile) {
      return
    }

    setDnsProbePending(true)
    try {
      const report = await dnsControlledRuntimeProbe(
        selectedDnsProfile.configYaml,
        selectedDnsProfile.testDomain || 'example.com',
      )
      setDnsProbeReport(report)
      showNotice.success('应用绑定 DNS profile 受控探测已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setDnsProbePending(false)
    }
  })

  const runSelectedAppReadiness = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    await runPlanningDiagnostics()

    if (selectedDnsProfile) {
      await handleProbeSelectedDnsProfile()
    }
  })

  const acceptDnsHandoff = useLockFn(async () => {
    setDnsHandoffPending(true)
    try {
      const report = await acceptAppRuntimeDnsHandoff()
      setDnsHandoffReport(report)
      showNotice.success('App runtime DNS handoff intake 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setDnsHandoffPending(false)
    }
  })

  const completeControlPlane = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setControlPlaneCompletionPending(true)
    try {
      const report = await completeAppRuntimeControlPlane({
        appId: selectedAppId,
      })
      setControlPlaneCompletionReport(report)
      setDnsHandoffReport(report.dnsHandoff)
      setProjectionArtifact(report.projectionArtifact)
      setActivationPreflight(report.activationPreflight)
      setPlan(report.projectionArtifact.plan)
      setProjection(report.projectionArtifact.projection)
      setDiagnostics(report.projectionArtifact.diagnostics)
      showNotice.success('App runtime control-plane completion 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setControlPlaneCompletionPending(false)
    }
  })

  const completeStagedActivationLifecycle = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setStagedActivationLifecyclePending(true)
    try {
      const report = await completeAppRuntimeStagedActivationLifecycle({
        appId: selectedAppId,
      })
      setStagedActivationLifecycleReport(report)
      setControlPlaneCompletionReport(report.controlPlaneCompletion)
      setDnsHandoffReport(report.controlPlaneCompletion.dnsHandoff)
      setProjectionArtifact(report.controlPlaneCompletion.projectionArtifact)
      setActivationPreflight(report.controlPlaneCompletion.activationPreflight)
      setPlan(report.controlPlaneCompletion.projectionArtifact.plan)
      setProjection(report.controlPlaneCompletion.projectionArtifact.projection)
      setDiagnostics(
        report.controlPlaneCompletion.projectionArtifact.diagnostics,
      )
      setState(await getAppRuntimeState())
      showNotice.success('App runtime staged activation lifecycle 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setStagedActivationLifecyclePending(false)
    }
  })

  const closeoutStagedActivation = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setStagedActivationCloseoutPending(true)
    try {
      const report = await closeoutAppRuntimeStagedActivationLifecycle({
        appId: selectedAppId,
      })
      setStagedActivationCloseoutReport(report)
      setStagedActivationLifecycleReport(report.lifecycle)
      setControlPlaneCompletionReport(report.lifecycle.controlPlaneCompletion)
      setDnsHandoffReport(report.lifecycle.controlPlaneCompletion.dnsHandoff)
      setProjectionArtifact(
        report.lifecycle.controlPlaneCompletion.projectionArtifact,
      )
      setActivationPreflight(
        report.lifecycle.controlPlaneCompletion.activationPreflight,
      )
      setPlan(report.lifecycle.controlPlaneCompletion.projectionArtifact.plan)
      setProjection(
        report.lifecycle.controlPlaneCompletion.projectionArtifact.projection,
      )
      setDiagnostics(
        report.lifecycle.controlPlaneCompletion.projectionArtifact.diagnostics,
      )
      setState(await getAppRuntimeState())
      showNotice.success('App runtime runtime-apply boundary closeout 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setStagedActivationCloseoutPending(false)
    }
  })

  const decideRuntimeApplyBoundary = useLockFn(
    async (
      decision: AppRuntimeRuntimeApplyBoundaryDecision,
      rationale: string,
    ) => {
      if (!selectedAppId) {
        return
      }

      setRuntimeApplyBoundaryDecisionPending(true)
      try {
        const report = await decideAppRuntimeRuntimeApplyBoundary({
          appId: selectedAppId,
          decision,
          rationale,
        })
        setRuntimeApplyBoundaryDecisionReport(report)
        setStagedActivationCloseoutReport(report.closeout)
        setStagedActivationLifecycleReport(report.closeout.lifecycle)
        setControlPlaneCompletionReport(
          report.closeout.lifecycle.controlPlaneCompletion,
        )
        setDnsHandoffReport(
          report.closeout.lifecycle.controlPlaneCompletion.dnsHandoff,
        )
        setProjectionArtifact(
          report.closeout.lifecycle.controlPlaneCompletion.projectionArtifact,
        )
        setActivationPreflight(
          report.closeout.lifecycle.controlPlaneCompletion.activationPreflight,
        )
        setPlan(
          report.closeout.lifecycle.controlPlaneCompletion.projectionArtifact
            .plan,
        )
        setProjection(
          report.closeout.lifecycle.controlPlaneCompletion.projectionArtifact
            .projection,
        )
        setDiagnostics(
          report.closeout.lifecycle.controlPlaneCompletion.projectionArtifact
            .diagnostics,
        )
        showNotice.success('Runtime-apply boundary 显式决策已记录')
      } catch (error) {
        showNotice.error(error)
      } finally {
        setRuntimeApplyBoundaryDecisionPending(false)
      }
    },
  )

  const buildProjectionArtifact = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setArtifactPending(true)
    try {
      const artifact = await buildAppRuntimeProjectionArtifact({
        appId: selectedAppId,
      })
      setProjectionArtifact(artifact)
      setActivationPreflight(null)
      setPlan(artifact.plan)
      setProjection(artifact.projection)
      setDiagnostics(artifact.diagnostics)
      showNotice.success('应用运行时 projection artifact dry-run 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setArtifactPending(false)
    }
  })

  const preflightProjectionActivation = useLockFn(async () => {
    if (!projectionArtifact) {
      return
    }

    setActivationPreflightPending(true)
    try {
      const report = await preflightAppRuntimeProjectionActivation({
        artifactId: projectionArtifact.artifactId,
        expectedChecksum: projectionArtifact.checksum,
      })
      setActivationPreflight(report)
      showNotice.success(
        report.status === 'blocked'
          ? '受控激活 preflight 已完成：guard 阻止 runtime mutation'
          : '受控激活 preflight 已完成',
      )
    } catch (error) {
      showNotice.error(error)
    } finally {
      setActivationPreflightPending(false)
    }
  })

  const markProjectionArtifactActive = useLockFn(async () => {
    if (!projectionArtifact) {
      return
    }

    setActivateMarkerPending(true)
    try {
      const nextState = await activateAppRuntimeProjectionArtifact({
        artifactId: projectionArtifact.artifactId,
        expectedChecksum: projectionArtifact.checksum,
      })
      setState(nextState)
      showNotice.success(
        '已标记 active projection artifact（未 reload Mihomo）',
      )
    } catch (error) {
      showNotice.error(error)
    } finally {
      setActivateMarkerPending(false)
    }
  })

  const rollbackProjectionActivation = useLockFn(async () => {
    if (!state.activeProjection) {
      return
    }

    setActivationRollbackPending(true)
    try {
      const nextState = await rollbackAppRuntimeProjectionActivation()
      setState(nextState)
      showNotice.success(
        '已回滚 active projection；如有 runtime candidate 已恢复',
      )
    } catch (error) {
      showNotice.error(error)
    } finally {
      setActivationRollbackPending(false)
    }
  })

  const applyProjectionArtifactToRuntime = useLockFn(async () => {
    if (!projectionArtifact) {
      return
    }
    if (!runtimeApplyDecisionAllowsCandidate) {
      showNotice.error('请先完成 runtime-apply boundary 的显式 allow 决策')
      return
    }
    const decisionRecord = runtimeApplyBoundaryDecisionReport?.decisionRecord
    if (!decisionRecord) {
      return
    }

    setRuntimeApplyPending(true)
    try {
      const nextState = await applyAppRuntimeProjectionArtifactToRuntime({
        artifactId: projectionArtifact.artifactId,
        expectedChecksum: projectionArtifact.checksum,
        runtimeApplyDecisionId: decisionRecord.decisionId,
        expectedRuntimeApplyDecisionChecksum: decisionRecord.checksum,
        force: true,
      })
      setState(nextState)
      setRuntimeVerification(null)
      showNotice.success('已显式应用 projection runtime candidate')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setRuntimeApplyPending(false)
    }
  })

  const verifyProjectionRuntimeApply = useLockFn(async () => {
    if (!projectionArtifact) {
      return
    }

    setRuntimeVerificationPending(true)
    try {
      const report = await verifyAppRuntimeProjectionRuntimeApply({
        artifactId: projectionArtifact.artifactId,
      })
      setRuntimeVerification(report)
      setState(await getAppRuntimeState())
      showNotice.success(
        report.status === 'healthy'
          ? 'runtime apply 运行态验证已通过'
          : 'runtime apply 运行态验证已完成',
      )
    } catch (error) {
      showNotice.error(error)
    } finally {
      setRuntimeVerificationPending(false)
    }
  })

  const focusStateAction = (message: string) => {
    setOverviewFilter(message)

    if (message === 'missing binding' || message === 'binding disabled') {
      setResourceKind('policyBindings')
      setSelectedResourceId(selectedBinding?.bindingId ?? newResourceValue)
      return
    }

    if (message.startsWith('missing node pool: ')) {
      setResourceKind('nodePools')
      setSelectedResourceId(message.replace('missing node pool: ', ''))
      return
    }

    if (message.startsWith('missing DNS profile: ')) {
      setResourceKind('dnsProfiles')
      setSelectedResourceId(message.replace('missing DNS profile: ', ''))
      return
    }

    if (message.startsWith('missing security profile: ')) {
      setResourceKind('securityProfiles')
      setSelectedResourceId(message.replace('missing security profile: ', ''))
    }
  }

  const handleAggregateDiagnosticAction = (
    action: AggregateDiagnosticAction,
  ) => {
    if (action.action === 'focus-state') {
      focusStateAction(action.message)
      return
    }

    if (action.action === 'run-diagnostics') {
      void runPlanningDiagnostics()
      return
    }

    if (action.action === 'run-dns-probe') {
      void handleProbeSelectedDnsProfile()
      return
    }

    if (action.action === 'accept-dns-handoff') {
      void acceptDnsHandoff()
      return
    }

    if (action.action === 'complete-control-plane') {
      void completeControlPlane()
      return
    }

    if (action.action === 'complete-staged-activation-lifecycle') {
      void completeStagedActivationLifecycle()
      return
    }

    if (action.action === 'closeout-staged-activation') {
      void closeoutStagedActivation()
    }
  }

  const handleStartSession = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setSessionPending(true)
    try {
      const report = await startAppRuntimeSession({ appId: selectedAppId })
      setState((current) => ({
        ...current,
        sessions: upsertSession(current.sessions, report.session),
      }))
      setSelectedSessionId(report.session.sessionId)
      setDiagnostics(report.diagnostics)
      setPlan(report.diagnostics.plan)
      setProjection(null)
      setEvaluation(null)
      setLeakReport(null)
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('应用运行时 session 已开始记录')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setSessionPending(false)
    }
  })

  const handleRecordObservation = useLockFn(async () => {
    if (!selectedSession) {
      return
    }

    setSessionPending(true)
    try {
      const session = await recordAppRuntimeSessionObservation(
        selectedSession.sessionId,
      )
      setState((current) => ({
        ...current,
        sessions: upsertSession(current.sessions, session),
      }))
      setSelectedSessionId(session.sessionId)
      setEvaluation(null)
      setLeakReport(null)
      showNotice.success('已记录连接指标快照')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setSessionPending(false)
    }
  })

  const handleEvaluateSession = useLockFn(async () => {
    if (!selectedSession) {
      return
    }

    setSessionPending(true)
    try {
      const report = await evaluateAppRuntimeSession(selectedSession.sessionId)
      setEvaluation(report)
      showNotice.success('Session 归因评估已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setSessionPending(false)
    }
  })

  const handleVerifySessionLeak = useLockFn(async () => {
    if (!selectedSession) {
      return
    }

    setSessionPending(true)
    try {
      const report = await verifyAppRuntimeSessionLeak(
        selectedSession.sessionId,
      )
      setLeakReport(report)
      showNotice.success('Session 泄漏维度检查已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setSessionPending(false)
    }
  })

  const handleFinishSession = useLockFn(
    async (status: FinishableSessionStatus) => {
      if (!selectedSession) {
        return
      }

      setSessionPending(true)
      try {
        const session = await finishAppRuntimeSession({
          sessionId: selectedSession.sessionId,
          status,
        })
        setState((current) => ({
          ...current,
          sessions: upsertSession(current.sessions, session),
        }))
        setSelectedSessionId(session.sessionId)
        showNotice.success(`Session 已标记为 ${status}`)
      } catch (error) {
        showNotice.error(error)
      } finally {
        setSessionPending(false)
      }
    },
  )

  const handleSaveAppDraft = useLockFn(async () => {
    if (!selectedApp) {
      return
    }

    setResourcePending(true)
    try {
      const processMatchers = appDraft.matcherPattern.trim()
        ? [
            {
              kind: appDraft.matcherKind,
              pattern: appDraft.matcherPattern.trim(),
            },
          ]
        : selectedApp.processMatchers
      const nextApp: AppRegistryEntry = {
        ...selectedApp,
        name: appDraft.name.trim() || selectedApp.name,
        executablePath: appDraft.executablePath.trim() || undefined,
        bundleId: appDraft.bundleId.trim() || undefined,
        workingDirectory: appDraft.workingDirectory.trim() || undefined,
        processMatchers,
        tags: appDraft.tags
          .split(',')
          .map((tag) => tag.trim())
          .filter(Boolean),
        updatedAt: now(),
      }
      const nextState = await upsertAppRegistryEntry(nextApp)
      setState(nextState)
      setSelectedAppId(nextApp.appId)
      setResourceKind('apps')
      setSelectedResourceId(nextApp.appId)
      setPlan(null)
      setProjection(null)
      setDiagnostics(null)
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('应用注册信息已保存')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setResourcePending(false)
    }
  })

  const handleSaveNodePoolDraft = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setResourcePending(true)
    try {
      const poolId = nodePoolDraft.poolId.trim() || `pool-${selectedAppId}`
      const candidateNodeName = nodePoolDraft.candidateNodeName.trim()
      const candidateNodes = candidateNodeName
        ? [
            {
              nodeName: candidateNodeName,
              proxyGroup: nodePoolDraft.candidateProxyGroup.trim() || undefined,
              tags: nodePoolDraft.candidateTags
                .split(',')
                .map((tag) => tag.trim())
                .filter(Boolean),
            },
          ]
        : (selectedNodePool?.candidateNodes ?? [])
      const nextNodePool: NodePool = {
        ...(selectedNodePool ?? createNodePoolTemplate()),
        poolId,
        name: nodePoolDraft.name.trim() || selectedNodePool?.name || poolId,
        region: nodePoolDraft.region.trim() || undefined,
        protocols: nodePoolDraft.protocols
          .split(',')
          .map((protocol) => protocol.trim())
          .filter(Boolean),
        purpose: nodePoolDraft.purpose.trim() || undefined,
        costTier: nodePoolDraft.costTier.trim() || undefined,
        candidateNodes,
        tags: nodePoolDraft.tags
          .split(',')
          .map((tag) => tag.trim())
          .filter(Boolean),
        updatedAt: now(),
      }
      const nextState = await upsertNodePool(nextNodePool)
      setState(nextState)
      setResourceKind('nodePools')
      setSelectedResourceId(nextNodePool.poolId)
      setBindingDraft((draft) => ({
        ...draft,
        nodePoolId: nextNodePool.poolId,
      }))
      setPlan(null)
      setProjection(null)
      setDiagnostics(null)
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('节点池已保存')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setResourcePending(false)
    }
  })

  const handleSaveDnsProfileDraft = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setResourcePending(true)
    try {
      const profileId =
        dnsProfileDraft.profileId.trim() || `dns-${selectedAppId}`
      const nextProfile: DnsProfile = {
        ...(selectedDnsProfile ?? createDnsProfileTemplate()),
        profileId,
        name:
          dnsProfileDraft.name.trim() || selectedDnsProfile?.name || profileId,
        configYaml:
          dnsProfileDraft.configYaml.trim() ||
          selectedDnsProfile?.configYaml ||
          'nameserver:\n  - 1.1.1.1',
        testDomain: dnsProfileDraft.testDomain.trim() || undefined,
        tags: dnsProfileDraft.tags
          .split(',')
          .map((tag) => tag.trim())
          .filter(Boolean),
        updatedAt: now(),
      }
      const nextState = await upsertDnsProfile(nextProfile)
      setState(nextState)
      setResourceKind('dnsProfiles')
      setSelectedResourceId(nextProfile.profileId)
      setBindingDraft((draft) => ({
        ...draft,
        dnsProfileId: nextProfile.profileId,
      }))
      setPlan(null)
      setProjection(null)
      setDiagnostics(null)
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('DNS profile 已保存')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setResourcePending(false)
    }
  })

  const handleSaveSecurityProfileDraft = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setResourcePending(true)
    try {
      const profileId =
        securityProfileDraft.profileId.trim() || `security-${selectedAppId}`
      const minRuntimeSupportedNameservers =
        securityProfileDraft.minRuntimeSupportedNameservers.trim()
          ? Number(securityProfileDraft.minRuntimeSupportedNameservers)
          : undefined
      const nextProfile: SecurityProfile = {
        ...(selectedSecurityProfile ?? createSecurityProfileTemplate()),
        profileId,
        name:
          securityProfileDraft.name.trim() ||
          selectedSecurityProfile?.name ||
          profileId,
        controls: {
          requireNodePool: securityProfileDraft.requireNodePool === 'true',
          requireDnsProfile: securityProfileDraft.requireDnsProfile === 'true',
          minRuntimeSupportedNameservers:
            minRuntimeSupportedNameservers !== undefined &&
            Number.isFinite(minRuntimeSupportedNameservers)
              ? minRuntimeSupportedNameservers
              : undefined,
          allowedRoutingIntents: securityProfileDraft.allowedRoutingIntents
            .split(',')
            .map((intent) => intent.trim())
            .filter(Boolean) as AppRoutingIntent[],
        },
        tags: securityProfileDraft.tags
          .split(',')
          .map((tag) => tag.trim())
          .filter(Boolean),
        updatedAt: now(),
      }
      const nextState = await upsertSecurityProfile(nextProfile)
      setState(nextState)
      setResourceKind('securityProfiles')
      setSelectedResourceId(nextProfile.profileId)
      setBindingDraft((draft) => ({
        ...draft,
        securityProfileId: nextProfile.profileId,
      }))
      setPlan(null)
      setProjection(null)
      setDiagnostics(null)
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('Security profile 已保存')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setResourcePending(false)
    }
  })

  const handleSaveBindingDraft = useLockFn(async () => {
    if (!selectedAppId) {
      return
    }

    setResourcePending(true)
    try {
      const bindingId = selectedBinding?.bindingId || `binding-${selectedAppId}`
      const nextState = await upsertAppPolicyBinding({
        bindingId,
        appId: selectedAppId,
        nodePoolId: bindingDraft.nodePoolId || undefined,
        dnsProfileId: bindingDraft.dnsProfileId || undefined,
        securityProfileId: bindingDraft.securityProfileId || undefined,
        routingIntent: bindingDraft.routingIntent,
        enabled: bindingDraft.enabled === 'true',
        updatedAt: now(),
      })
      setState(nextState)
      setResourceKind('policyBindings')
      setSelectedResourceId(bindingId)
      setPlan(null)
      setProjection(null)
      setDiagnostics(null)
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('应用策略绑定已保存')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setResourcePending(false)
    }
  })

  const handleSaveResource = useLockFn(async () => {
    setResourcePending(true)
    try {
      let nextState: AppRuntimeStateDocument
      let nextResourceId = ''
      switch (resourceKind) {
        case 'apps': {
          const entry = parseJsonObject<AppRegistryEntry>(resourceJson)
          nextState = await upsertAppRegistryEntry({
            ...entry,
            updatedAt: now(),
          })
          nextResourceId = entry.appId
          setSelectedAppId(entry.appId)
          break
        }
        case 'nodePools': {
          const nodePool = parseJsonObject<NodePool>(resourceJson)
          nextState = await upsertNodePool({
            ...nodePool,
            updatedAt: now(),
          })
          nextResourceId = nodePool.poolId
          break
        }
        case 'dnsProfiles': {
          const dnsProfile = parseJsonObject<DnsProfile>(resourceJson)
          nextState = await upsertDnsProfile({
            ...dnsProfile,
            updatedAt: now(),
          })
          nextResourceId = dnsProfile.profileId
          break
        }
        case 'securityProfiles': {
          const securityProfile = parseJsonObject<SecurityProfile>(resourceJson)
          nextState = await upsertSecurityProfile({
            ...securityProfile,
            updatedAt: now(),
          })
          nextResourceId = securityProfile.profileId
          break
        }
        case 'policyBindings': {
          const binding = parseJsonObject<AppPolicyBinding>(resourceJson)
          nextState = await upsertAppPolicyBinding({
            ...binding,
            updatedAt: now(),
          })
          nextResourceId = binding.bindingId
          break
        }
      }
      setState(nextState)
      setSelectedResourceId(nextResourceId)
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('应用编排资源已保存')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setResourcePending(false)
    }
  })

  const handleDeleteResource = useLockFn(async () => {
    if (selectedResourceId === newResourceValue) {
      return
    }

    setResourcePending(true)
    try {
      let nextState: AppRuntimeStateDocument
      switch (resourceKind) {
        case 'apps':
          nextState = await deleteAppRegistryEntry(selectedResourceId)
          if (selectedAppId === selectedResourceId) {
            setSelectedAppId(nextState.apps[0]?.appId ?? '')
          }
          break
        case 'nodePools':
          nextState = await deleteNodePool(selectedResourceId)
          break
        case 'dnsProfiles':
          nextState = await deleteDnsProfile(selectedResourceId)
          break
        case 'securityProfiles':
          nextState = await deleteSecurityProfile(selectedResourceId)
          break
        case 'policyBindings':
          nextState = await deleteAppPolicyBinding(selectedResourceId)
          break
      }
      setState(nextState)
      setSelectedResourceId(newResourceValue)
      setPlan(null)
      setProjection(null)
      setDiagnostics(null)
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('应用编排资源已删除')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setResourcePending(false)
    }
  })

  const handleExportConfig = () => {
    setBulkJson(
      formatJson({
        apps: state.apps,
        nodePools: state.nodePools,
        dnsProfiles: state.dnsProfiles,
        securityProfiles: state.securityProfiles,
        policyBindings: state.policyBindings,
      }),
    )
  }

  const handleLoadDemoSeed = useLockFn(async () => {
    setResourcePending(true)
    try {
      const seed = await buildAppRuntimeDemoSeed()
      setBulkJson(
        formatJson({
          apps: seed.apps,
          nodePools: seed.nodePools,
          dnsProfiles: seed.dnsProfiles,
          securityProfiles: seed.securityProfiles,
          policyBindings: seed.policyBindings,
        }),
      )
      showNotice.success('Demo seed 已加载到批量导入 JSON')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setResourcePending(false)
    }
  })

  const handleImportConfig = useLockFn(async () => {
    setResourcePending(true)
    try {
      const document =
        parseJsonObject<Partial<AppRuntimeStateDocument>>(bulkJson)
      let nextState = state
      for (const app of document.apps ?? []) {
        nextState = await upsertAppRegistryEntry({ ...app, updatedAt: now() })
      }
      for (const nodePool of document.nodePools ?? []) {
        nextState = await upsertNodePool({ ...nodePool, updatedAt: now() })
      }
      for (const dnsProfile of document.dnsProfiles ?? []) {
        nextState = await upsertDnsProfile({ ...dnsProfile, updatedAt: now() })
      }
      for (const securityProfile of document.securityProfiles ?? []) {
        nextState = await upsertSecurityProfile({
          ...securityProfile,
          updatedAt: now(),
        })
      }
      for (const binding of document.policyBindings ?? []) {
        nextState = await upsertAppPolicyBinding({
          ...binding,
          updatedAt: now(),
        })
      }
      setState(nextState)
      setSelectedAppId((current) => current || nextState.apps[0]?.appId || '')
      setDnsProbeReport(null)
      setProjectionArtifact(null)
      showNotice.success('应用编排配置已导入')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setResourcePending(false)
    }
  })

  useEffect(() => {
    void loadState()
  }, [loadState])

  useEffect(() => {
    const firstMatcher = selectedApp?.processMatchers[0]
    setAppDraft({
      name: selectedApp?.name ?? '',
      executablePath: selectedApp?.executablePath ?? '',
      bundleId: selectedApp?.bundleId ?? '',
      workingDirectory: selectedApp?.workingDirectory ?? '',
      matcherKind: firstMatcher?.kind ?? 'process_name',
      matcherPattern: firstMatcher?.pattern ?? '',
      tags: selectedApp?.tags.join(', ') ?? '',
    })
  }, [selectedApp])

  useEffect(() => {
    const firstCandidate = selectedNodePool?.candidateNodes[0]
    setNodePoolDraft({
      poolId:
        selectedNodePool?.poolId ??
        (selectedAppId ? `pool-${selectedAppId}` : ''),
      name: selectedNodePool?.name ?? '',
      region: selectedNodePool?.region ?? '',
      protocols: selectedNodePool?.protocols.join(', ') ?? '',
      purpose: selectedNodePool?.purpose ?? '',
      costTier: selectedNodePool?.costTier ?? '',
      candidateNodeName: firstCandidate?.nodeName ?? '',
      candidateProxyGroup: firstCandidate?.proxyGroup ?? '',
      candidateTags: firstCandidate?.tags.join(', ') ?? '',
      tags: selectedNodePool?.tags.join(', ') ?? '',
    })
  }, [selectedAppId, selectedNodePool])

  useEffect(() => {
    setDnsProfileDraft({
      profileId:
        selectedDnsProfile?.profileId ??
        (selectedAppId ? `dns-${selectedAppId}` : ''),
      name: selectedDnsProfile?.name ?? '',
      testDomain: selectedDnsProfile?.testDomain ?? '',
      configYaml: selectedDnsProfile?.configYaml ?? 'nameserver:\n  - 1.1.1.1',
      tags: selectedDnsProfile?.tags.join(', ') ?? '',
    })
  }, [selectedAppId, selectedDnsProfile])

  useEffect(() => {
    setSecurityProfileDraft({
      profileId:
        selectedSecurityProfile?.profileId ??
        (selectedAppId ? `security-${selectedAppId}` : ''),
      name: selectedSecurityProfile?.name ?? '',
      requireNodePool:
        selectedSecurityProfile?.controls.requireNodePool === false
          ? 'false'
          : 'true',
      requireDnsProfile:
        selectedSecurityProfile?.controls.requireDnsProfile === true
          ? 'true'
          : 'false',
      minRuntimeSupportedNameservers:
        selectedSecurityProfile?.controls.minRuntimeSupportedNameservers?.toString() ??
        '',
      allowedRoutingIntents:
        selectedSecurityProfile?.controls.allowedRoutingIntents.join(', ') ??
        'proxy, fallback',
      tags: selectedSecurityProfile?.tags.join(', ') ?? '',
    })
  }, [selectedAppId, selectedSecurityProfile])

  useEffect(() => {
    setBindingDraft({
      nodePoolId: selectedBinding?.nodePoolId ?? '',
      dnsProfileId: selectedBinding?.dnsProfileId ?? '',
      securityProfileId: selectedBinding?.securityProfileId ?? '',
      routingIntent: selectedBinding?.routingIntent ?? 'proxy',
      enabled: selectedBinding?.enabled === false ? 'false' : 'true',
    })
  }, [selectedBinding])

  useEffect(() => {
    const resource =
      selectedResourceId === newResourceValue
        ? templateFor(resourceKind, selectedAppId)
        : resources.find(
            (item) => resourceIdFor(resourceKind, item) === selectedResourceId,
          ) || templateFor(resourceKind, selectedAppId)
    setResourceJson(formatJson(resource))
  }, [resourceKind, resources, selectedAppId, selectedResourceId])

  const runtimeApplyDecisionAllowsCandidate =
    runtimeApplyBoundaryDecisionReport?.runtimeApplyCandidateAllowed === true &&
    runtimeApplyBoundaryDecisionReport.decisionRecord.decision ===
      'allowRuntimeCandidate' &&
    runtimeApplyBoundaryDecisionReport.decisionRecord.decisionAccepted === true &&
    runtimeApplyBoundaryDecisionReport.decisionRecord.artifactId ===
      projectionArtifact?.artifactId &&
    runtimeApplyBoundaryDecisionReport.decisionRecord.checksum ===
      projectionArtifact?.checksum

  return (
    <Card>
      <div className="space-y-4 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2 text-sm font-semibold">
              <Route className="h-4 w-4" />
              应用级代理编排（planning-only）
            </div>
            <div className="mt-1 text-xs text-muted-foreground">
              读取 Rust AppRuntimeStateDocument，生成计划、Mihomo projection
              与诊断摘要；不会启动应用或修改 Mihomo runtime。
            </div>
          </div>
          <div className="flex flex-wrap gap-2">
            <Button
              size="small"
              variant="outlined"
              onClick={() => void acceptDnsHandoff()}
              disabled={dnsHandoffPending}
            >
              {dnsHandoffPending ? '接收中...' : '接收 DNS handoff'}
            </Button>
            <Button
              size="small"
              variant="outlined"
              onClick={() => void completeControlPlane()}
              disabled={!selectedAppId || controlPlaneCompletionPending}
            >
              {controlPlaneCompletionPending
                ? '完成中...'
                : '完成 control-plane'}
            </Button>
            <Button
              size="small"
              variant="outlined"
              onClick={() => void completeStagedActivationLifecycle()}
              disabled={!selectedAppId || stagedActivationLifecyclePending}
            >
              {stagedActivationLifecyclePending
                ? '激活中...'
                : '完成 staged lifecycle'}
            </Button>
            <Button
              size="small"
              variant="outlined"
              onClick={() => void closeoutStagedActivation()}
              disabled={!selectedAppId || stagedActivationCloseoutPending}
            >
              {stagedActivationCloseoutPending
                ? '收口中...'
                : '收口 runtime boundary'}
            </Button>
            <Button
              size="small"
              variant="outlined"
              startIcon={<RefreshCw className="h-4 w-4" />}
              onClick={() => void loadState()}
              disabled={loading}
            >
              {loading ? '刷新中...' : '刷新状态'}
            </Button>
          </div>
        </div>

        <AppRuntimeOverviewPanel
          rows={overviewRows}
          filteredRows={filteredOverviewRows}
          filter={overviewFilter}
          selectedAppId={selectedAppId}
          onFilterChange={setOverviewFilter}
          onSelectApp={selectAppForDiagnostics}
        />

        {selectedApp ? (
          <AppRuntimeAggregateDiagnosticsPanel
            items={aggregateDiagnostics}
            actions={aggregateDiagnosticActions}
            dnsWarnings={dnsProbeReport?.warnings ?? []}
            onActionClick={handleAggregateDiagnosticAction}
          />
        ) : null}

        {dnsHandoffReport ? (
          <div className="space-y-2 rounded-lg border border-border p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="text-sm font-semibold">DNS handoff intake</div>
                <div className="mt-1 text-xs text-muted-foreground">
                  接收 DNS expanded control-plane completion manifest，并保持
                  phase8Allowed=false。
                </div>
              </div>
              <Chip
                size="small"
                color={statusColor(dnsHandoffReport.status)}
                label={dnsHandoffReport.status}
              />
            </div>
            <div className="grid gap-2 text-xs lg:grid-cols-2">
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Reason</div>
                <div className="mt-1 text-muted-foreground">
                  {dnsHandoffReport.reason}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Record</div>
                <div className="mt-1 truncate text-muted-foreground">
                  {dnsHandoffReport.handoffRecordPath ?? '未持久化'}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Next step</div>
                <div className="mt-1 text-muted-foreground">
                  {dnsHandoffReport.nextAppRuntimeStep}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Boundary</div>
                <div className="mt-1 text-muted-foreground">
                  accepts={String(dnsHandoffReport.appRuntimeAcceptsHandoff)} ·
                  phase8={String(dnsHandoffReport.phase8Allowed)} · mutates=
                  {String(dnsHandoffReport.mutatesRuntime)}
                </div>
              </div>
            </div>
            <div className="flex flex-wrap gap-2">
              <Button
                size="small"
                onClick={() =>
                  void decideRuntimeApplyBoundary(
                    'allowRuntimeCandidate',
                    'operator explicitly allows the runtime candidate after staged closeout',
                  )
                }
                disabled={
                  !selectedAppId ||
                  runtimeApplyBoundaryDecisionPending ||
                  !stagedActivationCloseoutReport?.closeoutComplete
                }
              >
                {runtimeApplyBoundaryDecisionPending
                  ? '记录中...'
                  : '允许 runtime candidate'}
              </Button>
              <Button
                size="small"
                variant="outlined"
                onClick={() =>
                  void decideRuntimeApplyBoundary(
                    'deferRuntimeApply',
                    'operator keeps holding at the runtime-apply boundary',
                  )
                }
                disabled={!selectedAppId || runtimeApplyBoundaryDecisionPending}
              >
                延后 runtime apply
              </Button>
              <Button
                size="small"
                variant="outlined"
                onClick={() =>
                  void decideRuntimeApplyBoundary(
                    'recommendRollback',
                    'operator recommends explicit staged rollback before runtime apply',
                  )
                }
                disabled={!selectedAppId || runtimeApplyBoundaryDecisionPending}
              >
                建议 staged rollback
              </Button>
            </div>
          </div>
        ) : null}

        {runtimeApplyBoundaryDecisionReport ? (
          <div className="space-y-2 rounded-lg border border-border p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="text-sm font-semibold">
                  Runtime-apply boundary decision
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  记录显式 allow/defer/rollback 决策；不 apply runtime、不
                  reload Mihomo。
                </div>
              </div>
              <Chip
                size="small"
                color={statusColor(runtimeApplyBoundaryDecisionReport.status)}
                label={runtimeApplyBoundaryDecisionReport.status}
              />
            </div>
            <div className="grid gap-2 text-xs lg:grid-cols-2">
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Decision record</div>
                <div className="mt-1 truncate text-muted-foreground">
                  {runtimeApplyBoundaryDecisionReport.decisionRecordPath ??
                    '未持久化'}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Decision</div>
                <div className="mt-1 text-muted-foreground">
                  {runtimeApplyBoundaryDecisionReport.decisionRecord.decision} ·
                  allowed=
                  {String(
                    runtimeApplyBoundaryDecisionReport.runtimeApplyCandidateAllowed,
                  )}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Next step</div>
                <div className="mt-1 text-muted-foreground">
                  {runtimeApplyBoundaryDecisionReport.nextAppRuntimeStep}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Boundary flags</div>
                <div className="mt-1 text-muted-foreground">
                  phase8=
                  {String(runtimeApplyBoundaryDecisionReport.phase8Allowed)} ·
                  reload=
                  {String(runtimeApplyBoundaryDecisionReport.reloadMihomo)}·
                  mutates=
                  {String(runtimeApplyBoundaryDecisionReport.mutatesRuntime)}
                </div>
              </div>
            </div>
          </div>
        ) : null}

        {controlPlaneCompletionReport ? (
          <div className="space-y-2 rounded-lg border border-border p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="text-sm font-semibold">
                  App runtime control-plane completion
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  合并 DNS handoff、projection artifact 与 staged activation
                  preflight；不会 runtime apply。
                </div>
              </div>
              <Chip
                size="small"
                color={statusColor(controlPlaneCompletionReport.status)}
                label={controlPlaneCompletionReport.status}
              />
            </div>
            <div className="grid gap-2 text-xs lg:grid-cols-2">
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Artifact</div>
                <div className="mt-1 truncate text-muted-foreground">
                  {controlPlaneCompletionReport.projectionArtifactPath ??
                    '未持久化'}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Preflight</div>
                <div className="mt-1 text-muted-foreground">
                  {controlPlaneCompletionReport.activationPreflight.status} ·{' '}
                  {controlPlaneCompletionReport.activationPreflight.reason}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Next step</div>
                <div className="mt-1 text-muted-foreground">
                  {controlPlaneCompletionReport.nextAppRuntimeStep}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Boundary</div>
                <div className="mt-1 text-muted-foreground">
                  staged=
                  {String(
                    controlPlaneCompletionReport.readyForStagedActivation,
                  )}{' '}
                  · runtimeApply=
                  {String(controlPlaneCompletionReport.runtimeApplyAllowed)} ·
                  phase8={String(controlPlaneCompletionReport.phase8Allowed)}
                </div>
              </div>
            </div>
          </div>
        ) : null}

        {stagedActivationLifecycleReport ? (
          <div className="space-y-2 rounded-lg border border-border p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="text-sm font-semibold">
                  App runtime staged activation lifecycle
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  合并 control-plane completion 与 staged marker activation；
                  runtime apply 仍保持关闭。
                </div>
              </div>
              <Chip
                size="small"
                color={statusColor(stagedActivationLifecycleReport.status)}
                label={stagedActivationLifecycleReport.status}
              />
            </div>
            <div className="grid gap-2 text-xs lg:grid-cols-2">
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Marker</div>
                <div className="mt-1 truncate text-muted-foreground">
                  {stagedActivationLifecycleReport.activeProjection
                    ?.artifactId ?? '未激活'}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Rollback boundary</div>
                <div className="mt-1 text-muted-foreground">
                  available=
                  {String(
                    stagedActivationLifecycleReport.rollbackBoundaryAvailable,
                  )}{' '}
                  · {stagedActivationLifecycleReport.rollbackStrategy ?? 'none'}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Next step</div>
                <div className="mt-1 text-muted-foreground">
                  {stagedActivationLifecycleReport.nextAppRuntimeStep}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Boundary</div>
                <div className="mt-1 text-muted-foreground">
                  marker=
                  {String(stagedActivationLifecycleReport.markerActivated)}·
                  runtimeApply=
                  {String(stagedActivationLifecycleReport.runtimeApplyAllowed)}{' '}
                  · reload=
                  {String(stagedActivationLifecycleReport.reloadMihomo)}
                </div>
              </div>
            </div>
          </div>
        ) : null}

        {stagedActivationCloseoutReport ? (
          <div className="space-y-2 rounded-lg border border-border p-3">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div>
                <div className="text-sm font-semibold">
                  App runtime runtime-apply boundary closeout
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  收口 staged activation，并持久化 runtime-apply boundary
                  manifest；仍不自动应用 runtime。
                </div>
              </div>
              <Chip
                size="small"
                color={statusColor(stagedActivationCloseoutReport.status)}
                label={stagedActivationCloseoutReport.status}
              />
            </div>
            <div className="grid gap-2 text-xs lg:grid-cols-2">
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Boundary manifest</div>
                <div className="mt-1 truncate text-muted-foreground">
                  {stagedActivationCloseoutReport.boundaryManifestPath ??
                    '未持久化'}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Closeout</div>
                <div className="mt-1 text-muted-foreground">
                  complete=
                  {String(stagedActivationCloseoutReport.closeoutComplete)} ·
                  persisted=
                  {String(
                    stagedActivationCloseoutReport.boundaryManifestPersisted,
                  )}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Next step</div>
                <div className="mt-1 text-muted-foreground">
                  {stagedActivationCloseoutReport.nextAppRuntimeStep}
                </div>
              </div>
              <div className="rounded-md bg-muted/40 px-3 py-2">
                <div className="font-medium">Boundary flags</div>
                <div className="mt-1 text-muted-foreground">
                  runtimeApply=
                  {String(stagedActivationCloseoutReport.runtimeApplyAllowed)} ·
                  phase8={String(stagedActivationCloseoutReport.phase8Allowed)}{' '}
                  · reload={String(stagedActivationCloseoutReport.reloadMihomo)}
                </div>
              </div>
            </div>
          </div>
        ) : null}

        <AppRuntimeSecurityProfileForm
          selectedApp={selectedApp}
          selectedBinding={selectedBinding}
          draft={securityProfileDraft}
          pending={resourcePending}
          setDraft={setSecurityProfileDraft}
          onSave={() => void handleSaveSecurityProfileDraft()}
        />

        <AppRuntimeDnsProfileForm
          selectedApp={selectedApp}
          selectedBinding={selectedBinding}
          draft={dnsProfileDraft}
          pending={resourcePending}
          setDraft={setDnsProfileDraft}
          onSave={() => void handleSaveDnsProfileDraft()}
        />

        <AppRuntimeNodePoolForm
          selectedApp={selectedApp}
          selectedBinding={selectedBinding}
          draft={nodePoolDraft}
          pending={resourcePending}
          setDraft={setNodePoolDraft}
          onSave={() => void handleSaveNodePoolDraft()}
        />

        <AppRuntimeAppRegistryForm
          selectedApp={selectedApp}
          draft={appDraft}
          pending={resourcePending}
          setDraft={setAppDraft}
          onSave={() => void handleSaveAppDraft()}
        />

        <AppRuntimePolicyBindingForm
          selectedApp={selectedApp}
          selectedAppId={selectedAppId}
          selectedBinding={selectedBinding}
          draft={bindingDraft}
          pending={resourcePending}
          nodePoolOptions={optionalNodePoolOptions}
          dnsProfileOptions={optionalDnsProfileOptions}
          securityProfileOptions={optionalSecurityProfileOptions}
          setDraft={setBindingDraft}
          onSave={() => void handleSaveBindingDraft()}
        />

        <AppRuntimeResourceManagerPanel
          state={state}
          resourceKind={resourceKind}
          selectedResourceId={selectedResourceId}
          resourceOptions={resourceOptions}
          resourceJson={resourceJson}
          bulkJson={bulkJson}
          pending={resourcePending}
          onResourceKindChange={setResourceKind}
          onSelectedResourceIdChange={setSelectedResourceId}
          onResourceJsonChange={setResourceJson}
          onBulkJsonChange={setBulkJson}
          onSaveResource={() => void handleSaveResource()}
          onDeleteResource={() => void handleDeleteResource()}
          onExportConfig={handleExportConfig}
          onLoadDemoSeed={() => void handleLoadDemoSeed()}
          onImportConfig={() => void handleImportConfig()}
        />

        {state.apps.length === 0 ? (
          <div className="rounded-lg border border-border px-3 py-4 text-sm text-muted-foreground">
            当前还没有应用注册项。先通过后续管理入口写入 app registry / node
            pool / policy binding 后，这里会展示可诊断的规划结果。
          </div>
        ) : (
          <div className="grid gap-3 lg:grid-cols-[minmax(0,1fr)_auto]">
            <Select
              fullWidth
              size="small"
              label="选择应用"
              value={selectedAppId}
              options={appOptions}
              onChange={(value: string | number) => {
                selectAppForDiagnostics(String(value))
              }}
            />
            <div className="flex flex-wrap items-end gap-2">
              <Button
                size="small"
                variant="outlined"
                startIcon={<Activity className="h-4 w-4" />}
                onClick={() => void runPlanningDiagnostics()}
                disabled={!selectedAppId || planning}
              >
                {planning ? '诊断中...' : '运行规划诊断'}
              </Button>
              <Button
                size="small"
                startIcon={<Activity className="h-4 w-4" />}
                onClick={() => void runSelectedAppReadiness()}
                disabled={!selectedAppId || planning || dnsProbePending}
              >
                {planning || dnsProbePending ? '检查中...' : '运行 readiness'}
              </Button>
              <Button
                size="small"
                variant="outlined"
                startIcon={<Route className="h-4 w-4" />}
                onClick={() => void buildProjectionArtifact()}
                disabled={!selectedAppId || artifactPending}
              >
                {artifactPending ? '生成中...' : '生成 artifact'}
              </Button>
            </div>
          </div>
        )}

        {selectedApp ? (
          <div className="rounded-lg border border-border px-3 py-2 text-xs text-muted-foreground">
            {selectedApp.processMatchers.length > 0
              ? selectedApp.processMatchers
                  .map((matcher) => `${matcher.kind}:${matcher.pattern}`)
                  .join(' / ')
              : '该应用尚未配置 process matcher。'}
          </div>
        ) : null}

        {selectedApp ? (
          <div className="space-y-3 rounded-lg border border-border p-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="text-sm font-semibold">
                  绑定 DNS profile 受控探测
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  对当前 app policy 绑定的 DNS profile 运行 Rust controlled
                  probe，只探测 runtime-supported nameserver，不切默认 DNS。
                </div>
              </div>
              <Button
                size="small"
                variant="outlined"
                startIcon={<Activity className="h-4 w-4" />}
                onClick={() => void handleProbeSelectedDnsProfile()}
                disabled={!selectedDnsProfile || dnsProbePending}
              >
                {dnsProbePending ? '探测中...' : '探测绑定 DNS'}
              </Button>
            </div>

            {selectedDnsProfile ? (
              <div className="flex flex-wrap gap-2">
                <Chip
                  size="small"
                  label={`Profile: ${selectedDnsProfile.name}`}
                />
                <Chip
                  size="small"
                  label={`Domain: ${selectedDnsProfile.testDomain || 'example.com'}`}
                />
              </div>
            ) : (
              <div className="rounded-md bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                当前应用没有可探测的 DNS profile 绑定。
              </div>
            )}

            {dnsProbeReport ? (
              <div className="space-y-2">
                <div className="grid gap-2 text-xs sm:grid-cols-4">
                  <div>Total: {dnsProbeReport.summary.totalTargets}</div>
                  <div>
                    Supported: {dnsProbeReport.summary.runtimeSupportedTargets}
                  </div>
                  <div>Healthy: {dnsProbeReport.summary.healthyTargets}</div>
                  <div>Failed: {dnsProbeReport.summary.failedTargets}</div>
                </div>
                <div className="flex flex-wrap gap-2">
                  {dnsProbeReport.targets.map((target) => (
                    <Chip
                      key={`${target.server}-${target.protocol}`}
                      size="small"
                      color={
                        target.healthy
                          ? 'success'
                          : target.runtimeSupported
                            ? 'error'
                            : 'warning'
                      }
                      label={`${target.server} · ${target.providerLabel ?? target.protocol}`}
                      title={target.message}
                    />
                  ))}
                </div>
                {dnsProbeReport.warnings.length > 0 ? (
                  <div className="rounded-md bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                    {dnsProbeReport.warnings.join('；')}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        ) : null}

        {selectedApp ? (
          <AppRuntimeSessionPanel
            sessions={appSessions}
            selectedSession={selectedSession}
            evaluation={evaluation}
            leakReport={leakReport}
            pending={sessionPending}
            onSelectSession={(sessionId) => {
              setSelectedSessionId(sessionId)
              setEvaluation(null)
              setLeakReport(null)
            }}
            onStartSession={() => void handleStartSession()}
            onRecordObservation={() => void handleRecordObservation()}
            onEvaluateSession={() => void handleEvaluateSession()}
            onVerifySessionLeak={() => void handleVerifySessionLeak()}
            onFinishSession={(status) => void handleFinishSession(status)}
          />
        ) : null}

        <AppRuntimePlanningResultPanel
          diagnostics={diagnostics}
          plan={plan}
          projection={projection}
          projectionArtifact={projectionArtifact}
          activationPreflight={activationPreflight}
          activationPreflightPending={activationPreflightPending}
          activeProjection={state.activeProjection ?? null}
          latestRuntimeApplyAudit={latestRuntimeApplyAudit}
          runtimeVerification={runtimeVerification}
          activateMarkerPending={activateMarkerPending}
          runtimeApplyAllowed={runtimeApplyDecisionAllowsCandidate}
          runtimeApplyPending={runtimeApplyPending}
          runtimeVerificationPending={runtimeVerificationPending}
          activationRollbackPending={activationRollbackPending}
          onPreflightActivation={() => void preflightProjectionActivation()}
          onMarkActive={() => void markProjectionArtifactActive()}
          onApplyRuntime={() => void applyProjectionArtifactToRuntime()}
          onVerifyRuntime={() => void verifyProjectionRuntimeApply()}
          onRollbackActivation={() => void rollbackProjectionActivation()}
        />
      </div>
    </Card>
  )
}
