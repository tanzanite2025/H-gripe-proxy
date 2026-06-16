import type { ChangeEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { TextField } from '@/components/tailwind/TextField'
import type {
  AppPolicyBinding,
  AppRegistryEntry,
  AppRuntimeSessionRecord,
  DnsProfile,
  NodePool,
  SecurityProfile,
} from '@/services/app-runtime'

export interface AppRuntimeOverviewRow {
  app: AppRegistryEntry
  binding: AppPolicyBinding | null
  nodePool: NodePool | null
  dnsProfile: DnsProfile | null
  securityProfile: SecurityProfile | null
  sessions: AppRuntimeSessionRecord[]
  openSessions: number
  issues: string[]
}

interface AppRuntimeOverviewPanelProps {
  rows: AppRuntimeOverviewRow[]
  filteredRows: AppRuntimeOverviewRow[]
  filter: string
  selectedAppId: string
  onFilterChange: (value: string) => void
  onSelectApp: (appId: string) => void
}

export function AppRuntimeOverviewPanel({
  rows,
  filteredRows,
  filter,
  selectedAppId,
  onFilterChange,
  onSelectApp,
}: AppRuntimeOverviewPanelProps) {
  if (rows.length === 0) {
    return null
  }

  return (
    <div className="space-y-3 rounded-lg border border-border p-3">
      <div>
        <div className="text-sm font-semibold">应用编排概览</div>
        <div className="mt-1 text-xs text-muted-foreground">
          汇总 Rust state 中的 app → policy binding → node / DNS / security
          关系，便于快速定位下一步诊断对象。
        </div>
      </div>

      <div className="grid gap-2 lg:grid-cols-[minmax(0,1fr)_auto]">
        <TextField
          fullWidth
          size="small"
          label="过滤应用 / 绑定 / profile / issue"
          value={filter}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => onFilterChange(event.target.value)}
        />
        <div className="flex flex-wrap items-end gap-2">
          <Chip
            size="small"
            color={
              rows.some((row) => row.issues.length > 0) ? 'warning' : 'success'
            }
            label={`Issues: ${rows.filter((row) => row.issues.length > 0).length}`}
          />
          <Chip
            size="small"
            label={`Showing: ${filteredRows.length}/${rows.length}`}
          />
        </div>
      </div>

      <div className="grid gap-2">
        {filteredRows.map((row) => (
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
                  <span className="text-warning">{row.issues.join('；')}</span>
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
                  row.app.appId === selectedAppId ? 'contained' : 'outlined'
                }
                onClick={() => onSelectApp(row.app.appId)}
              >
                {row.app.appId === selectedAppId ? '已选择' : '选择'}
              </Button>
            </div>
          </div>
        ))}
        {filteredRows.length === 0 ? (
          <div className="rounded-md bg-muted/40 px-3 py-2 text-xs text-muted-foreground">
            没有匹配当前过滤条件的应用。
          </div>
        ) : null}
      </div>
    </div>
  )
}
