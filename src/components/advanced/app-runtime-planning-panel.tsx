import { useLockFn } from 'ahooks'
import {
  Activity,
  Boxes,
  ClipboardList,
  Download,
  RefreshCw,
  Route,
  Save,
  Trash2,
  Upload,
} from 'lucide-react'
import { type ChangeEvent, useEffect, useMemo, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import { Select } from '@/components/tailwind/Select'
import { TextField } from '@/components/tailwind/TextField'
import {
  deleteAppPolicyBinding,
  deleteAppRegistryEntry,
  deleteDnsProfile,
  deleteNodePool,
  deleteSecurityProfile,
  diagnoseAppRuntime,
  evaluateAppRuntimeSession,
  finishAppRuntimeSession,
  getAppRuntimeState,
  recordAppRuntimeSessionObservation,
  projectAppRuntimePlanToMihomo,
  startAppRuntimeSession,
  upsertAppPolicyBinding,
  upsertAppRegistryEntry,
  upsertDnsProfile,
  upsertNodePool,
  upsertSecurityProfile,
  verifyAppRuntimeSessionLeak,
  type AppPolicyBinding,
  type AppProcessMatcherKind,
  type AppRegistryEntry,
  type AppRoutingIntent,
  type AppRuntimeDiagnosticsReport,
  type AppRuntimeMihomoProjection,
  type AppRuntimePlan,
  type AppRuntimeSessionEvaluationReport,
  type AppRuntimeSessionLeakReport,
  type AppRuntimeSessionRecord,
  type AppRuntimeSessionStatus,
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

const emptyState: AppRuntimeStateDocument = {
  apps: [],
  nodePools: [],
  dnsProfiles: [],
  securityProfiles: [],
  policyBindings: [],
  sessions: [],
}

function stateCountLabel(label: string, count: number) {
  return `${label}: ${count}`
}

function selectAppLabel(app: AppRegistryEntry) {
  return `${app.name} (${app.appId})`
}

function statusColor(
  status: string,
): 'default' | 'success' | 'warning' | 'error' {
  switch (status) {
    case 'ready':
    case 'healthy':
    case 'planned':
    case 'completed':
    case 'passed':
    case 'pass':
    case 'appMatched':
      return 'success'
    case 'degraded':
    case 'warning':
    case 'warn':
    case 'skipped':
    case 'notApplicable':
    case 'unattributed':
      return 'warning'
    case 'blocked':
    case 'rejected':
    case 'failed':
    case 'fail':
    case 'appMismatch':
      return 'error'
    default:
      return 'default'
  }
}

function sortSessions(sessions: AppRuntimeSessionRecord[]) {
  return [...sessions].sort(
    (left, right) =>
      right.startedAt - left.startedAt ||
      right.sessionId.localeCompare(left.sessionId),
  )
}

function upsertSession(
  sessions: AppRuntimeSessionRecord[],
  nextSession: AppRuntimeSessionRecord,
) {
  const nextSessions = sessions.filter(
    (session) => session.sessionId !== nextSession.sessionId,
  )
  nextSessions.push(nextSession)
  return sortSessions(nextSessions)
}

type FinishableSessionStatus = Exclude<AppRuntimeSessionStatus, 'planned'>

type RuntimeResourceKind =
  | 'apps'
  | 'nodePools'
  | 'dnsProfiles'
  | 'securityProfiles'
  | 'policyBindings'

const resourceKindOptions = [
  { value: 'apps', label: 'Apps' },
  { value: 'nodePools', label: 'Node pools' },
  { value: 'dnsProfiles', label: 'DNS profiles' },
  { value: 'securityProfiles', label: 'Security profiles' },
  { value: 'policyBindings', label: 'Policy bindings' },
]

const routingIntentOptions = [
  { value: 'direct', label: 'direct' },
  { value: 'proxy', label: 'proxy' },
  { value: 'reject', label: 'reject' },
  { value: 'auto', label: 'auto' },
  { value: 'fallback', label: 'fallback' },
]

const enabledOptions = [
  { value: 'true', label: 'enabled' },
  { value: 'false', label: 'disabled' },
]

const processMatcherKindOptions = [
  { value: 'process_name', label: 'process_name' },
  { value: 'process_path', label: 'process_path' },
  { value: 'process_name_regex', label: 'process_name_regex' },
  { value: 'process_path_regex', label: 'process_path_regex' },
  { value: 'bundle_id', label: 'bundle_id' },
]

const newResourceValue = '__new__'

function now() {
  return Date.now()
}

function createAppTemplate(): AppRegistryEntry {
  return {
    appId: 'new-app',
    name: 'New App',
    launchArgs: [],
    env: [],
    processMatchers: [{ kind: 'process_name', pattern: 'new-app.exe' }],
    platformMetadata: {},
    tags: [],
    updatedAt: now(),
  }
}

function createNodePoolTemplate(): NodePool {
  return {
    poolId: 'new-pool',
    name: 'New Node Pool',
    tags: [],
    protocols: [],
    healthConstraints: {},
    candidateNodes: [{ nodeName: 'Proxy', tags: [] }],
    updatedAt: now(),
  }
}

function createDnsProfileTemplate(): DnsProfile {
  return {
    profileId: 'new-dns-profile',
    name: 'New DNS Profile',
    configYaml: 'nameserver:\n  - 1.1.1.1',
    testDomain: 'example.com',
    tags: [],
    updatedAt: now(),
  }
}

function createSecurityProfileTemplate(): SecurityProfile {
  return {
    profileId: 'new-security-profile',
    name: 'New Security Profile',
    controls: {
      requireNodePool: true,
      requireDnsProfile: false,
      allowedRoutingIntents: ['proxy', 'fallback'],
    },
    tags: [],
    updatedAt: now(),
  }
}

function createPolicyBindingTemplate(appId = ''): AppPolicyBinding {
  return {
    bindingId: 'new-binding',
    appId: appId || 'new-app',
    routingIntent: 'proxy',
    enabled: true,
    updatedAt: now(),
  }
}

function resourceIdFor(
  kind: RuntimeResourceKind,
  resource:
    | AppRegistryEntry
    | NodePool
    | DnsProfile
    | SecurityProfile
    | AppPolicyBinding,
) {
  switch (kind) {
    case 'apps':
      return (resource as AppRegistryEntry).appId
    case 'nodePools':
      return (resource as NodePool).poolId
    case 'dnsProfiles':
      return (resource as DnsProfile).profileId
    case 'securityProfiles':
      return (resource as SecurityProfile).profileId
    case 'policyBindings':
      return (resource as AppPolicyBinding).bindingId
  }
}

function resourceNameFor(
  kind: RuntimeResourceKind,
  resource:
    | AppRegistryEntry
    | NodePool
    | DnsProfile
    | SecurityProfile
    | AppPolicyBinding,
) {
  switch (kind) {
    case 'apps':
      return (resource as AppRegistryEntry).name
    case 'nodePools':
      return (resource as NodePool).name
    case 'dnsProfiles':
      return (resource as DnsProfile).name
    case 'securityProfiles':
      return (resource as SecurityProfile).name
    case 'policyBindings':
      return `${(resource as AppPolicyBinding).appId} → ${(resource as AppPolicyBinding).routingIntent}`
  }
}

function collectionFor(
  state: AppRuntimeStateDocument,
  kind: RuntimeResourceKind,
) {
  switch (kind) {
    case 'apps':
      return state.apps
    case 'nodePools':
      return state.nodePools
    case 'dnsProfiles':
      return state.dnsProfiles
    case 'securityProfiles':
      return state.securityProfiles
    case 'policyBindings':
      return state.policyBindings
  }
}

function templateFor(kind: RuntimeResourceKind, appId = '') {
  switch (kind) {
    case 'apps':
      return createAppTemplate()
    case 'nodePools':
      return createNodePoolTemplate()
    case 'dnsProfiles':
      return createDnsProfileTemplate()
    case 'securityProfiles':
      return createSecurityProfileTemplate()
    case 'policyBindings':
      return createPolicyBindingTemplate(appId)
  }
}

function parseJsonObject<T extends object>(raw: string): T {
  const parsed: unknown = JSON.parse(raw)
  if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error('JSON 必须是对象')
  }
  return parsed as T
}

function formatJson(value: unknown) {
  return JSON.stringify(value, null, 2)
}

function formatTime(timestamp?: number) {
  return timestamp ? new Date(timestamp).toLocaleString() : '-'
}

function formatBytes(bytes: number) {
  if (bytes < 1024) {
    return `${bytes} B`
  }
  if (bytes < 1024 * 1024) {
    return `${(bytes / 1024).toFixed(1)} KiB`
  }
  return `${(bytes / 1024 / 1024).toFixed(1)} MiB`
}

export function AppRuntimePlanningPanel() {
  const [state, setState] = useState<AppRuntimeStateDocument>(emptyState)
  const [selectedAppId, setSelectedAppId] = useState('')
  const [loading, setLoading] = useState(false)
  const [planning, setPlanning] = useState(false)
  const [sessionPending, setSessionPending] = useState(false)
  const [dnsProbePending, setDnsProbePending] = useState(false)
  const [plan, setPlan] = useState<AppRuntimePlan | null>(null)
  const [projection, setProjection] =
    useState<AppRuntimeMihomoProjection | null>(null)
  const [diagnostics, setDiagnostics] =
    useState<AppRuntimeDiagnosticsReport | null>(null)
  const [selectedSessionId, setSelectedSessionId] = useState('')
  const [evaluation, setEvaluation] =
    useState<AppRuntimeSessionEvaluationReport | null>(null)
  const [leakReport, setLeakReport] =
    useState<AppRuntimeSessionLeakReport | null>(null)
  const [dnsProbeReport, setDnsProbeReport] =
    useState<DnsResolverRuntimeProbeReport | null>(null)
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
    ]
  }, [
    diagnostics,
    dnsProbeReport,
    projection,
    selectedApp,
    selectedDnsProfile,
    selectedOverviewRow,
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

        {overviewRows.length > 0 ? (
          <div className="space-y-3 rounded-lg border border-border p-3">
            <div>
              <div className="text-sm font-semibold">应用编排概览</div>
              <div className="mt-1 text-xs text-muted-foreground">
                汇总 Rust state 中的 app → policy binding → node / DNS /
                security 关系，便于快速定位下一步诊断对象。
              </div>
            </div>

            <div className="grid gap-2 lg:grid-cols-[minmax(0,1fr)_auto]">
              <TextField
                fullWidth
                size="small"
                label="过滤应用 / 绑定 / profile / issue"
                value={overviewFilter}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => setOverviewFilter(event.target.value)}
              />
              <div className="flex flex-wrap items-end gap-2">
                <Chip
                  size="small"
                  color={
                    overviewRows.some((row) => row.issues.length > 0)
                      ? 'warning'
                      : 'success'
                  }
                  label={`Issues: ${
                    overviewRows.filter((row) => row.issues.length > 0).length
                  }`}
                />
                <Chip
                  size="small"
                  label={`Showing: ${filteredOverviewRows.length}/${overviewRows.length}`}
                />
              </div>
            </div>

            <div className="grid gap-2">
              {filteredOverviewRows.map((row) => (
                <div
                  key={row.app.appId}
                  className="grid gap-3 rounded-md bg-muted/40 px-3 py-2 text-xs lg:grid-cols-[minmax(0,1.4fr)_minmax(0,2fr)_auto]"
                >
                  <div className="space-y-1">
                    <div className="font-semibold">{row.app.name}</div>
                    <div className="text-muted-foreground">{row.app.appId}</div>
                    <div className="flex flex-wrap gap-1">
                      {row.app.tags.slice(0, 4).map((tag) => (
                        <Chip key={tag} size="small" label={tag} />
                      ))}
                    </div>
                  </div>

                  <div className="grid gap-1 sm:grid-cols-2">
                    <div>
                      <span className="text-muted-foreground">Routing: </span>
                      {row.binding?.routingIntent ?? 'unbound'}
                    </div>
                    <div>
                      <span className="text-muted-foreground">Binding: </span>
                      {row.binding?.enabled === false
                        ? 'disabled'
                        : (row.binding?.bindingId ?? 'missing')}
                    </div>
                    <div>
                      <span className="text-muted-foreground">Node pool: </span>
                      {row.nodePool?.name ?? row.binding?.nodePoolId ?? '-'}
                    </div>
                    <div>
                      <span className="text-muted-foreground">DNS: </span>
                      {row.dnsProfile?.name ?? row.binding?.dnsProfileId ?? '-'}
                    </div>
                    <div>
                      <span className="text-muted-foreground">Security: </span>
                      {row.securityProfile?.name ??
                        row.binding?.securityProfileId ??
                        '-'}
                    </div>
                    <div>
                      <span className="text-muted-foreground">Sessions: </span>
                      {row.sessions.length} total / {row.openSessions} open
                    </div>
                    {row.issues.length > 0 ? (
                      <div className="sm:col-span-2">
                        <span className="text-muted-foreground">Issues: </span>
                        <span className="text-warning">
                          {row.issues.join('；')}
                        </span>
                      </div>
                    ) : (
                      <div className="sm:col-span-2 text-success">
                        State references resolved
                      </div>
                    )}
                  </div>

                  <div className="flex items-center justify-end">
                    <Button
                      size="small"
                      variant={
                        row.app.appId === selectedAppId
                          ? 'contained'
                          : 'outlined'
                      }
                      onClick={() => selectAppForDiagnostics(row.app.appId)}
                    >
                      {row.app.appId === selectedAppId ? '已选择' : '选择'}
                    </Button>
                  </div>
                </div>
              ))}
              {filteredOverviewRows.length === 0 ? (
                <div className="rounded-md bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                  没有匹配当前过滤条件的应用。
                </div>
              ) : null}
            </div>
          </div>
        ) : null}

        {selectedApp ? (
          <div className="space-y-3 rounded-lg border border-border p-3">
            <div>
              <div className="text-sm font-semibold">
                Security profile 快速表单
              </div>
              <div className="mt-1 text-xs text-muted-foreground">
                编辑当前 app 绑定的 security profile 约束；仍只影响 diagnostics
                / planning。
              </div>
            </div>

            <div className="grid gap-3 lg:grid-cols-2">
              <TextField
                fullWidth
                size="small"
                label="Profile ID"
                value={securityProfileDraft.profileId}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setSecurityProfileDraft((draft) => ({
                    ...draft,
                    profileId: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Name"
                value={securityProfileDraft.name}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setSecurityProfileDraft((draft) => ({
                    ...draft,
                    name: event.target.value,
                  }))
                }}
              />
              <Select
                fullWidth
                size="small"
                label="Require node pool"
                value={securityProfileDraft.requireNodePool}
                options={enabledOptions}
                onChange={(value: string | number) => {
                  setSecurityProfileDraft((draft) => ({
                    ...draft,
                    requireNodePool: String(value),
                  }))
                }}
              />
              <Select
                fullWidth
                size="small"
                label="Require DNS profile"
                value={securityProfileDraft.requireDnsProfile}
                options={enabledOptions}
                onChange={(value: string | number) => {
                  setSecurityProfileDraft((draft) => ({
                    ...draft,
                    requireDnsProfile: String(value),
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Min runtime-supported nameservers"
                value={securityProfileDraft.minRuntimeSupportedNameservers}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setSecurityProfileDraft((draft) => ({
                    ...draft,
                    minRuntimeSupportedNameservers: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Allowed routing intents"
                value={securityProfileDraft.allowedRoutingIntents}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setSecurityProfileDraft((draft) => ({
                    ...draft,
                    allowedRoutingIntents: event.target.value,
                  }))
                }}
                helperText="逗号分隔，例如 proxy, fallback。"
              />
              <TextField
                fullWidth
                size="small"
                label="Tags"
                value={securityProfileDraft.tags}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setSecurityProfileDraft((draft) => ({
                    ...draft,
                    tags: event.target.value,
                  }))
                }}
                helperText="逗号分隔。"
              />
              <div className="flex items-end">
                <Button
                  size="small"
                  startIcon={<Save className="h-4 w-4" />}
                  onClick={() => void handleSaveSecurityProfileDraft()}
                  disabled={resourcePending}
                >
                  保存 security profile
                </Button>
              </div>
            </div>

            <div className="text-xs text-muted-foreground">
              当前绑定:{' '}
              {selectedBinding?.securityProfileId || '未绑定 security profile'}
            </div>
          </div>
        ) : null}

        {selectedApp ? (
          <div className="space-y-3 rounded-lg border border-border p-3">
            <div>
              <div className="text-sm font-semibold">DNS profile 快速表单</div>
              <div className="mt-1 text-xs text-muted-foreground">
                编辑当前 app 绑定的 DNS profile；保存后可直接运行绑定 DNS
                controlled probe。
              </div>
            </div>

            <div className="grid gap-3 lg:grid-cols-2">
              <TextField
                fullWidth
                size="small"
                label="Profile ID"
                value={dnsProfileDraft.profileId}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setDnsProfileDraft((draft) => ({
                    ...draft,
                    profileId: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Name"
                value={dnsProfileDraft.name}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setDnsProfileDraft((draft) => ({
                    ...draft,
                    name: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Test domain"
                value={dnsProfileDraft.testDomain}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setDnsProfileDraft((draft) => ({
                    ...draft,
                    testDomain: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Tags"
                value={dnsProfileDraft.tags}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setDnsProfileDraft((draft) => ({
                    ...draft,
                    tags: event.target.value,
                  }))
                }}
                helperText="逗号分隔。"
              />
              <div className="lg:col-span-2">
                <TextField
                  fullWidth
                  multiline
                  rows={6}
                  size="small"
                  label="DNS YAML"
                  value={dnsProfileDraft.configYaml}
                  onChange={(
                    event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                  ) => {
                    setDnsProfileDraft((draft) => ({
                      ...draft,
                      configYaml: event.target.value,
                    }))
                  }}
                  helperText="用于 Rust DnsResolverPlan / controlled probe；不会切默认 DNS runtime。"
                />
              </div>
              <div className="flex items-end">
                <Button
                  size="small"
                  startIcon={<Save className="h-4 w-4" />}
                  onClick={() => void handleSaveDnsProfileDraft()}
                  disabled={resourcePending}
                >
                  保存 DNS profile
                </Button>
              </div>
            </div>

            <div className="text-xs text-muted-foreground">
              当前绑定: {selectedBinding?.dnsProfileId || '未绑定 DNS profile'}
            </div>
          </div>
        ) : null}

        {selectedApp ? (
          <div className="space-y-3 rounded-lg border border-border p-3">
            <div>
              <div className="text-sm font-semibold">Node pool 快速表单</div>
              <div className="mt-1 text-xs text-muted-foreground">
                编辑当前 app 绑定的节点池常用字段；保存后可在 policy binding
                表单中选择该 pool。
              </div>
            </div>

            <div className="grid gap-3 lg:grid-cols-2">
              <TextField
                fullWidth
                size="small"
                label="Pool ID"
                value={nodePoolDraft.poolId}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    poolId: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Name"
                value={nodePoolDraft.name}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    name: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Region"
                value={nodePoolDraft.region}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    region: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Protocols"
                value={nodePoolDraft.protocols}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    protocols: event.target.value,
                  }))
                }}
                helperText="逗号分隔。"
              />
              <TextField
                fullWidth
                size="small"
                label="Purpose"
                value={nodePoolDraft.purpose}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    purpose: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Cost tier"
                value={nodePoolDraft.costTier}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    costTier: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Candidate node"
                value={nodePoolDraft.candidateNodeName}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    candidateNodeName: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Candidate proxy group"
                value={nodePoolDraft.candidateProxyGroup}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    candidateProxyGroup: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Candidate tags"
                value={nodePoolDraft.candidateTags}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    candidateTags: event.target.value,
                  }))
                }}
                helperText="逗号分隔。"
              />
              <TextField
                fullWidth
                size="small"
                label="Pool tags"
                value={nodePoolDraft.tags}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setNodePoolDraft((draft) => ({
                    ...draft,
                    tags: event.target.value,
                  }))
                }}
                helperText="逗号分隔。"
              />
              <div className="flex items-end">
                <Button
                  size="small"
                  startIcon={<Save className="h-4 w-4" />}
                  onClick={() => void handleSaveNodePoolDraft()}
                  disabled={resourcePending}
                >
                  保存 node pool
                </Button>
              </div>
            </div>

            <div className="text-xs text-muted-foreground">
              当前绑定: {selectedBinding?.nodePoolId || '未绑定 node pool'}
            </div>
          </div>
        ) : null}

        {selectedApp ? (
          <div className="space-y-3 rounded-lg border border-border p-3">
            <div>
              <div className="text-sm font-semibold">App registry 快速表单</div>
              <div className="mt-1 text-xs text-muted-foreground">
                常用应用注册字段可直接通过表单保存；高级字段仍可用 JSON editor。
              </div>
            </div>

            <div className="grid gap-3 lg:grid-cols-2">
              <TextField
                fullWidth
                size="small"
                label="Name"
                value={appDraft.name}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setAppDraft((draft) => ({
                    ...draft,
                    name: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Executable path"
                value={appDraft.executablePath}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setAppDraft((draft) => ({
                    ...draft,
                    executablePath: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Bundle ID"
                value={appDraft.bundleId}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setAppDraft((draft) => ({
                    ...draft,
                    bundleId: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Working directory"
                value={appDraft.workingDirectory}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setAppDraft((draft) => ({
                    ...draft,
                    workingDirectory: event.target.value,
                  }))
                }}
              />
              <Select
                fullWidth
                size="small"
                label="Matcher kind"
                value={appDraft.matcherKind}
                options={processMatcherKindOptions}
                onChange={(value: string | number) => {
                  setAppDraft((draft) => ({
                    ...draft,
                    matcherKind: String(value) as AppProcessMatcherKind,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Matcher pattern"
                value={appDraft.matcherPattern}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setAppDraft((draft) => ({
                    ...draft,
                    matcherPattern: event.target.value,
                  }))
                }}
              />
              <TextField
                fullWidth
                size="small"
                label="Tags"
                value={appDraft.tags}
                onChange={(
                  event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
                ) => {
                  setAppDraft((draft) => ({
                    ...draft,
                    tags: event.target.value,
                  }))
                }}
                helperText="逗号分隔。"
              />
              <div className="flex items-end">
                <Button
                  size="small"
                  startIcon={<Save className="h-4 w-4" />}
                  onClick={() => void handleSaveAppDraft()}
                  disabled={resourcePending}
                >
                  保存 app
                </Button>
              </div>
            </div>

            <div className="text-xs text-muted-foreground">
              App ID: {selectedApp.appId}
            </div>
          </div>
        ) : null}

        {selectedApp ? (
          <div className="space-y-3 rounded-lg border border-border p-3">
            <div>
              <div className="text-sm font-semibold">
                Policy binding 快速表单
              </div>
              <div className="mt-1 text-xs text-muted-foreground">
                常用绑定字段可直接通过表单保存；底层仍写入 Rust
                AppRuntimeStateDocument。
              </div>
            </div>

            <div className="grid gap-3 lg:grid-cols-2">
              <Select
                fullWidth
                size="small"
                label="Node pool"
                value={bindingDraft.nodePoolId}
                options={optionalNodePoolOptions}
                onChange={(value: string | number) => {
                  setBindingDraft((draft) => ({
                    ...draft,
                    nodePoolId: String(value),
                  }))
                }}
              />
              <Select
                fullWidth
                size="small"
                label="DNS profile"
                value={bindingDraft.dnsProfileId}
                options={optionalDnsProfileOptions}
                onChange={(value: string | number) => {
                  setBindingDraft((draft) => ({
                    ...draft,
                    dnsProfileId: String(value),
                  }))
                }}
              />
              <Select
                fullWidth
                size="small"
                label="Security profile"
                value={bindingDraft.securityProfileId}
                options={optionalSecurityProfileOptions}
                onChange={(value: string | number) => {
                  setBindingDraft((draft) => ({
                    ...draft,
                    securityProfileId: String(value),
                  }))
                }}
              />
              <Select
                fullWidth
                size="small"
                label="Routing intent"
                value={bindingDraft.routingIntent}
                options={routingIntentOptions}
                onChange={(value: string | number) => {
                  setBindingDraft((draft) => ({
                    ...draft,
                    routingIntent: String(value) as AppRoutingIntent,
                  }))
                }}
              />
              <Select
                fullWidth
                size="small"
                label="Binding status"
                value={bindingDraft.enabled}
                options={enabledOptions}
                onChange={(value: string | number) => {
                  setBindingDraft((draft) => ({
                    ...draft,
                    enabled: String(value),
                  }))
                }}
              />
              <div className="flex items-end">
                <Button
                  size="small"
                  startIcon={<Save className="h-4 w-4" />}
                  onClick={() => void handleSaveBindingDraft()}
                  disabled={resourcePending}
                >
                  保存 binding
                </Button>
              </div>
            </div>

            <div className="text-xs text-muted-foreground">
              Binding ID:{' '}
              {selectedBinding?.bindingId || `binding-${selectedAppId}`}
            </div>
          </div>
        ) : null}

        <div className="flex flex-wrap gap-2">
          <Chip
            size="small"
            label={stateCountLabel('Apps', state.apps.length)}
          />
          <Chip
            size="small"
            label={stateCountLabel('Node pools', state.nodePools.length)}
          />
          <Chip
            size="small"
            label={stateCountLabel('DNS profiles', state.dnsProfiles.length)}
          />
          <Chip
            size="small"
            label={stateCountLabel(
              'Security profiles',
              state.securityProfiles.length,
            )}
          />
          <Chip
            size="small"
            label={stateCountLabel('Bindings', state.policyBindings.length)}
          />
        </div>

        <div className="space-y-3 rounded-lg border border-border p-3">
          <div>
            <div className="text-sm font-semibold">Rust state 管理</div>
            <div className="mt-1 text-xs text-muted-foreground">
              基于现有 app-runtime upsert/delete commands 管理 Rust
              state；保存后仍只生成 planning / projection，不直接修改 Mihomo
              runtime。
            </div>
          </div>

          <div className="grid gap-3 lg:grid-cols-[220px_minmax(0,1fr)]">
            <Select
              fullWidth
              size="small"
              label="资源类型"
              value={resourceKind}
              options={resourceKindOptions}
              onChange={(value: string | number) => {
                setResourceKind(String(value) as RuntimeResourceKind)
                setSelectedResourceId(newResourceValue)
              }}
            />
            <Select
              fullWidth
              size="small"
              label="资源"
              value={selectedResourceId}
              options={resourceOptions}
              onChange={(value: string | number) => {
                setSelectedResourceId(String(value))
              }}
            />
          </div>

          <TextField
            fullWidth
            multiline
            rows={10}
            size="small"
            label="资源 JSON"
            value={resourceJson}
            onChange={(
              event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
            ) => setResourceJson(event.target.value)}
            helperText="字段与 AppRuntimeStateDocument 中对应资源类型保持一致。"
          />

          <div className="flex flex-wrap gap-2">
            <Button
              size="small"
              startIcon={<Save className="h-4 w-4" />}
              onClick={() => void handleSaveResource()}
              disabled={resourcePending}
            >
              保存资源
            </Button>
            <Button
              size="small"
              variant="outlined"
              color="error"
              startIcon={<Trash2 className="h-4 w-4" />}
              onClick={() => void handleDeleteResource()}
              disabled={
                resourcePending || selectedResourceId === newResourceValue
              }
            >
              删除资源
            </Button>
            <Button
              size="small"
              variant="outlined"
              startIcon={<Download className="h-4 w-4" />}
              onClick={handleExportConfig}
              disabled={resourcePending}
            >
              导出配置 JSON
            </Button>
          </div>

          <TextField
            fullWidth
            multiline
            rows={6}
            size="small"
            label="批量导入 JSON"
            value={bulkJson}
            onChange={(
              event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
            ) => setBulkJson(event.target.value)}
            helperText="支持 apps / nodePools / dnsProfiles / securityProfiles / policyBindings，导入为合并 upsert。"
          />

          <Button
            size="small"
            variant="outlined"
            startIcon={<Upload className="h-4 w-4" />}
            onClick={() => void handleImportConfig()}
            disabled={resourcePending || !bulkJson.trim()}
          >
            导入/合并配置
          </Button>
        </div>

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
            <Button
              size="small"
              startIcon={<Activity className="h-4 w-4" />}
              onClick={() => void runPlanningDiagnostics()}
              disabled={!selectedAppId || planning}
            >
              {planning ? '诊断中...' : '运行规划诊断'}
            </Button>
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
            <div>
              <div className="text-sm font-semibold">聚合诊断摘要</div>
              <div className="mt-1 text-xs text-muted-foreground">
                把 overview state issue、planning diagnostics、DNS controlled
                probe 和 runtime boundary 放在同一视图，避免分散查状态。
              </div>
            </div>

            <div className="grid gap-2 lg:grid-cols-2">
              {aggregateDiagnostics.map((item) => (
                <div
                  key={item.key}
                  className="space-y-1 rounded-md bg-muted/40 px-3 py-2 text-xs"
                >
                  <div className="flex flex-wrap items-center justify-between gap-2">
                    <span className="font-medium">{item.label}</span>
                    <Chip
                      size="small"
                      color={statusColor(item.status)}
                      label={item.status}
                    />
                  </div>
                  <div className="text-muted-foreground">{item.detail}</div>
                </div>
              ))}
            </div>

            {dnsProbeReport?.warnings.length ? (
              <div className="rounded-md bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                DNS probe warnings: {dnsProbeReport.warnings.join('；')}
              </div>
            ) : null}
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
          <div className="space-y-3 rounded-lg border border-border p-3">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="flex items-center gap-2 text-sm font-semibold">
                  <ClipboardList className="h-4 w-4" />
                  Session 观测
                </div>
                <div className="mt-1 text-xs text-muted-foreground">
                  记录 app runtime session
                  与连接指标快照，用于后续归因和泄漏维度检查。
                </div>
              </div>
              <Button
                size="small"
                onClick={() => void handleStartSession()}
                disabled={sessionPending}
              >
                {sessionPending ? '处理中...' : '开始 session'}
              </Button>
            </div>

            {appSessions.length > 0 ? (
              <>
                <Select
                  fullWidth
                  size="small"
                  label="选择 session"
                  value={selectedSession?.sessionId ?? ''}
                  options={appSessions.map((session) => ({
                    value: session.sessionId,
                    label: `${session.sessionId} · ${session.status} · ${session.observations.length} obs`,
                  }))}
                  onChange={(value: string | number) => {
                    setSelectedSessionId(String(value))
                    setEvaluation(null)
                    setLeakReport(null)
                  }}
                />

                {selectedSession ? (
                  <div className="space-y-3">
                    <div className="flex flex-wrap gap-2">
                      <Chip
                        size="small"
                        color={statusColor(selectedSession.status)}
                        label={`Session: ${selectedSession.status}`}
                      />
                      <Chip
                        size="small"
                        color={statusColor(selectedSession.diagnosticsStatus)}
                        label={`Diagnostics: ${selectedSession.diagnosticsStatus}`}
                      />
                      <Chip
                        size="small"
                        label={`Observations: ${selectedSession.observations.length}`}
                      />
                      <Chip
                        size="small"
                        label={`Started: ${formatTime(selectedSession.startedAt)}`}
                      />
                      {selectedSession.endedAt ? (
                        <Chip
                          size="small"
                          label={`Ended: ${formatTime(selectedSession.endedAt)}`}
                        />
                      ) : null}
                    </div>

                    <div className="flex flex-wrap gap-2">
                      <Button
                        size="small"
                        variant="outlined"
                        onClick={() => void handleRecordObservation()}
                        disabled={sessionPending}
                      >
                        记录快照
                      </Button>
                      <Button
                        size="small"
                        variant="outlined"
                        onClick={() => void handleEvaluateSession()}
                        disabled={sessionPending}
                      >
                        评估归因
                      </Button>
                      <Button
                        size="small"
                        variant="outlined"
                        onClick={() => void handleVerifySessionLeak()}
                        disabled={sessionPending}
                      >
                        检查泄漏维度
                      </Button>
                    </div>

                    <div className="flex flex-wrap gap-2">
                      <Button
                        size="small"
                        variant="outlined"
                        color="success"
                        onClick={() => void handleFinishSession('completed')}
                        disabled={sessionPending || !!selectedSession.endedAt}
                      >
                        标记完成
                      </Button>
                      <Button
                        size="small"
                        variant="outlined"
                        color="warning"
                        onClick={() => void handleFinishSession('blocked')}
                        disabled={sessionPending || !!selectedSession.endedAt}
                      >
                        标记阻塞
                      </Button>
                      <Button
                        size="small"
                        variant="outlined"
                        color="error"
                        onClick={() => void handleFinishSession('failed')}
                        disabled={sessionPending || !!selectedSession.endedAt}
                      >
                        标记失败
                      </Button>
                    </div>

                    <div className="grid gap-2 text-xs sm:grid-cols-3">
                      <div>Rules: {selectedSession.projectedRules.length}</div>
                      <div>
                        Proxy groups:{' '}
                        {selectedSession.projectedProxyGroups.length}
                      </div>
                      <div>Warnings: {selectedSession.warnings.length}</div>
                    </div>

                    {selectedSession.observations.length > 0 ? (
                      <div className="space-y-2">
                        <div className="text-xs font-semibold">
                          Observation timeline
                        </div>
                        <div className="space-y-2">
                          {selectedSession.observations
                            .slice()
                            .reverse()
                            .slice(0, 5)
                            .map((observation) => (
                              <div
                                key={observation.observationId}
                                className="space-y-2 rounded-md bg-muted/40 px-3 py-2 text-xs"
                              >
                                <div className="flex flex-wrap items-center gap-2">
                                  <Chip
                                    size="small"
                                    color={statusColor(
                                      observation.attributionStatus,
                                    )}
                                    label={observation.attributionStatus}
                                  />
                                  <span>
                                    {formatTime(observation.recordedAt)}
                                  </span>
                                  <span>
                                    Active:{' '}
                                    {observation.traffic.activeConnectionCount}
                                  </span>
                                  <span>
                                    Closed:{' '}
                                    {observation.traffic.closedSinceLast}
                                  </span>
                                  <span>
                                    Up:{' '}
                                    {formatBytes(
                                      observation.traffic.uploadTotal,
                                    )}
                                  </span>
                                  <span>
                                    Down:{' '}
                                    {formatBytes(
                                      observation.traffic.downloadTotal,
                                    )}
                                  </span>
                                </div>
                                {observation.attributionCandidates.length >
                                0 ? (
                                  <div className="flex flex-wrap gap-2">
                                    {observation.attributionCandidates
                                      .slice(0, 4)
                                      .map((candidate) => (
                                        <Chip
                                          key={candidate.connectionId}
                                          size="small"
                                          label={`${candidate.host || candidate.process || candidate.connectionId} · ${candidate.chains.join(' > ') || 'no chain'}`}
                                          title={`rule=${candidate.rule}; matchedBy=${candidate.matchedBy.join(', ')}`}
                                        />
                                      ))}
                                  </div>
                                ) : (
                                  <div className="text-muted-foreground">
                                    No attribution candidates captured.
                                  </div>
                                )}
                                {observation.warnings.length > 0 ? (
                                  <div className="text-muted-foreground">
                                    {observation.warnings.join('；')}
                                  </div>
                                ) : null}
                              </div>
                            ))}
                        </div>
                      </div>
                    ) : null}
                  </div>
                ) : null}
              </>
            ) : (
              <div className="rounded-md bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                该应用还没有 session 记录。
              </div>
            )}

            {evaluation ? (
              <div className="space-y-2 rounded-md bg-muted/40 px-3 py-2 text-xs">
                <div className="flex flex-wrap items-center gap-2">
                  <Chip
                    size="small"
                    color={statusColor(evaluation.status)}
                    label={`Evaluation: ${evaluation.status}`}
                  />
                  <span className="font-medium">{evaluation.reason}</span>
                </div>
                <div className="grid gap-1 sm:grid-cols-4">
                  <div>Observations: {evaluation.summary.observationCount}</div>
                  <div>Matched: {evaluation.summary.matchedObservations}</div>
                  <div>Mismatch: {evaluation.summary.mismatchObservations}</div>
                  <div>
                    Unattributed: {evaluation.summary.unattributedObservations}
                  </div>
                  <div>Stale: {evaluation.summary.staleObservations}</div>
                  <div>
                    Candidates: {evaluation.summary.attributionCandidateCount}
                  </div>
                  <div>
                    Upload: {formatBytes(evaluation.summary.uploadTotal)}
                  </div>
                  <div>
                    Download: {formatBytes(evaluation.summary.downloadTotal)}
                  </div>
                </div>
                {evaluation.summary.observedHosts.length > 0 ? (
                  <div className="flex flex-wrap gap-2">
                    {evaluation.summary.observedHosts
                      .slice(0, 8)
                      .map((host) => (
                        <Chip key={host} size="small" label={host} />
                      ))}
                  </div>
                ) : null}
                {evaluation.summary.observedChains.length > 0 ? (
                  <div className="text-muted-foreground">
                    Chains: {evaluation.summary.observedChains.join(' / ')}
                  </div>
                ) : null}
                {evaluation.warnings.length > 0 ? (
                  <div className="text-muted-foreground">
                    {evaluation.warnings.join('；')}
                  </div>
                ) : null}
              </div>
            ) : null}

            {leakReport ? (
              <div className="space-y-2 rounded-md bg-muted/40 px-3 py-2 text-xs">
                <div className="flex flex-wrap items-center gap-2">
                  <Chip
                    size="small"
                    color={statusColor(leakReport.status)}
                    label={`Leak: ${leakReport.status}`}
                  />
                  <span className="font-medium">{leakReport.reason}</span>
                </div>
                <div className="grid gap-1 sm:grid-cols-4">
                  <div>Pass: {leakReport.summary.pass}</div>
                  <div>Warn: {leakReport.summary.warn}</div>
                  <div>Fail: {leakReport.summary.fail}</div>
                  <div>N/A: {leakReport.summary.notApplicable}</div>
                </div>
                <div className="flex flex-wrap gap-2">
                  {leakReport.checks.map((check) => (
                    <Chip
                      key={check.dimension}
                      size="small"
                      color={statusColor(check.status)}
                      label={`${check.dimension}: ${check.status}`}
                    />
                  ))}
                </div>
                <div className="space-y-1">
                  {leakReport.checks.map((check) => (
                    <div
                      key={`${check.dimension}-detail`}
                      className="rounded-md border border-border px-2 py-1"
                    >
                      <div className="font-medium">{check.message}</div>
                      {check.facts.length > 0 ? (
                        <div className="text-muted-foreground">
                          {check.facts.join('；')}
                        </div>
                      ) : null}
                      {check.warnings.length > 0 ? (
                        <div className="text-muted-foreground">
                          {check.warnings.join('；')}
                        </div>
                      ) : null}
                    </div>
                  ))}
                </div>
                {leakReport.warnings.length > 0 ? (
                  <div className="text-muted-foreground">
                    {leakReport.warnings.join('；')}
                  </div>
                ) : null}
              </div>
            ) : null}
          </div>
        ) : null}

        {diagnostics && plan && projection ? (
          <div className="space-y-3 rounded-lg border border-border p-3">
            <div className="flex flex-wrap items-center gap-2">
              <Chip
                size="small"
                color={statusColor(plan.status)}
                label={`Plan: ${plan.status}`}
              />
              <Chip
                size="small"
                color={statusColor(diagnostics.status)}
                label={`Diagnostics: ${diagnostics.status}`}
              />
              <Chip
                size="small"
                color={projection.mutatesRuntime ? 'error' : 'success'}
                label={
                  projection.mutatesRuntime
                    ? 'mutates runtime'
                    : 'planning-only projection'
                }
              />
            </div>

            <div className="grid gap-2 text-xs sm:grid-cols-2 lg:grid-cols-4">
              <div>Rules: {projection.rules.length}</div>
              <div>Proxy groups: {projection.proxyGroups.length}</div>
              <div>Facts: {diagnostics.facts.length}</div>
              <div>Warnings: {diagnostics.warnings.length}</div>
            </div>

            <div className="text-sm font-medium">{diagnostics.reason}</div>

            {diagnostics.checks.length > 0 ? (
              <div className="space-y-1">
                {diagnostics.checks.map((check) => (
                  <div
                    key={check.checkId}
                    className="flex items-center justify-between gap-3 rounded-md bg-muted/40 px-2 py-1 text-xs"
                  >
                    <span>{check.message}</span>
                    <Chip
                      size="small"
                      color={statusColor(check.status)}
                      label={check.status}
                    />
                  </div>
                ))}
              </div>
            ) : null}

            {projection.yamlPatch ? (
              <pre className="max-h-48 overflow-auto rounded-md bg-muted/50 p-2 text-xs">
                {projection.yamlPatch}
              </pre>
            ) : (
              <div className="flex items-center gap-2 rounded-md bg-muted/40 px-2 py-2 text-xs text-muted-foreground">
                <Boxes className="h-3 w-3" />
                当前规划没有生成 YAML patch。
              </div>
            )}
          </div>
        ) : null}
      </div>
    </Card>
  )
}
