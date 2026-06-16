import { useLockFn } from 'ahooks'
import { ShieldCheck, Undo2 } from 'lucide-react'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import {
  dnsDefaultRuntimeExpandedOptInExecution,
  dnsDefaultRuntimeExpandedLifecycleCloseout,
  dnsDefaultRuntimeExpandedOptInExecutionGate,
  dnsDefaultRuntimeExpandedOptInExecutionPreflight,
  dnsDefaultRuntimeExpandedHoldPolicy,
  dnsDefaultRuntimeExpandedPostExecutionObservedVerification,
  dnsDefaultRuntimeExpandedReverify,
  dnsDefaultRuntimeExpandedReverifyHistory,
  dnsDefaultRuntimeExpandedRollback,
  dnsDefaultRuntimeExpandedRollbackDrill,
  dnsDefaultRuntimeExpandedStabilityGate,
  dnsDefaultRuntimePostExecutionObservedVerification,
  dnsDefaultRuntimeRollbackDrill,
  type DnsDefaultRuntimeExpandedOptInExecutionReport,
  type DnsDefaultRuntimeExpandedOptInExecutionStatus,
  type DnsDefaultRuntimeExpandedOptInExecutionGateReport,
  type DnsDefaultRuntimeExpandedOptInExecutionGateStatus,
  type DnsDefaultRuntimeExpandedOptInExecutionPreflightReport,
  type DnsDefaultRuntimeExpandedOptInExecutionPreflightStatus,
  type DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport,
  type DnsDefaultRuntimeExpandedReverifyHistoryReport,
  type DnsDefaultRuntimeExpandedReverifyReport,
  type DnsDefaultRuntimeExpandedRollbackDrillReport,
  type DnsDefaultRuntimeExpandedRollbackReport,
  type DnsDefaultRuntimeExpandedRollbackStatus,
  type DnsDefaultRuntimeExpandedHoldPolicyReport,
  type DnsDefaultRuntimeExpandedLifecycleCloseoutReport,
  type DnsDefaultRuntimeExpandedStabilityGateReport,
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

function expandedExecutionStatusColor(
  status: DnsDefaultRuntimeExpandedOptInExecutionStatus,
): DnsStatusColor {
  switch (status) {
    case 'executed':
      return 'success'
    case 'blocked':
    case 'failed':
      return 'error'
  }
}

function expandedRollbackStatusColor(
  status: DnsDefaultRuntimeExpandedRollbackStatus,
): DnsStatusColor {
  switch (status) {
    case 'restored':
      return 'success'
    case 'blocked':
    case 'failed':
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
  const [expandedExecutionReport, setExpandedExecutionReport] =
    useState<DnsDefaultRuntimeExpandedOptInExecutionReport | null>(null)
  const [expandedPostExecutionReport, setExpandedPostExecutionReport] =
    useState<DnsDefaultRuntimeExpandedPostExecutionObservedVerificationReport | null>(
      null,
    )
  const [expandedDrillReport, setExpandedDrillReport] =
    useState<DnsDefaultRuntimeExpandedRollbackDrillReport | null>(null)
  const [expandedStabilityGateReport, setExpandedStabilityGateReport] =
    useState<DnsDefaultRuntimeExpandedStabilityGateReport | null>(null)
  const [expandedHoldPolicyReport, setExpandedHoldPolicyReport] =
    useState<DnsDefaultRuntimeExpandedHoldPolicyReport | null>(null)
  const [expandedReverifyReport, setExpandedReverifyReport] =
    useState<DnsDefaultRuntimeExpandedReverifyReport | null>(null)
  const [expandedReverifyHistoryReport, setExpandedReverifyHistoryReport] =
    useState<DnsDefaultRuntimeExpandedReverifyHistoryReport | null>(null)
  const [expandedLifecycleCloseoutReport, setExpandedLifecycleCloseoutReport] =
    useState<DnsDefaultRuntimeExpandedLifecycleCloseoutReport | null>(null)
  const [expandedRollbackReport, setExpandedRollbackReport] =
    useState<DnsDefaultRuntimeExpandedRollbackReport | null>(null)
  const [pending, setPending] = useState<
    | 'verify'
    | 'drill'
    | 'gate'
    | 'preflight'
    | 'execute'
    | 'expandedVerify'
    | 'expandedDrill'
    | 'expandedStability'
    | 'expandedHold'
    | 'expandedReverify'
    | 'expandedHistory'
    | 'expandedCloseout'
    | 'rollback'
    | null
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

  const handleExpandedExecution = useLockFn(async () => {
    setPending('execute')
    try {
      const nextReport = await dnsDefaultRuntimeExpandedOptInExecution(
        undefined,
        POST_EXECUTION_DOMAIN,
        true,
      )
      setExpandedExecutionReport(nextReport)
      setPreflightReport(nextReport.preflight)
      setGateReport(nextReport.preflight.gate)
      setVerificationReport(nextReport.preflight.gate.postExecution)
      setDrillReport(nextReport.preflight.gate.postExecution.rollbackDrill)
      showNotice.success('默认 DNS runtime expanded execution 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleExpandedRollback = useLockFn(async () => {
    setPending('rollback')
    try {
      const nextReport = await dnsDefaultRuntimeExpandedRollback()
      setExpandedRollbackReport(nextReport)
      showNotice.success('默认 DNS runtime expanded rollback 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleExpandedPostExecution = useLockFn(async () => {
    setPending('expandedVerify')
    try {
      const nextReport =
        await dnsDefaultRuntimeExpandedPostExecutionObservedVerification(
          undefined,
          POST_EXECUTION_DOMAIN,
        )
      setExpandedPostExecutionReport(nextReport)
      setExpandedDrillReport(nextReport.rollbackDrill)
      showNotice.success('默认 DNS runtime expanded observed verification 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleExpandedDrill = useLockFn(async () => {
    setPending('expandedDrill')
    try {
      const nextReport = await dnsDefaultRuntimeExpandedRollbackDrill()
      setExpandedDrillReport(nextReport)
      showNotice.success('默认 DNS runtime expanded rollback drill 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleExpandedStabilityGate = useLockFn(async () => {
    setPending('expandedStability')
    try {
      const nextReport = await dnsDefaultRuntimeExpandedStabilityGate(
        undefined,
        POST_EXECUTION_DOMAIN,
        true,
      )
      setExpandedStabilityGateReport(nextReport)
      setExpandedPostExecutionReport(nextReport.postExecution)
      setExpandedDrillReport(nextReport.postExecution.rollbackDrill)
      showNotice.success('默认 DNS runtime expanded stability gate 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleExpandedHoldPolicy = useLockFn(async () => {
    setPending('expandedHold')
    try {
      const nextReport = await dnsDefaultRuntimeExpandedHoldPolicy(
        undefined,
        POST_EXECUTION_DOMAIN,
        true,
      )
      setExpandedHoldPolicyReport(nextReport)
      setExpandedStabilityGateReport(nextReport.stabilityGate)
      setExpandedPostExecutionReport(nextReport.stabilityGate.postExecution)
      setExpandedDrillReport(
        nextReport.stabilityGate.postExecution.rollbackDrill,
      )
      showNotice.success('默认 DNS runtime expanded hold policy 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleExpandedReverify = useLockFn(async () => {
    setPending('expandedReverify')
    try {
      const nextReport = await dnsDefaultRuntimeExpandedReverify(
        undefined,
        POST_EXECUTION_DOMAIN,
        true,
      )
      setExpandedReverifyReport(nextReport)
      setExpandedHoldPolicyReport(nextReport.holdPolicy)
      setExpandedStabilityGateReport(nextReport.holdPolicy.stabilityGate)
      setExpandedPostExecutionReport(
        nextReport.holdPolicy.stabilityGate.postExecution,
      )
      setExpandedDrillReport(
        nextReport.holdPolicy.stabilityGate.postExecution.rollbackDrill,
      )
      showNotice.success('默认 DNS runtime expanded reverify 已记录')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleExpandedReverifyHistory = useLockFn(async () => {
    setPending('expandedHistory')
    try {
      const nextReport = await dnsDefaultRuntimeExpandedReverifyHistory()
      setExpandedReverifyHistoryReport(nextReport)
      showNotice.success('默认 DNS runtime expanded reverify history 已汇总')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleExpandedLifecycleCloseout = useLockFn(async () => {
    setPending('expandedCloseout')
    try {
      const nextReport = await dnsDefaultRuntimeExpandedLifecycleCloseout()
      setExpandedLifecycleCloseoutReport(nextReport)
      setExpandedReverifyHistoryReport(nextReport.history)
      showNotice.success('默认 DNS runtime expanded lifecycle closeout 已完成')
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
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedExecution}
            disabled={pending !== null}
          >
            {pending === 'execute' ? '执行中...' : 'Expanded execute'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedPostExecution}
            disabled={pending !== null}
          >
            {pending === 'expandedVerify'
              ? '验证中...'
              : 'Expanded verify'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedDrill}
            disabled={pending !== null}
          >
            {pending === 'expandedDrill' ? '演练中...' : 'Expanded drill'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedStabilityGate}
            disabled={pending !== null}
          >
            {pending === 'expandedStability'
              ? '评估中...'
              : 'Expanded stability'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedHoldPolicy}
            disabled={pending !== null}
          >
            {pending === 'expandedHold' ? '评估中...' : 'Expanded hold'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedReverify}
            disabled={pending !== null}
          >
            {pending === 'expandedReverify'
              ? '记录中...'
              : 'Expanded reverify'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedReverifyHistory}
            disabled={pending !== null}
          >
            {pending === 'expandedHistory'
              ? '汇总中...'
              : 'Reverify history'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedLifecycleCloseout}
            disabled={pending !== null}
          >
            {pending === 'expandedCloseout'
              ? '收口中...'
              : 'Lifecycle closeout'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleExpandedRollback}
            disabled={pending !== null}
          >
            {pending === 'rollback' ? '回滚中...' : 'Expanded rollback'}
          </Button>
        </div>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        Batch Q 只读取 Batch P active state、execution audit 与 rollback
        metadata；比较执行后 observed query 与执行前 shadow evidence，不自动
        rollout、不自动 rollback、不 reload Mihomo。Expanded gate / preflight
        只判断是否允许后续更大范围显式 opt-in，并持久化 active profile reload
        候选预检记录；Expanded execute 需要用户显式点击，才会通过现有 Mihomo config
        reload 路径应用 DNS config，并可由 Expanded rollback 恢复。
        Expanded verify / drill 只读取 Batch T active state 与 audit
        metadata，生成 failure audit，不自动回滚。Expanded stability 只决定当前 session
        是否可保持 active，不做长期默认推广。Expanded hold 再加最小/最大观察窗口，
        超窗只建议显式 rollback。Expanded reverify 将一次显式 hold-window
        评估持久化为 audit record，方便重复验证。Reverify history 汇总多次记录，
        给出 stable threshold / rollback trend / closeout 建议。Lifecycle closeout
        合并 history 与 active state，给出下一控制面交接建议。
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

      {expandedLifecycleCloseoutReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Lifecycle closeout"
              chipLabel={expandedLifecycleCloseoutReport.status}
              chipColor={
                expandedLifecycleCloseoutReport.status === 'complete'
                  ? 'success'
                  : expandedLifecycleCloseoutReport.status === 'blocked'
                    ? 'error'
                    : 'warning'
              }
            />
            <DnsTextRow
              label="结果"
              value={expandedLifecycleCloseoutReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedLifecycleCloseoutReport.reason}
            />
            <DnsTextRow
              label="Next step"
              value={expandedLifecycleCloseoutReport.nextControlPlaneStep}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedLifecycleCloseoutReport.nextControlPlaneStep}
            />
            <DnsTextRow
              label="Recommended"
              value={expandedLifecycleCloseoutReport.recommendedAction}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedLifecycleCloseoutReport.recommendedAction}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={
                expandedLifecycleCloseoutReport.observationClosed
                  ? 'success'
                  : 'warning'
              }
              label={`closed=${String(expandedLifecycleCloseoutReport.observationClosed)}`}
            />
            <Chip
              size="small"
              color={
                expandedLifecycleCloseoutReport.handoffReady
                  ? 'success'
                  : 'default'
              }
              label={`handoff=${String(expandedLifecycleCloseoutReport.handoffReady)}`}
            />
            <Chip
              size="small"
              color={
                expandedLifecycleCloseoutReport.rollbackRecommended
                  ? 'warning'
                  : 'default'
              }
              label={`rollbackRecommended=${String(expandedLifecycleCloseoutReport.rollbackRecommended)}`}
            />
          </div>

          {expandedLifecycleCloseoutReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Lifecycle closeout blockers:{' '}
              {expandedLifecycleCloseoutReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {expandedReverifyHistoryReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Reverify history"
              chipLabel={expandedReverifyHistoryReport.status}
              chipColor={
                expandedReverifyHistoryReport.status === 'ready'
                  ? 'success'
                  : expandedReverifyHistoryReport.status === 'blocked'
                    ? 'error'
                    : 'warning'
              }
            />
            <DnsTextRow
              label="结果"
              value={expandedReverifyHistoryReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedReverifyHistoryReport.reason}
            />
            <DnsTextRow
              label="Records"
              value={`${expandedReverifyHistoryReport.recordCount} total / ${expandedReverifyHistoryReport.recordedCount} keep-active`}
            />
            <DnsTextRow
              label="Stable streak"
              value={`${expandedReverifyHistoryReport.stableStreak}/${expandedReverifyHistoryReport.requiredStableRecords}`}
            />
            <DnsTextRow
              label="Rollback trend"
              value={`${expandedReverifyHistoryReport.rollbackRecommendedCount} recommended`}
            />
            <DnsTextRow
              label="Recommended"
              value={expandedReverifyHistoryReport.recommendedAction}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedReverifyHistoryReport.recommendedAction}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={
                expandedReverifyHistoryReport.closeoutReady
                  ? 'success'
                  : 'warning'
              }
              label={`closeout=${String(expandedReverifyHistoryReport.closeoutReady)}`}
            />
            <Chip
              size="small"
              color={
                expandedReverifyHistoryReport.rollbackRecommended
                  ? 'warning'
                  : 'default'
              }
              label={`rollbackRecommended=${String(expandedReverifyHistoryReport.rollbackRecommended)}`}
            />
            <Chip
              size="small"
              color="default"
              label={`promotion=${String(expandedReverifyHistoryReport.promotionAllowed)}`}
            />
          </div>

          {expandedReverifyHistoryReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Reverify history blockers:{' '}
              {expandedReverifyHistoryReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {expandedReverifyReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Expanded reverify"
              chipLabel={expandedReverifyReport.status}
              chipColor={
                expandedReverifyReport.status === 'recorded'
                  ? 'success'
                  : expandedReverifyReport.status === 'blocked'
                    ? 'error'
                    : 'warning'
              }
            />
            <DnsTextRow
              label="结果"
              value={expandedReverifyReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedReverifyReport.reason}
            />
            <DnsTextRow
              label="Record"
              value={
                expandedReverifyReport.reverifyPersisted
                  ? '已持久化'
                  : '未持久化'
              }
            />
            <DnsTextRow
              label="Path"
              value={expandedReverifyReport.reverifyRecordPath ?? '未生成'}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedReverifyReport.reverifyRecordPath ?? undefined}
            />
            <DnsTextRow
              label="Hold status"
              value={expandedReverifyReport.reverifyRecord.holdStatus}
            />
            <DnsTextRow
              label="Post status"
              value={expandedReverifyReport.reverifyRecord.postExecutionStatus}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={
                expandedReverifyReport.keepActiveAllowed
                  ? 'success'
                  : 'warning'
              }
              label={`keepActive=${String(expandedReverifyReport.keepActiveAllowed)}`}
            />
            <Chip
              size="small"
              color={
                expandedReverifyReport.nextVerificationRequired
                  ? 'warning'
                  : 'default'
              }
              label={`reverify=${String(expandedReverifyReport.nextVerificationRequired)}`}
            />
            <Chip
              size="small"
              color={
                expandedReverifyReport.rollbackRecommended
                  ? 'warning'
                  : 'default'
              }
              label={`rollbackRecommended=${String(expandedReverifyReport.rollbackRecommended)}`}
            />
          </div>

          {expandedReverifyReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Expanded reverify blockers:{' '}
              {expandedReverifyReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {expandedHoldPolicyReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Expanded hold"
              chipLabel={expandedHoldPolicyReport.status}
              chipColor={
                expandedHoldPolicyReport.status === 'ready'
                  ? 'success'
                  : expandedHoldPolicyReport.status === 'blocked'
                    ? 'error'
                    : 'warning'
              }
            />
            <DnsTextRow
              label="结果"
              value={expandedHoldPolicyReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedHoldPolicyReport.reason}
            />
            <DnsTextRow
              label="Active age"
              value={
                expandedHoldPolicyReport.activeAgeSeconds == null
                  ? '未读取'
                  : `${expandedHoldPolicyReport.activeAgeSeconds}s`
              }
            />
            <DnsTextRow
              label="Hold window"
              value={`${expandedHoldPolicyReport.minimumHoldSeconds}s - ${expandedHoldPolicyReport.maximumHoldSeconds}s`}
            />
            <DnsTextRow
              label="Recommended"
              value={expandedHoldPolicyReport.recommendedAction}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedHoldPolicyReport.recommendedAction}
            />
            <DnsTextRow
              label="Promotion"
              value={
                expandedHoldPolicyReport.promotionAllowed
                  ? '允许推广'
                  : '不允许长期默认'
              }
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={
                expandedHoldPolicyReport.keepActiveAllowed
                  ? 'success'
                  : 'warning'
              }
              label={`keepActive=${String(expandedHoldPolicyReport.keepActiveAllowed)}`}
            />
            <Chip
              size="small"
              color={
                expandedHoldPolicyReport.nextVerificationRequired
                  ? 'warning'
                  : 'default'
              }
              label={`reverify=${String(expandedHoldPolicyReport.nextVerificationRequired)}`}
            />
            <Chip
              size="small"
              color={
                expandedHoldPolicyReport.rollbackRecommended
                  ? 'warning'
                  : 'default'
              }
              label={`rollbackRecommended=${String(expandedHoldPolicyReport.rollbackRecommended)}`}
            />
          </div>

          {expandedHoldPolicyReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Expanded hold blockers:{' '}
              {expandedHoldPolicyReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {expandedStabilityGateReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Expanded stability"
              chipLabel={expandedStabilityGateReport.status}
              chipColor={
                expandedStabilityGateReport.status === 'ready'
                  ? 'success'
                  : 'error'
              }
            />
            <DnsTextRow
              label="结果"
              value={expandedStabilityGateReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedStabilityGateReport.reason}
            />
            <DnsTextRow
              label="Recommended"
              value={expandedStabilityGateReport.recommendedAction}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedStabilityGateReport.recommendedAction}
            />
            <DnsTextRow
              label="Promotion"
              value={
                expandedStabilityGateReport.promotionAllowed
                  ? '允许推广'
                  : '不允许长期默认'
              }
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={
                expandedStabilityGateReport.keepActiveAllowed
                  ? 'success'
                  : 'warning'
              }
              label={`keepActive=${String(expandedStabilityGateReport.keepActiveAllowed)}`}
            />
            <Chip
              size="small"
              color={
                expandedStabilityGateReport.rollbackRecommended
                  ? 'warning'
                  : 'default'
              }
              label={`rollbackRecommended=${String(expandedStabilityGateReport.rollbackRecommended)}`}
            />
            <Chip
              size="small"
              color="default"
              label={`autoRollout=${String(expandedStabilityGateReport.autoRollout)}`}
            />
          </div>

          {expandedStabilityGateReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Expanded stability blockers:{' '}
              {expandedStabilityGateReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {expandedPostExecutionReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Expanded verification"
              chipLabel={expandedPostExecutionReport.status}
              chipColor={verificationStatusColor(
                expandedPostExecutionReport.status,
              )}
            />
            <DnsTextRow
              label="结果"
              value={expandedPostExecutionReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedPostExecutionReport.reason}
            />
            <DnsTextRow
              label="Observed"
              value={expandedPostExecutionReport.observedEvidence.status}
            />
            <DnsTextRow
              label="Active state"
              value={expandedPostExecutionReport.activeState?.state ?? '未读取'}
            />
            <DnsTextRow
              label="Preflight"
              value={
                expandedPostExecutionReport.preflightRecord
                  ? '已读取'
                  : '未读取'
              }
            />
            <DnsTextRow
              label="Expanded drill"
              value={expandedPostExecutionReport.rollbackDrill.status}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={
                expandedPostExecutionReport.failureAudit.required
                  ? 'warning'
                  : 'success'
              }
              label={`failureAudit=${String(expandedPostExecutionReport.failureAudit.required)}`}
            />
            <Chip
              size="small"
              color={
                expandedPostExecutionReport.rollbackDrill.status === 'ready'
                  ? 'success'
                  : 'error'
              }
              label={`rollbackDrill=${expandedPostExecutionReport.rollbackDrill.status}`}
            />
          </div>

          {expandedPostExecutionReport.failureAudit.reasons.length > 0 ? (
            <div className="rounded-md border border-warning/40 bg-warning/5 px-3 py-2 text-xs text-muted-foreground">
              Expanded failure audit:{' '}
              {expandedPostExecutionReport.failureAudit.reasons.join('；')}
            </div>
          ) : null}

          {expandedPostExecutionReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Expanded verification blockers:{' '}
              {expandedPostExecutionReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {expandedDrillReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Expanded rollback drill"
              chipLabel={expandedDrillReport.status}
              chipColor={rollbackDrillStatusColor(expandedDrillReport.status)}
            />
            <DnsTextRow
              label="结果"
              value={expandedDrillReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedDrillReport.reason}
            />
            <DnsTextRow
              label="Would restore"
              value={expandedDrillReport.wouldRestoreRuntime}
            />
            <DnsTextRow
              label="Auto rollback"
              value={
                expandedDrillReport.autoRollback ? '会自动回滚' : '不会自动回滚'
              }
            />
          </div>

          {expandedDrillReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Expanded drill blockers:{' '}
              {expandedDrillReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {expandedExecutionReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Expanded execution"
              chipLabel={expandedExecutionReport.status}
              chipColor={expandedExecutionStatusColor(
                expandedExecutionReport.status,
              )}
            />
            <DnsTextRow
              label="结果"
              value={expandedExecutionReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedExecutionReport.reason}
            />
            <DnsTextRow
              label="Apply DNS config"
              value={
                expandedExecutionReport.dnsConfigApplied
                  ? '已应用并 reload'
                  : expandedExecutionReport.dnsConfigApplyAttempted
                    ? '已尝试但失败'
                    : '未尝试'
              }
            />
            <DnsTextRow
              label="Active runtime"
              value={
                expandedExecutionReport.activeState?.activeRuntime ?? '未激活'
              }
            />
            <DnsTextRow
              label="Rollback"
              value={
                expandedExecutionReport.rollbackAvailable
                  ? '可显式回滚'
                  : '不可回滚'
              }
            />
            <DnsTextRow
              label="Execution record"
              value={
                expandedExecutionReport.executionRecordPath
                  ? '已持久化'
                  : '未持久化'
              }
              valueTitle={
                expandedExecutionReport.executionRecordPath ?? undefined
              }
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={
                expandedExecutionReport.mutatesRuntime ? 'warning' : 'success'
              }
              label={`mutatesRuntime=${String(expandedExecutionReport.mutatesRuntime)}`}
            />
            <Chip
              size="small"
              color={expandedExecutionReport.reloadMihomo ? 'warning' : 'success'}
              label={`reloadMihomo=${String(expandedExecutionReport.reloadMihomo)}`}
            />
          </div>

          {expandedExecutionReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Execution blockers:{' '}
              {expandedExecutionReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {expandedRollbackReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Expanded rollback"
              chipLabel={expandedRollbackReport.status}
              chipColor={expandedRollbackStatusColor(
                expandedRollbackReport.status,
              )}
            />
            <DnsTextRow
              label="结果"
              value={expandedRollbackReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={expandedRollbackReport.reason}
            />
            <DnsTextRow
              label="Restore DNS config"
              value={
                expandedRollbackReport.dnsConfigRestored
                  ? '已恢复并 reload'
                  : expandedRollbackReport.dnsConfigRestoreAttempted
                    ? '已尝试但失败'
                    : '未尝试'
              }
            />
            <DnsTextRow
              label="Restored runtime"
              value={
                expandedRollbackReport.restoredState?.activeRuntime ?? '未恢复'
              }
            />
          </div>

          {expandedRollbackReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Rollback blockers: {expandedRollbackReport.blockers.join('；')}
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
