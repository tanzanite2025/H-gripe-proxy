import { useLockFn } from 'ahooks'
import { Activity, Boxes, ClipboardList, RefreshCw, Route } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import { Select } from '@/components/tailwind/Select'
import {
  diagnoseAppRuntime,
  evaluateAppRuntimeSession,
  getAppRuntimeState,
  recordAppRuntimeSessionObservation,
  projectAppRuntimePlanToMihomo,
  startAppRuntimeSession,
  verifyAppRuntimeSessionLeak,
  type AppRegistryEntry,
  type AppRuntimeDiagnosticsReport,
  type AppRuntimeMihomoProjection,
  type AppRuntimePlan,
  type AppRuntimeSessionEvaluationReport,
  type AppRuntimeSessionLeakReport,
  type AppRuntimeSessionRecord,
  type AppRuntimeStateDocument,
} from '@/services/app-runtime'
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
      return 'success'
    case 'degraded':
    case 'warning':
    case 'warn':
    case 'skipped':
    case 'notApplicable':
      return 'warning'
    case 'blocked':
    case 'rejected':
    case 'failed':
    case 'fail':
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

export function AppRuntimePlanningPanel() {
  const [state, setState] = useState<AppRuntimeStateDocument>(emptyState)
  const [selectedAppId, setSelectedAppId] = useState('')
  const [loading, setLoading] = useState(false)
  const [planning, setPlanning] = useState(false)
  const [sessionPending, setSessionPending] = useState(false)
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
      showNotice.success('应用运行时规划诊断已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPlanning(false)
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

  useEffect(() => {
    void loadState()
  }, [loadState])

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
                setSelectedAppId(String(value))
                setSelectedSessionId('')
                setPlan(null)
                setProjection(null)
                setDiagnostics(null)
                setEvaluation(null)
                setLeakReport(null)
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

                    <div className="grid gap-2 text-xs sm:grid-cols-3">
                      <div>Rules: {selectedSession.projectedRules.length}</div>
                      <div>
                        Proxy groups:{' '}
                        {selectedSession.projectedProxyGroups.length}
                      </div>
                      <div>Warnings: {selectedSession.warnings.length}</div>
                    </div>
                  </div>
                ) : null}
              </>
            ) : (
              <div className="rounded-md bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
                该应用还没有 session 记录。
              </div>
            )}

            {evaluation ? (
              <div className="rounded-md bg-muted/40 px-3 py-2 text-xs">
                <div className="mb-1 font-medium">{evaluation.reason}</div>
                <div className="grid gap-1 sm:grid-cols-3">
                  <div>Matched: {evaluation.summary.matchedObservations}</div>
                  <div>Mismatch: {evaluation.summary.mismatchObservations}</div>
                  <div>
                    Unattributed: {evaluation.summary.unattributedObservations}
                  </div>
                </div>
              </div>
            ) : null}

            {leakReport ? (
              <div className="space-y-2 rounded-md bg-muted/40 px-3 py-2 text-xs">
                <div className="font-medium">{leakReport.reason}</div>
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
