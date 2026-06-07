import { CheckCircle2, Loader2, Pencil, Trash2 } from 'lucide-react'

import { Switch } from '@/components/base'
import { Chip } from '@/components/tailwind'
import type { ResidentialProxy } from '@/services/coordinator'
import type { ResidentialProxyVerification } from '@/services/ip-reputation'

import { getVerificationColor, getVerificationLabel } from './constants'

interface ResidentialProxyRowProps {
  proxy: ResidentialProxy
  verification?: ResidentialProxyVerification
  verifying: boolean
  onToggleEnabled: (enabled: boolean) => void
  onVerify: () => void
  onEdit: () => void
  onDelete: () => void
}

export function ResidentialProxyRow({
  proxy,
  verification,
  verifying,
  onToggleEnabled,
  onVerify,
  onEdit,
  onDelete,
}: ResidentialProxyRowProps) {
  return (
    <div
      className={`flex items-center justify-between rounded-lg border p-3 ${
        proxy.enabled
          ? 'border-green-200 bg-green-50/50 dark:border-green-800 dark:bg-green-900/10'
          : 'border-gray-200 bg-gray-50 dark:border-gray-700 dark:bg-gray-800/50'
      }`}
    >
      <div className="flex min-w-0 items-center gap-3">
        <Switch checked={proxy.enabled} onCheckedChange={onToggleEnabled} />

        <div className="min-w-0">
          <div className="flex items-center gap-2">
            <span className="truncate text-sm font-medium">{proxy.name}</span>
            <Chip label={proxy.proxyType.toUpperCase()} color="default" size="small" />
            {proxy.region && (
              <Chip label={proxy.region} color="info" size="small" />
            )}
            {verification && (
              <Chip
                label={getVerificationLabel(verification)}
                color={getVerificationColor(verification)}
                size="small"
              />
            )}
          </div>
          <span className="text-xs text-gray-500">
            {proxy.server}:{proxy.port}
          </span>
        </div>
      </div>

      <div className="flex shrink-0 items-center gap-1">
        <button
          onClick={onVerify}
          disabled={verifying || !proxy.enabled}
          className="rounded p-1.5 hover:bg-gray-200 disabled:opacity-50 dark:hover:bg-gray-700"
          title="验证出口"
        >
          {verifying ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <CheckCircle2 className="h-3.5 w-3.5" />
          )}
        </button>
        <button
          onClick={onEdit}
          className="rounded p-1.5 hover:bg-gray-200 dark:hover:bg-gray-700"
          title="编辑节点"
        >
          <Pencil className="h-3.5 w-3.5" />
        </button>
        <button
          onClick={onDelete}
          className="rounded p-1.5 text-red-500 hover:bg-red-100 dark:hover:bg-red-900/30"
          title="删除节点"
        >
          <Trash2 className="h-3.5 w-3.5" />
        </button>
      </div>
    </div>
  )
}
