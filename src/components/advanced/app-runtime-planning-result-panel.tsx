import { Boxes } from 'lucide-react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import type {
  AppRuntimeActiveProjectionRecord,
  AppRuntimeDiagnosticsReport,
  AppRuntimeMihomoProjection,
  AppRuntimePlan,
  AppRuntimeProjectionActivationPreflightReport,
  AppRuntimeProjectionArtifact,
  AppRuntimeProjectionRuntimeApplyAuditRecord,
  AppRuntimeProjectionRuntimeVerificationReport,
} from '@/services/app-runtime'

import { statusColor } from './app-runtime-planning-utils'

interface AppRuntimePlanningResultPanelProps {
  diagnostics: AppRuntimeDiagnosticsReport | null
  plan: AppRuntimePlan | null
  projection: AppRuntimeMihomoProjection | null
  projectionArtifact: AppRuntimeProjectionArtifact | null
  activationPreflight: AppRuntimeProjectionActivationPreflightReport | null
  activationPreflightPending: boolean
  activeProjection: AppRuntimeActiveProjectionRecord | null
  latestRuntimeApplyAudit: AppRuntimeProjectionRuntimeApplyAuditRecord | null
  runtimeVerification: AppRuntimeProjectionRuntimeVerificationReport | null
  activateMarkerPending: boolean
  runtimeApplyAllowed: boolean
  runtimeApplyPending: boolean
  runtimeVerificationPending: boolean
  activationRollbackPending: boolean
  onPreflightActivation: () => void
  onMarkActive: () => void
  onApplyRuntime: () => void
  onVerifyRuntime: () => void
  onRollbackActivation: () => void
}

