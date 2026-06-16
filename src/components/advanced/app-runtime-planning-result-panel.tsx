import { Boxes } from 'lucide-react'

import { Chip } from '@/components/tailwind/Chip'
import type {
  AppRuntimeDiagnosticsReport,
  AppRuntimeMihomoProjection,
  AppRuntimePlan,
  AppRuntimeProjectionArtifact,
} from '@/services/app-runtime'

import { statusColor } from './app-runtime-planning-utils'

interface AppRuntimePlanningResultPanelProps {
  diagnostics: AppRuntimeDiagnosticsReport | null
  plan: AppRuntimePlan | null
  projection: AppRuntimeMihomoProjection | null
  projectionArtifact: AppRuntimeProjectionArtifact | null
}

export function AppRuntimePlanningResultPanel({
  diagnostics,
  plan,
  projection,
  projectionArtifact,
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
          <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-4">
            <div>ID: {projectionArtifact.artifactId}</div>
            <div>Checksum: {projectionArtifact.checksum.slice(0, 12)}</div>
            <div>Binding: {projectionArtifact.bindingId || '-'}</div>
            <div>{projectionArtifact.validation.reason}</div>
          </div>
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
