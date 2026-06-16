import { useLockFn } from 'ahooks'
import { ShieldAlert } from 'lucide-react'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import {
  dnsDefaultRuntimeOptInSwitchGuard,
  type DnsDefaultRuntimeOptInSwitchGuardReport,
  type DnsDefaultRuntimeOptInSwitchGuardStatus,
} from '@/services/dns-api'
import { showNotice } from '@/services/notice-service'

import type { DnsStatusColor } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

const SWITCH_GUARD_DOMAIN = 'example.com'

function switchGuardStatusColor(
  status: DnsDefaultRuntimeOptInSwitchGuardStatus,
): DnsStatusColor {
  switch (status) {
    case 'ready':
      return 'success'
    case 'blocked':
      return 'error'
  }
}

export function RuntimeSwitchGuardSection() {
  const [report, setReport] =
    useState<DnsDefaultRuntimeOptInSwitchGuardReport | null>(null)
  const [pending, setPending] = useState(false)

  const handleSwitchGuard = useLockFn(async () => {
    setPending(true)
    try {
      const nextReport = await dnsDefaultRuntimeOptInSwitchGuard(
        undefined,
        SWITCH_GUARD_DOMAIN,
        true,
      )
      setReport(nextReport)
      showNotice.success('默认 DNS runtime opt-in switch guard 已完成')
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
          title="默认 DNS runtime opt-in guard"
          icon={<ShieldAlert className="h-3 w-3" />}
        />
        <Button
          size="small"
          variant="outlined"
          onClick={handleSwitchGuard}
          disabled={pending}
        >
          {pending ? '预检中...' : '预检 opt-in guard'}
        </Button>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        实验性显式 opt-in 预检：要求 readiness=ready 且 shadow evidence 可用；这里只生成
        guard report，不执行 runtime mutation。
      </div>

      {report ? (
        <div className="space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Guard"
              chipLabel={report.status}
              chipColor={switchGuardStatusColor(report.status)}
            />
            <DnsTextRow
              label="结果"
              value={report.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={report.reason}
            />
          </div>

          <div className="grid gap-2 sm:grid-cols-2">
            <DnsTextRow label="Readiness" value={report.readiness.status} />
            <DnsTextRow label="Shadow" value={report.shadowEvidence.status} />
            <DnsTextRow
              label="Mutation"
              value={report.mutatesRuntime ? '会修改 runtime' : '只读预检'}
            />
            <DnsTextRow
              label="Rollback"
              value={report.rollbackPlan.supported ? '已规划' : '未支持'}
              valueTitle={report.rollbackPlan.strategy}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={report.explicitOptIn ? 'success' : 'error'}
              label={`explicitOptIn=${String(report.explicitOptIn)}`}
            />
            <Chip
              size="small"
              color="default"
              label={`activation=${report.activationMode}`}
            />
            <Chip
              size="small"
              color="default"
              label={`${report.rollbackPlan.previousRuntime} → ${report.rollbackPlan.candidateRuntime}`}
            />
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
