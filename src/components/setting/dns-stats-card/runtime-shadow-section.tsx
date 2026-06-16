import { useLockFn } from 'ahooks'
import { GitCompareArrows } from 'lucide-react'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import {
  dnsDefaultRuntimeShadowEvidence,
  type DnsDefaultRuntimeShadowEvidenceReport,
  type DnsDefaultRuntimeShadowEvidenceStatus,
} from '@/services/dns-api'
import { showNotice } from '@/services/notice-service'

import type { DnsStatusColor } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

const SHADOW_EVIDENCE_DOMAIN = 'example.com'

function shadowStatusColor(
  status: DnsDefaultRuntimeShadowEvidenceStatus,
): DnsStatusColor {
  switch (status) {
    case 'matched':
      return 'success'
    case 'mismatched':
      return 'warning'
    case 'blocked':
      return 'error'
    case 'incomplete':
      return 'warning'
  }
}

export function RuntimeShadowSection() {
  const [report, setReport] =
    useState<DnsDefaultRuntimeShadowEvidenceReport | null>(null)
  const [pending, setPending] = useState(false)

  const handleShadowEvidence = useLockFn(async () => {
    setPending(true)
    try {
      const nextReport = await dnsDefaultRuntimeShadowEvidence(
        undefined,
        SHADOW_EVIDENCE_DOMAIN,
      )
      setReport(nextReport)
      showNotice.success('默认 DNS runtime shadow evidence 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(false)
    }
  })

  const rustResult = report?.query.rustReport.result
  const systemResult = report?.query.systemResult

  return (
    <div>
      <div className="mb-2 flex items-center justify-between gap-3">
        <DnsSectionHeading
          title="默认 DNS runtime shadow evidence"
          icon={<GitCompareArrows className="h-3 w-3" />}
        />
        <Button
          size="small"
          variant="outlined"
          onClick={handleShadowEvidence}
          disabled={pending}
        >
          {pending ? '对比中...' : '采集 shadow evidence'}
        </Button>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        对同一域名只读对比 Rust resolver 与系统 resolver 结果；只采集 evidence，不切换默认
        DNS runtime。
      </div>

      {report ? (
        <div className="space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Shadow"
              chipLabel={report.status}
              chipColor={shadowStatusColor(report.status)}
            />
            <DnsTextRow
              label="结果"
              value={report.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={report.reason}
            />
          </div>

          <div className="grid gap-2 sm:grid-cols-2">
            <DnsTextRow label="域名" value={report.query.domain} />
            <DnsTextRow
              label="Readiness"
              value={report.readiness.status}
              valueClassName="text-sm font-bold"
            />
            <DnsTextRow
              label="Rust IP"
              value={rustResult?.ip || '未返回'}
              valueTitle={rustResult?.error ?? undefined}
            />
            <DnsTextRow
              label="System IP"
              value={systemResult?.ip || '未返回'}
              valueTitle={systemResult?.error ?? undefined}
            />
            <DnsTextRow
              label="Rust 延迟"
              value={rustResult ? `${rustResult.latency}ms` : '未返回'}
            />
            <DnsTextRow
              label="System 延迟"
              value={systemResult ? `${systemResult.latency}ms` : '未返回'}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            {report.query.rustReport.attemptedServers.map((server) => (
              <Chip
                key={server}
                size="small"
                color={report.query.ipMatch ? 'success' : 'warning'}
                label={`Rust target: ${server}`}
              />
            ))}
          </div>

          {report.query.mismatchReason ? (
            <div className="rounded-md border border-warning/40 bg-warning/5 px-3 py-2 text-xs text-muted-foreground">
              Diff: {report.query.mismatchReason}
            </div>
          ) : null}

          {report.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Blockers: {report.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}
