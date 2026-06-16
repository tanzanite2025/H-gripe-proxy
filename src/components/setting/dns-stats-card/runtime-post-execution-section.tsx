import { useLockFn } from 'ahooks'
import { ShieldCheck, Undo2 } from 'lucide-react'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import {
  dnsDefaultRuntimePostExecutionObservedVerification,
  dnsDefaultRuntimeRollbackDrill,
  type DnsDefaultRuntimePostExecutionObservedVerificationReport,
  type DnsDefaultRuntimePostExecutionVerificationStatus,
  type DnsDefaultRuntimeRollbackDrillReport,
  type DnsDefaultRuntimeRollbackDrillStatus,
} from '@/services/dns-api'
import { showNotice } from '@/services/notice-service'

import type { DnsStatusColor } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

const POST_EXECUTION_DOMAIN = 'example.com'

function verificationStatusColor(
  status: DnsDefaultRuntimePostExecutionVerificationStatus,
): DnsStatusColor {
  switch (status) {
    case 'verified':
      return 'success'
    case 'failed':
      return 'warning'
    case 'blocked':
      return 'error'
  }
}

function rollbackDrillStatusColor(
  status: DnsDefaultRuntimeRollbackDrillStatus,
): DnsStatusColor {
  switch (status) {
    case 'ready':
      return 'success'
    case 'blocked':
      return 'error'
  }
}

export function RuntimePostExecutionSection() {
  const [verificationReport, setVerificationReport] =
    useState<DnsDefaultRuntimePostExecutionObservedVerificationReport | null>(
      null,
    )
  const [drillReport, setDrillReport] =
    useState<DnsDefaultRuntimeRollbackDrillReport | null>(null)
  const [pending, setPending] = useState<'verify' | 'drill' | null>(null)

  const handleVerify = useLockFn(async () => {
    setPending('verify')
    try {
      const nextReport =
        await dnsDefaultRuntimePostExecutionObservedVerification(
          undefined,
          POST_EXECUTION_DOMAIN,
        )
      setVerificationReport(nextReport)
      setDrillReport(nextReport.rollbackDrill)
      showNotice.success('默认 DNS runtime 执行后观测验证已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleDrill = useLockFn(async () => {
    setPending('drill')
    try {
      const nextReport = await dnsDefaultRuntimeRollbackDrill()
      setDrillReport(nextReport)
      showNotice.success('默认 DNS runtime rollback drill 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  return (
    <div>
      <div className="mb-2 flex items-center justify-between gap-3">
        <DnsSectionHeading
          title="默认 DNS runtime post-execution verification"
          icon={<ShieldCheck className="h-3 w-3" />}
        />
        <div className="flex gap-2">
          <Button
            size="small"
            variant="outlined"
            onClick={handleVerify}
            disabled={pending !== null}
          >
            {pending === 'verify' ? '验证中...' : 'Observed verification'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleDrill}
            disabled={pending !== null}
          >
            <Undo2 className="mr-1 h-3 w-3" />
            {pending === 'drill' ? '演练中...' : 'Rollback drill'}
          </Button>
        </div>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        Batch Q 只读取 Batch P active state、execution audit 与 rollback
        metadata；比较执行后 observed query 与执行前 shadow evidence，不自动
        rollout、不自动 rollback、不 reload Mihomo。
      </div>

      {verificationReport ? (
        <div className="space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Verification"
              chipLabel={verificationReport.status}
              chipColor={verificationStatusColor(verificationReport.status)}
            />
            <DnsTextRow
              label="结果"
              value={verificationReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={verificationReport.reason}
            />
            <DnsTextRow
              label="Pre-shadow"
              value={
                verificationReport.preExecutionAuditRecord?.shadowStatus ??
                '未读取'
              }
            />
            <DnsTextRow
              label="Observed"
              value={verificationReport.observedEvidence.status}
            />
            <DnsTextRow
              label="Active runtime"
              value={verificationReport.activeState?.activeRuntime ?? '未激活'}
            />
            <DnsTextRow
              label="Reload Mihomo"
              value={verificationReport.reloadMihomo ? '会 reload' : '不会 reload'}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={verificationReport.failureAudit.required ? 'warning' : 'success'}
              label={`failureAudit=${String(verificationReport.failureAudit.required)}`}
            />
            <Chip
              size="small"
              color={verificationReport.mutatesRuntime ? 'warning' : 'default'}
              label={`mutatesRuntime=${String(verificationReport.mutatesRuntime)}`}
            />
            <Chip
              size="small"
              color={
                verificationReport.rollbackDrill.status === 'ready'
                  ? 'success'
                  : 'error'
              }
              label={`rollbackDrill=${verificationReport.rollbackDrill.status}`}
            />
          </div>

          {verificationReport.failureAudit.reasons.length > 0 ? (
            <div className="rounded-md border border-warning/40 bg-warning/5 px-3 py-2 text-xs text-muted-foreground">
              Failure audit: {verificationReport.failureAudit.reasons.join('；')}
            </div>
          ) : null}

          {verificationReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Blockers: {verificationReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {drillReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Rollback drill"
              chipLabel={drillReport.status}
              chipColor={rollbackDrillStatusColor(drillReport.status)}
            />
            <DnsTextRow
              label="结果"
              value={drillReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={drillReport.reason}
            />
            <DnsTextRow
              label="Would restore"
              value={drillReport.wouldRestoreRuntime}
            />
            <DnsTextRow
              label="Auto rollback"
              value={drillReport.autoRollback ? '会自动回滚' : '不会自动回滚'}
            />
          </div>

          {drillReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Drill blockers: {drillReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}
