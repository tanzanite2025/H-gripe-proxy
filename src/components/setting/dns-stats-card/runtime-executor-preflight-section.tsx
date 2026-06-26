import { useLockFn } from 'ahooks'
import { FileCheck2 } from 'lucide-react'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import {
  dnsDefaultRuntimeOptInExecutorPreflight,
  type DnsDefaultRuntimeExecutorPreflightStatus,
  type DnsDefaultRuntimeOptInExecutorPreflightReport,
} from '@/services/dns-api'
import { showNotice } from '@/services/notice-service'

import type { DnsStatusColor } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

const EXECUTOR_PREFLIGHT_DOMAIN = 'example.com'

function executorPreflightStatusColor(
  status: DnsDefaultRuntimeExecutorPreflightStatus,
): DnsStatusColor {
  switch (status) {
    case 'ready':
      return 'success'
    case 'blocked':
      return 'error'
  }
}

export function RuntimeExecutorPreflightSection() {
  const [report, setReport] =
    useState<DnsDefaultRuntimeOptInExecutorPreflightReport | null>(null)
  const [pending, setPending] = useState(false)

  const handleExecutorPreflight = useLockFn(async () => {
    setPending(true)
    try {
      const nextReport = await dnsDefaultRuntimeOptInExecutorPreflight(
        undefined,
        EXECUTOR_PREFLIGHT_DOMAIN,
        true,
      )
      setReport(nextReport)
      showNotice.success('默认 DNS runtime executor preflight 已完成')
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
          title="默认 DNS runtime executor preflight"
          icon={<FileCheck2 className="h-3 w-3" />}
        />
        <Button
          size="small"
          variant="outlined"
          onClick={handleExecutorPreflight}
          disabled={pending}
        >
          {pending ? 'Dry-run 中...' : 'Dry-run executor'}
        </Button>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        只生成默认 DNS runtime 执行器 dry-run、mutation diff、audit record 与 rollback
        marker；不写配置、不 reload 内核、不执行切换。
      </div>

      {report ? (
        <div className="space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Executor"
              chipLabel={report.status}
              chipColor={executorPreflightStatusColor(report.status)}
            />
            <DnsTextRow
              label="结果"
              value={report.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={report.reason}
            />
          </div>

          <div className="grid gap-2 sm:grid-cols-2">
            <DnsTextRow label="Guard" value={report.guard.status} />
            <DnsTextRow label="Audit" value={report.auditRecord.eventId} />
            <DnsTextRow
              label="Mutation"
              value={report.wouldMutateRuntime ? '候选会修改' : '不会修改'}
            />
            <DnsTextRow
              label="执行状态"
              value={report.executed ? '已执行' : '未执行'}
            />
            <DnsTextRow
              label="Reload 内核"
              value={report.reloadMihomo ? '会 reload' : '不会 reload'}
            />
            <DnsTextRow
              label="Rollback marker"
              value={report.rollbackMarker.prepared ? '已准备' : '未准备'}
              valueTitle={report.rollbackMarker.strategy}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={report.dryRun ? 'success' : 'error'}
              label={`dryRun=${String(report.dryRun)}`}
            />
            <Chip
              size="small"
              color="default"
              label={`${report.mutationDiff.runtimeOwnerBefore} → ${report.mutationDiff.runtimeOwnerAfter}`}
            />
            <Chip
              size="small"
              color="default"
              label={`${report.mutationDiff.nameserverTargets.length} DNS target(s)`}
            />
          </div>

          {report.mutationDiff.nameserverTargets.length > 0 ? (
            <div className="rounded-md border border-gray-200 px-3 py-2 text-xs text-muted-foreground dark:border-gray-700">
              Targets: {report.mutationDiff.nameserverTargets.join('；')}
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
