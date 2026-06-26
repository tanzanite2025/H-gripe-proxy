import { useLockFn } from 'ahooks'
import { ShieldCheck } from 'lucide-react'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import {
  dnsDefaultRuntimeOptInExecutionGuard,
  type DnsDefaultRuntimeExecutionGuardStatus,
  type DnsDefaultRuntimeOptInExecutionGuardReport,
} from '@/services/dns-api'
import { showNotice } from '@/services/notice-service'

import type { DnsStatusColor } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

const EXECUTION_GUARD_DOMAIN = 'example.com'

function executionGuardStatusColor(
  status: DnsDefaultRuntimeExecutionGuardStatus,
): DnsStatusColor {
  switch (status) {
    case 'ready':
      return 'success'
    case 'blocked':
      return 'error'
  }
}

export function RuntimeExecutionGuardSection() {
  const [report, setReport] =
    useState<DnsDefaultRuntimeOptInExecutionGuardReport | null>(null)
  const [pending, setPending] = useState(false)

  const handleExecutionGuard = useLockFn(async () => {
    setPending(true)
    try {
      const nextReport = await dnsDefaultRuntimeOptInExecutionGuard(
        undefined,
        EXECUTION_GUARD_DOMAIN,
        true,
      )
      setReport(nextReport)
      showNotice.success('默认 DNS runtime execution guard 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(false)
    }
  })

  return (
    <div>
      <div className="mb-2 flex items-center justify-between gap-3">
        <DnsSectionHeading
          title="默认 DNS runtime execution guard"
          icon={<ShieldCheck className="h-3 w-3" />}
        />
        <Button
          size="small"
          variant="outlined"
          onClick={handleExecutionGuard}
          disabled={pending}
        >
          {pending ? '门禁检查中...' : '检查 execution guard'}
        </Button>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        真实切换前最后门禁：要求 executor preflight ready，并持久化 audit /
        rollback / superseded metadata；仍不写配置、不 reload 内核、不执行切换。
      </div>

      {report ? (
        <div className="space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Execution guard"
              chipLabel={report.status}
              chipColor={executionGuardStatusColor(report.status)}
            />
            <DnsTextRow
              label="结果"
              value={report.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={report.reason}
            />
          </div>

          <div className="grid gap-2 sm:grid-cols-2">
            <DnsTextRow label="Preflight" value={report.preflight.status} />
            <DnsTextRow
              label="Execution allowed"
              value={report.executionAllowed ? '允许后续显式执行' : '不允许执行'}
            />
            <DnsTextRow
              label="Persisted"
              value={report.persistence.prepared ? '已持久化' : '未持久化'}
            />
            <DnsTextRow
              label="Superseded"
              value={report.supersededState.state}
              valueTitle={report.supersededState.reason}
            />
            <DnsTextRow
              label="执行状态"
              value={report.executed ? '已执行' : '未执行'}
            />
            <DnsTextRow
              label="Reload 内核"
              value={report.reloadMihomo ? '会 reload' : '不会 reload'}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={report.userTriggerRequired ? 'success' : 'error'}
              label={`userTriggerRequired=${String(report.userTriggerRequired)}`}
            />
            <Chip
              size="small"
              color={report.mutatesRuntime ? 'error' : 'success'}
              label={`mutatesRuntime=${String(report.mutatesRuntime)}`}
            />
            <Chip
              size="small"
              color="default"
              label={`${report.persistence.auditPersisted ? 1 : 0} audit`}
            />
            <Chip
              size="small"
              color="default"
              label={`${report.persistence.rollbackMarkerPersisted ? 1 : 0} rollback`}
            />
          </div>

          {report.persistence.auditRecordPath ? (
            <div className="rounded-md border border-gray-200 px-3 py-2 text-xs text-muted-foreground dark:border-gray-700">
              Audit: {report.persistence.auditRecordPath}
            </div>
          ) : null}

          {report.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Blockers: {report.blockers.join('；')}
            </div>
          ) : null}

          {report.warnings.length > 0 ? (
            <div className="rounded-md border border-warning/40 bg-warning/5 px-3 py-2 text-xs text-muted-foreground">
              Warnings: {report.warnings.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}