export function AppRuntimePlanningResultPanel({
  diagnostics,
  plan,
  projection,
  projectionArtifact,
  activationPreflight,
  activationPreflightPending,
  activeProjection,
  latestRuntimeApplyAudit,
  runtimeVerification,
  activateMarkerPending,
  runtimeApplyAllowed,
  runtimeApplyPending,
  runtimeVerificationPending,
  activationRollbackPending,
  onPreflightActivation,
  onMarkActive,
  onApplyRuntime,
  onVerifyRuntime,
  onRollbackActivation,
}: AppRuntimePlanningResultPanelProps) {
  if (!diagnostics || !plan || !projection) {
    return null
  }

  return (
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

      {projectionArtifact ? (
        <div className="space-y-2 rounded-md border border-border bg-muted/30 p-2 text-xs">
          <div className="flex flex-wrap items-center justify-between gap-2">
            <div className="flex flex-wrap items-center gap-2">
              <span className="font-medium">Projection artifact</span>
              <Chip
                size="small"
                color={statusColor(projectionArtifact.validation.status)}
                label={projectionArtifact.validation.status}
              />
              <Chip
                size="small"
                color={projectionArtifact.mutatesRuntime ? 'error' : 'success'}
                label={`activation: ${projectionArtifact.activationMode}`}
              />
            </div>
            <Button
              size="small"
              variant="outlined"
              onClick={onPreflightActivation}
              disabled={
                activationPreflightPending || !projectionArtifact.storagePath
              }
            >
              {activationPreflightPending
                ? 'Preflight 中...'
                : 'Activation preflight'}
            </Button>
            <Button
              size="small"
              onClick={onMarkActive}
              disabled={
                activateMarkerPending || !projectionArtifact.storagePath
              }
            >
              {activateMarkerPending ? '标记中...' : '标记 active'}
            </Button>
            <Button
              size="small"
              variant="outlined"
              onClick={onApplyRuntime}
              disabled={
                runtimeApplyPending ||
                !projectionArtifact.storagePath ||
                !runtimeApplyAllowed ||
                projectionArtifact.validation.status === 'blocked' ||
                activeProjection?.artifactId !==
                  projectionArtifact.artifactId ||
                (activeProjection?.mutatesRuntime ?? false)
              }
            >
              {runtimeApplyPending
                ? '应用中...'
                : runtimeApplyAllowed
                  ? '显式应用 runtime'
                  : '等待 boundary 决策'}
            </Button>
            <Button
              size="small"
              variant="outlined"
              onClick={onVerifyRuntime}
              disabled={
                runtimeVerificationPending ||
                activeProjection?.artifactId !==
                  projectionArtifact.artifactId ||
                !(activeProjection?.mutatesRuntime ?? false)
              }
            >
              {runtimeVerificationPending ? '验证中...' : '验证 runtime'}
            </Button>
          </div>
          <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
            <div>ID: {projectionArtifact.artifactId}</div>
            <div>Checksum: {projectionArtifact.checksum.slice(0, 12)}</div>
            <div>Binding: {projectionArtifact.bindingId || '-'}</div>
            <div>{projectionArtifact.validation.reason}</div>
          </div>
          {projectionArtifact.storagePath ? (
            <div className="rounded-md bg-background/60 px-2 py-1">
              Stored: {projectionArtifact.storagePath}
            </div>
          ) : null}
          <div className="space-y-1">
            {projectionArtifact.validation.checks.map((check) => (
              <div
                key={check.checkId}
                className="flex items-center justify-between gap-3 rounded-md bg-background/60 px-2 py-1"
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
          {activationPreflight ? (
            <div className="space-y-2 rounded-md border border-border bg-background/50 p-2">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">Activation preflight</span>
                <Chip
                  size="small"
                  color={statusColor(activationPreflight.status)}
                  label={activationPreflight.status}
                />
                <span>{activationPreflight.reason}</span>
              </div>
              <div className="space-y-1">
                {activationPreflight.checks.map((check) => (
                  <div
                    key={check.checkId}
                    className="flex items-center justify-between gap-3 rounded-md bg-muted/40 px-2 py-1"
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
            </div>
          ) : null}
          {latestRuntimeApplyAudit ? (
            <div className="space-y-1 rounded-md border border-border bg-background/50 p-2">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">Runtime apply audit</span>
                <Chip
                  size="small"
                  color={statusColor(latestRuntimeApplyAudit.status)}
                  label={latestRuntimeApplyAudit.status}
                />
                {latestRuntimeApplyAudit.latestVerificationStatus ? (
                  <Chip
                    size="small"
                    color={statusColor(
                      latestRuntimeApplyAudit.latestVerificationStatus,
                    )}
                    label={`verified: ${latestRuntimeApplyAudit.latestVerificationStatus}`}
                  />
                ) : null}
              </div>
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
                <div>Audit: {latestRuntimeApplyAudit.auditId}</div>
                {latestRuntimeApplyAudit.runtimeApplyDecisionId ? (
                  <div>
                    Decision: {latestRuntimeApplyAudit.runtimeApplyDecisionId}
                  </div>
                ) : null}
                <div>
                  Candidate:{' '}
                  {latestRuntimeApplyAudit.candidateSummary.profileItemUid}
                </div>
                <div>
                  Groups:{' '}
                  {latestRuntimeApplyAudit.candidateSummary.proxyGroupCount}
                </div>
                <div>
                  Rules: {latestRuntimeApplyAudit.candidateSummary.ruleCount}
                </div>
              </div>
              <div className="text-muted-foreground">
                Validation: {latestRuntimeApplyAudit.validationOutcome}
              </div>
              <div className="text-muted-foreground">
                Rollback: {latestRuntimeApplyAudit.rollbackStrategy}
              </div>
            </div>
          ) : null}
          {runtimeVerification ? (
            <div className="space-y-1 rounded-md border border-border bg-background/50 p-2">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">
                  Observed runtime verification
                </span>
                <Chip
                  size="small"
                  color={statusColor(runtimeVerification.status)}
                  label={runtimeVerification.status}
                />
              </div>
              <div className="text-muted-foreground">
                {runtimeVerification.reason}
              </div>
              <div className="grid gap-2 sm:grid-cols-4">
                <div>Passed: {runtimeVerification.summary.passed}</div>
                <div>Warnings: {runtimeVerification.summary.warnings}</div>
                <div>Failed: {runtimeVerification.summary.failed}</div>
                <div>Skipped: {runtimeVerification.summary.skipped}</div>
              </div>
            </div>
          ) : null}
          {activeProjection ? (
            <div className="space-y-1 rounded-md border border-border bg-background/50 p-2">
              <div className="flex flex-wrap items-center gap-2">
                <span className="font-medium">Active projection marker</span>
                <Chip
                  size="small"
                  color={
                    activeProjection.artifactId ===
                    projectionArtifact.artifactId
                      ? 'success'
                      : 'warning'
                  }
                  label={activeProjection.activationKind}
                />
                <Button
                  size="small"
                  variant="outlined"
                  onClick={onRollbackActivation}
                  disabled={activationRollbackPending}
                >
                  {activationRollbackPending ? '回滚中...' : '回滚 marker'}
                </Button>
              </div>
              <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
                <div>ID: {activeProjection.artifactId}</div>
                <div>Checksum: {activeProjection.checksum.slice(0, 12)}</div>
                <div>
                  Mutates runtime: {String(activeProjection.mutatesRuntime)}
                </div>
                <div>
                  Rollback:{' '}
                  {activeProjection.rollback.previousArtifactId || 'empty'}
                </div>
              </div>
            </div>
          ) : null}
        </div>
      ) : null}

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
  )
}
