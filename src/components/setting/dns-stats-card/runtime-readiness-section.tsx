import { useLockFn } from 'ahooks'
import { ShieldCheck } from 'lucide-react'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import {
  dnsDefaultRuntimeReadiness,
  type DnsDefaultRuntimeReadinessCheckStatus,
  type DnsDefaultRuntimeReadinessReport,
  type DnsDefaultRuntimeReadinessStatus,
  type DnsResolverRuntimeProbeReport,
} from '@/services/dns-api'
import { showNotice } from '@/services/notice-service'

import type { DnsStatusColor } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

interface RuntimeReadinessSectionProps {
  probeReport: DnsResolverRuntimeProbeReport | null
}

function readinessStatusColor(
  status: DnsDefaultRuntimeReadinessStatus,
): DnsStatusColor {
  switch (status) {
    case 'ready':
      return 'success'
    case 'degraded':
      return 'warning'
    case 'blocked':
      return 'error'
  }
}

function readinessCheckColor(
  status: DnsDefaultRuntimeReadinessCheckStatus,
): DnsStatusColor {
  switch (status) {
    case 'passed':
      return 'success'
    case 'warning':
      return 'warning'
    case 'failed':
      return 'error'
    case 'skipped':
      return 'default'
  }
}

export function RuntimeReadinessSection({
  probeReport,
}: RuntimeReadinessSectionProps) {
  const [report, setReport] =
    useState<DnsDefaultRuntimeReadinessReport | null>(null)
  const [pending, setPending] = useState(false)

  const handleReadiness = useLockFn(async () => {
    setPending(true)
    try {
      const nextReport = await dnsDefaultRuntimeReadiness(
        undefined,
        probeReport,
      )
      setReport(nextReport)
      showNotice.success('默认 DNS runtime readiness 已完成')
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
          title="默认 DNS runtime readiness"
          icon={<ShieldCheck className="h-3 w-3" />}
        />
        <Button
          size="small"
          variant="outlined"
          onClick={handleReadiness}
          disabled={pending}
        >
          {pending ? '评估中...' : '评估 readiness'}
        </Button>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        只读评估当前 runtime DNS 是否具备切换到 Rust 默认 resolver
        的条件；不会修改配置、不会 reload 内核。
      </div>

      {report ? (
        <div className="space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Readiness"
              chipLabel={report.status}
              chipColor={readinessStatusColor(report.status)}
            />
            <DnsTextRow
              label="结果"
              value={report.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={report.reason}
            />
          </div>

          <div className="grid gap-2 sm:grid-cols-4">
            <DnsTextRow label="Passed" value={report.summary.passed} />
            <DnsTextRow label="Warnings" value={report.summary.warnings} />
            <DnsTextRow label="Failed" value={report.summary.failed} />
            <DnsTextRow label="Skipped" value={report.summary.skipped} />
          </div>

          <div className="flex flex-wrap gap-2">
            {report.checks.map((check) => (
              <Chip
                key={check.checkId}
                size="small"
                color={readinessCheckColor(check.status)}
                label={`${check.checkId}: ${check.status}`}
                title={[check.message, ...check.details].join('\n')}
              />
            ))}
          </div>

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
