import { ClipboardList } from 'lucide-react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { Select } from '@/components/tailwind/Select'
import type {
  AppRuntimeSessionEvaluationReport,
  AppRuntimeSessionLeakReport,
  AppRuntimeSessionRecord,
} from '@/services/app-runtime'

import {
  formatBytes,
  formatTime,
  statusColor,
  type FinishableSessionStatus,
} from './app-runtime-planning-utils'

interface AppRuntimeSessionPanelProps {
  sessions: AppRuntimeSessionRecord[]
  selectedSession: AppRuntimeSessionRecord | null
  evaluation: AppRuntimeSessionEvaluationReport | null
  leakReport: AppRuntimeSessionLeakReport | null
  pending: boolean
  onSelectSession: (sessionId: string) => void
  onStartSession: () => void
  onRecordObservation: () => void
  onEvaluateSession: () => void
  onVerifySessionLeak: () => void
  onFinishSession: (status: FinishableSessionStatus) => void
}

export function AppRuntimeSessionPanel({
  sessions,
  selectedSession,
  evaluation,
  leakReport,
  pending,
  onSelectSession,
  onStartSession,
  onRecordObservation,
  onEvaluateSession,
  onVerifySessionLeak,
  onFinishSession,
}: AppRuntimeSessionPanelProps) {
  return (
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
        <Button size="small" onClick={onStartSession} disabled={pending}>
          {pending ? '处理中...' : '开始 session'}
        </Button>
      </div>

      {sessions.length > 0 ? (
        <>
          <Select
            fullWidth
            size="small"
            label="选择 session"
            value={selectedSession?.sessionId ?? ''}
            options={sessions.map((session) => ({
              value: session.sessionId,
              label: `${session.sessionId} · ${session.status} · ${session.observations.length} obs`,
            }))}
            onChange={(value: string | number) =>
              onSelectSession(String(value))
            }
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
                  onClick={onRecordObservation}
                  disabled={pending}
                >
                  记录快照
                </Button>
                <Button
                  size="small"
                  variant="outlined"
                  onClick={onEvaluateSession}
                  disabled={pending}
                >
                  评估归因
                </Button>
                <Button
                  size="small"
                  variant="outlined"
                  onClick={onVerifySessionLeak}
                  disabled={pending}
                >
                  检查泄漏维度
                </Button>
              </div>

              <div className="flex flex-wrap gap-2">
                <Button
                  size="small"
                  variant="outlined"
                  color="success"
                  onClick={() => onFinishSession('completed')}
                  disabled={pending || !!selectedSession.endedAt}
                >
                  标记完成
                </Button>
                <Button
                  size="small"
                  variant="outlined"
                  color="warning"
                  onClick={() => onFinishSession('blocked')}
                  disabled={pending || !!selectedSession.endedAt}
                >
                  标记阻塞
                </Button>
                <Button
                  size="small"
                  variant="outlined"
                  color="error"
                  onClick={() => onFinishSession('failed')}
                  disabled={pending || !!selectedSession.endedAt}
                >
                  标记失败
                </Button>
              </div>

              <div className="grid gap-2 text-xs sm:grid-cols-3">
                <div>Rules: {selectedSession.projectedRules.length}</div>
                <div>
                  Proxy groups: {selectedSession.projectedProxyGroups.length}
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
                              color={statusColor(observation.attributionStatus)}
                              label={observation.attributionStatus}
                            />
                            <span>{formatTime(observation.recordedAt)}</span>
                            <span>
                              Active:{' '}
                              {observation.traffic.activeConnectionCount}
                            </span>
                            <span>
                              Closed: {observation.traffic.closedSinceLast}
                            </span>
                            <span>
                              Up: {formatBytes(observation.traffic.uploadTotal)}
                            </span>
                            <span>
                              Down:{' '}
                              {formatBytes(observation.traffic.downloadTotal)}
                            </span>
                          </div>
                          {observation.attributionCandidates.length > 0 ? (
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
            <div>Upload: {formatBytes(evaluation.summary.uploadTotal)}</div>
            <div>Download: {formatBytes(evaluation.summary.downloadTotal)}</div>
          </div>
          {evaluation.summary.observedHosts.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {evaluation.summary.observedHosts.slice(0, 8).map((host) => (
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
  )
}
