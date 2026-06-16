import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'

import { statusColor } from './app-runtime-planning-utils'

export interface AggregateDiagnosticsItem {
  key: string
  label: string
  status: string
  detail: string
}

export interface AggregateDiagnosticAction {
  key: string
  scope: string
  status: string
  message: string
  detail: string
  actionLabel?: string
  action?:
    | 'focus-state'
    | 'run-diagnostics'
    | 'run-dns-probe'
    | 'accept-dns-handoff'
    | 'complete-control-plane'
    | 'complete-staged-activation-lifecycle'
    | 'closeout-staged-activation'
}

interface AppRuntimeAggregateDiagnosticsPanelProps {
  items: AggregateDiagnosticsItem[]
  actions: AggregateDiagnosticAction[]
  dnsWarnings: string[]
  onActionClick?: (action: AggregateDiagnosticAction) => void
}

export function AppRuntimeAggregateDiagnosticsPanel({
  items,
  actions,
  dnsWarnings,
  onActionClick,
}: AppRuntimeAggregateDiagnosticsPanelProps) {
  return (
    <div className="space-y-3 rounded-lg border border-border p-3">
      <div>
        <div className="text-sm font-semibold">聚合诊断摘要</div>
        <div className="mt-1 text-xs text-muted-foreground">
          把 overview state issue、planning diagnostics、DNS controlled probe 和
          runtime boundary 放在同一视图，避免分散查状态。
        </div>
      </div>

      <div className="grid gap-2 lg:grid-cols-2">
        {items.map((item) => (
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

      <div className="space-y-2">
        <div className="text-xs font-semibold">待处理动作</div>
        <div className="space-y-1">
          {actions.map((action) => (
            <div
              key={action.key}
              className="grid gap-2 rounded-md bg-muted/40 px-3 py-2 text-xs lg:grid-cols-[120px_minmax(0,1fr)_auto]"
            >
              <div className="text-muted-foreground">{action.scope}</div>
              <div>
                <div className="font-medium">{action.message}</div>
                <div className="text-muted-foreground">{action.detail}</div>
              </div>
              <div className="flex items-start justify-end gap-2">
                {action.action && action.actionLabel ? (
                  <Button
                    size="small"
                    variant="outlined"
                    onClick={() => onActionClick?.(action)}
                  >
                    {action.actionLabel}
                  </Button>
                ) : null}
                <Chip
                  size="small"
                  color={statusColor(action.status)}
                  label={action.status}
                />
              </div>
            </div>
          ))}
        </div>
      </div>

      {dnsWarnings.length ? (
        <div className="rounded-md bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
          DNS probe warnings: {dnsWarnings.join('；')}
        </div>
      ) : null}
    </div>
  )
}
