import { AlertCircle, CheckCircle2, ShieldAlert } from 'lucide-react'

import { Alert } from '@/components/tailwind/Alert'
import { Chip } from '@/components/tailwind/Chip'
import type { LeakMonitorStatus } from '@/services/local-security'

interface StatusSummaryProps {
  status: LeakMonitorStatus
  monitorRunning: boolean
}

export function StatusSummary({
  status,
  monitorRunning,
}: StatusSummaryProps) {
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2">
        <Chip
          icon={
            status.localBindingSecure ? (
              <CheckCircle2 className="h-3.5 w-3.5" />
            ) : (
              <AlertCircle className="h-3.5 w-3.5" />
            )
          }
          label="本地绑定"
          color={status.localBindingSecure ? 'success' : 'error'}
          size="small"
        />
        <Chip
          icon={
            status.firewallRulesActive ? (
              <CheckCircle2 className="h-3.5 w-3.5" />
            ) : (
              <AlertCircle className="h-3.5 w-3.5" />
            )
          }
          label="防火墙规则"
          color={status.firewallRulesActive ? 'success' : 'warning'}
          size="small"
        />
        <Chip
          icon={
            status.externalAccessBlocked ? (
              <CheckCircle2 className="h-3.5 w-3.5" />
            ) : (
              <AlertCircle className="h-3.5 w-3.5" />
            )
          }
          label="外部访问阻断"
          color={status.externalAccessBlocked ? 'success' : 'error'}
          size="small"
        />
        <Chip
          icon={
            status.processHidden ? (
              <CheckCircle2 className="h-3.5 w-3.5" />
            ) : (
              <AlertCircle className="h-3.5 w-3.5" />
            )
          }
          label="进程隐匿"
          color={status.processHidden ? 'success' : 'default'}
          size="small"
        />
        <Chip
          label={monitorRunning ? '监控运行中' : '监控未启动'}
          color={monitorRunning ? 'info' : 'default'}
          size="small"
        />
        {status.autoFixApplied ? (
          <Chip label="已执行自动修复" color="success" size="small" />
        ) : null}
      </div>

      {status.leakDetected ? (
        <Alert severity="error" className="text-sm">
          <div className="space-y-1">
            <div className="flex items-center gap-2 font-bold">
              <ShieldAlert className="h-4 w-4" />
              检测到本地安全泄漏
            </div>
            {status.leakType ? <div className="text-xs">{status.leakType}</div> : null}
          </div>
        </Alert>
      ) : null}
    </div>
  )
}
