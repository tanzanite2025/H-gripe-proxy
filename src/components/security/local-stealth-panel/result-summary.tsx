import { ShieldCheck, ShieldX } from 'lucide-react'

import { Alert, Chip } from '@/components/tailwind'
import type { StealthApplyResult } from '@/services/local-stealth'

interface ResultSummaryProps {
  result: StealthApplyResult
}

export function ResultSummary({ result }: ResultSummaryProps) {
  return (
    <div className="space-y-3 rounded-xl border border-border bg-paper p-4">
      <div className="flex flex-wrap gap-2">
        <Chip
          size="small"
          color={result.process_stealth_applied ? 'success' : 'error'}
          icon={
            result.process_stealth_applied ? (
              <ShieldCheck className="h-3.5 w-3.5" />
            ) : (
              <ShieldX className="h-3.5 w-3.5" />
            )
          }
          label="进程隐匿"
        />
        <Chip
          size="small"
          color={result.port_stealth_applied ? 'success' : 'error'}
          icon={
            result.port_stealth_applied ? (
              <ShieldCheck className="h-3.5 w-3.5" />
            ) : (
              <ShieldX className="h-3.5 w-3.5" />
            )
          }
          label={
            result.allocated_port
              ? `端口隐匿 (${result.allocated_port})`
              : '端口隐匿'
          }
        />
        <Chip
          size="small"
          color={result.anti_discovery_applied ? 'success' : 'error'}
          icon={
            result.anti_discovery_applied ? (
              <ShieldCheck className="h-3.5 w-3.5" />
            ) : (
              <ShieldX className="h-3.5 w-3.5" />
            )
          }
          label="防本地发现"
        />
      </div>

      {result.discovery_messages.length > 0 ? (
        <Alert severity="info" className="text-sm">
          {result.discovery_messages.join(' / ')}
        </Alert>
      ) : null}

      {result.errors.length > 0 ? (
        <Alert severity="warning" className="text-sm">
          {result.errors.join('；')}
        </Alert>
      ) : null}
    </div>
  )
}
