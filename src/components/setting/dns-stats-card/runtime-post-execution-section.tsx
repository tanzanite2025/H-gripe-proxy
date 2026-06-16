import { useLockFn } from 'ahooks'
import { ShieldCheck, Undo2 } from 'lucide-react'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import {
  dnsDefaultRuntimeExpandedOptInExecutionGate,
  dnsDefaultRuntimeExpandedOptInExecutionPreflight,
  dnsDefaultRuntimePostExecutionObservedVerification,
  dnsDefaultRuntimeRollbackDrill,
  type DnsDefaultRuntimeExpandedOptInExecutionGateReport,
  type DnsDefaultRuntimeExpandedOptInExecutionGateStatus,
  type DnsDefaultRuntimeExpandedOptInExecutionPreflightReport,
  type DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus,
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

function expandedGateStatusColor(
  status: DnsDefaultRuntimeExpandedOptInExecutionGateStatus,
): DnsStatusColor {
  switch (status) {
    case 'ready':
      return 'success'
    case 'blocked':
      return 'error'
  }
}

function expandedPreflightStatusColor(
  status: DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus,
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
  const [gateReport, setGateReport] =
    useState<DnsDefaultRuntimeExpandedOptInExecutionGateReport | null>(null)
  const [preflightReport, setPreflightReport] =
    useState<DnsDefaultRuntimeExpandedOptInExecutionPreflightReport | null>(
      null,
    )
  const [pending, setPending] = useState<
    'verify' | 'drill' | 'gate' | 'preflight' | null
  >(null)

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

  const handleExpandedGate = useLockFn(async () => {
    setPending('gate')
    try {
      const nextReport = await dnsDefaultRuntimeExpandedOptInExecutionGate(
        undefined,
        POST_EXECUTION_DOMAIN,
        true,
      )
      setGateReport(nextReport)
      setVerificationReport(nextReport.postExecution)
      setDrillReport(nextReport.postExecution.rollbackDrill)
      showNotice.success('默认 DNS runtime expanded opt-in gate 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleExpandedPreflight = useLockFn(async () => {
    setPending('preflight')
    try {
      const nextReport =
        await dnsDefaultRuntimeExpandedOptInExecutionPreflight(
          undefined,
          POST_EXECUTION_DOMAIN,
          true,
        )
      setPreflightReport(nextReport)
      setGateReport(nextReport.gate)
      setVerificationReport(nextReport.gate.postExecution)
      setDrillReport(nextReport.gate.postExecution.rollbackDrill)
      showNotice.success('默认 DNS runtime expanded execution preflight 已完成')
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
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedGate}
            disabled={pending !== null}
          >
            {pending === 'gate' ? '评估中...' : 'Expanded gate'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedPreflight}
            disabled={pending !== null}
          >
            {pending === 'preflight' ? '预检中...' : 'Expanded preflight'}
          </Button>
        </div>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        Batch Q 只读取 Batch P active state、execution audit 与 rollback
        metadata；比较执行后 observed query 与执行前 shadow evidence，不自动
        rollout、不自动 rollback、不 reload Mihomo。Expanded gate / preflight
        只判断是否允许后续更大范围显式 opt-in，并持久化下一批 active profile reload
        候选预检记录；本批次仍不执行 rollout。
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

      {gateReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Expanded gate"
              chipLabel={gateReport.status}
              chipColor={expandedGateStatusColor(gateReport.status)}
            />
            <DnsTextRow
              label="结果"
              value={gateReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={gateReport.reason}
            />
            <DnsTextRow
              label="Candidate scope"
              value={gateReport.candidateScope.name}
              valueTitle={gateReport.candidateScope.description}
            />
            <DnsTextRow
              label="Expansion"
              value={
                gateReport.expansionAllowed
                  ? '允许后续显式 opt-in'
                  : '不允许扩大执行'
              }
            />
            <DnsTextRow
              label="Auto rollout"
              value={gateReport.autoRollout ? '会自动 rollout' : '不会自动 rollout'}
            />
            <DnsTextRow
              label="执行状态"
              value={gateReport.executed ? '已执行' : '未执行'}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={gateReport.userTriggerRequired ? 'success' : 'error'}
              label={`userTriggerRequired=${String(gateReport.userTriggerRequired)}`}
            />
            <Chip
              size="small"
              color={gateReport.failureAuditRequired ? 'warning' : 'success'}
              label={`failureAuditRequired=${String(gateReport.failureAuditRequired)}`}
            />
            <Chip
              size="small"
              color={gateReport.mutatesRuntime ? 'error' : 'success'}
              label={`mutatesRuntime=${String(gateReport.mutatesRuntime)}`}
            />
          </div>

          {gateReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Gate blockers: {gateReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {preflightReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Expanded preflight"
              chipLabel={preflightReport.status}
              chipColor={expandedPreflightStatusColor(preflightReport.status)}
            />
            <DnsTextRow
              label="结果"
              value={preflightReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={preflightReport.reason}
            />
            <DnsTextRow
              label="Execution mode"
              value={preflightReport.preflightRecord.mutationPlan.executionMode}
              valueTitle={
                preflightReport.preflightRecord.mutationPlan.executionMode
              }
            />
            <DnsTextRow
              label="Record"
              value={
                preflightReport.preflightPersisted
                  ? '已持久化 preflight'
                  : '未持久化'
              }
              valueTitle={preflightReport.preflightRecordPath ?? undefined}
            />
            <DnsTextRow
              label="Would write profile"
              value={
                preflightReport.preflightRecord.mutationPlan.activeProfileWrite
                  ? '候选会写 active profile'
                  : '不会写 active profile'
              }
            />
            <DnsTextRow
              label="Would reload Mihomo"
              value={
                preflightReport.preflightRecord.mutationPlan.mihomoReload
                  ? '候选会 reload'
                  : '不会 reload'
              }
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={preflightReport.wouldMutateRuntime ? 'warning' : 'success'}
              label={`wouldMutateRuntime=${String(preflightReport.wouldMutateRuntime)}`}
            />
            <Chip
              size="small"
              color={preflightReport.mutatesRuntime ? 'error' : 'success'}
              label={`mutatesRuntime=${String(preflightReport.mutatesRuntime)}`}
            />
            <Chip
              size="small"
              color={preflightReport.reloadMihomo ? 'error' : 'success'}
              label={`reloadMihomo=${String(preflightReport.reloadMihomo)}`}
            />
          </div>

          {preflightReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Preflight blockers: {preflightReport.blockers.join('；')}
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
