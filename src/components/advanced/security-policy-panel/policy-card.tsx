import { Play, Shield, Square, Trash2 } from 'lucide-react'

import { Chip, IconButton, Switch } from '@/components/tailwind'

interface PolicyCardProps {
  policy: ISecurityPolicy
  state: IAppliedPolicyState | null
  busy: boolean
  hasUnsavedChanges: boolean
  onToggleEnabled: (enabled: boolean) => void
  onApply: () => void
  onRevoke: () => void
  onEdit: () => void
  onDelete: () => void
}

export function PolicyCard({
  policy,
  state,
  busy,
  hasUnsavedChanges,
  onToggleEnabled,
  onApply,
  onRevoke,
  onEdit,
  onDelete,
}: PolicyCardProps) {
  const isApplied = state?.applied ?? false

  return (
    <div
      className={`rounded-lg border bg-card p-4 ${
        isApplied ? 'border-green-500' : 'border-border'
      }`}
    >
      <div className="flex items-start justify-between gap-3">
        <div className="space-y-2">
          <div className="flex flex-wrap items-center gap-2">
            <span className="text-base font-semibold">{policy.name}</span>
            <Chip
              size="small"
              color={isApplied ? 'success' : 'default'}
              label={isApplied ? '已应用' : '未应用'}
            />
            {!policy.enabled ? (
              <Chip size="small" color="warning" label="已禁用" />
            ) : null}
            <Chip
              size="small"
              color="info"
              label={`${policy.rules.length} 条规则`}
            />
          </div>

          {policy.description ? (
            <p className="text-sm text-muted-foreground">{policy.description}</p>
          ) : null}
        </div>

        <div className="flex items-center gap-1">
          <Switch checked={policy.enabled} onCheckedChange={onToggleEnabled} />
          {isApplied ? (
            <IconButton
              size="small"
              color="warning"
              onClick={onRevoke}
              disabled={busy || hasUnsavedChanges}
              aria-label={`撤销策略 ${policy.name}`}
            >
              <Square className="h-4 w-4" />
            </IconButton>
          ) : (
            <IconButton
              size="small"
              color="primary"
              onClick={onApply}
              disabled={busy || hasUnsavedChanges || !policy.enabled}
              aria-label={`应用策略 ${policy.name}`}
            >
              <Play className="h-4 w-4" />
            </IconButton>
          )}
          <IconButton
            size="small"
            color="default"
            onClick={onEdit}
            aria-label={`编辑策略 ${policy.name}`}
          >
            <Shield className="h-4 w-4" />
          </IconButton>
          <IconButton
            size="small"
            color="error"
            onClick={onDelete}
            aria-label={`删除策略 ${policy.name}`}
          >
            <Trash2 className="h-4 w-4" />
          </IconButton>
        </div>
      </div>

      <div className="mt-3 space-y-1">
        {policy.rules.map((rule, index) => (
          <div
            key={`${policy.name}-${index}`}
            className="rounded bg-muted/50 px-2 py-1 font-mono text-xs"
          >
            <span className="text-blue-500">{rule.ruleType}</span>
            {', '}
            <span>{rule.payload}</span>
            {', '}
            <span className="text-green-600">{rule.proxy}</span>
          </div>
        ))}
      </div>

      {isApplied && state ? (
        <div className="mt-3 text-xs text-muted-foreground">
          运行时规则索引：[{state.ruleIndices.join(', ')}]
        </div>
      ) : null}
    </div>
  )
}
