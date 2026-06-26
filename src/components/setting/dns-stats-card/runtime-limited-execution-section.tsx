import { useLockFn } from 'ahooks'
import { Power, RotateCcw } from 'lucide-react'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import {
  dnsDefaultRuntimeLimitedOptInExecution,
  dnsDefaultRuntimeLimitedRollback,
  type DnsDefaultRuntimeLimitedExecutionStatus,
  type DnsDefaultRuntimeLimitedOptInExecutionReport,
  type DnsDefaultRuntimeLimitedRollbackReport,
  type DnsDefaultRuntimeLimitedRollbackStatus,
} from '@/services/dns-api'
import { showNotice } from '@/services/notice-service'

import type { DnsStatusColor } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

const LIMITED_EXECUTION_DOMAIN = 'example.com'

function executionStatusColor(
  status: DnsDefaultRuntimeLimitedExecutionStatus,
): DnsStatusColor {
  switch (status) {
    case 'executed':
      return 'success'
    case 'blocked':
      return 'error'
  }
}

function rollbackStatusColor(
  status: DnsDefaultRuntimeLimitedRollbackStatus,
): DnsStatusColor {
  switch (status) {
    case 'restored':
      return 'success'
    case 'blocked':
      return 'error'
  }
}

export function RuntimeLimitedExecutionSection() {
  const [executionReport, setExecutionReport] =
    useState<DnsDefaultRuntimeLimitedOptInExecutionReport | null>(null)
  const [rollbackReport, setRollbackReport] =
    useState<DnsDefaultRuntimeLimitedRollbackReport | null>(null)
  const [pending, setPending] = useState<'execute' | 'rollback' | null>(null)

  const handleExecute = useLockFn(async () => {
    setPending('execute')
    try {
      const nextReport = await dnsDefaultRuntimeLimitedOptInExecution(
        undefined,
        LIMITED_EXECUTION_DOMAIN,
        true,
      )
      setExecutionReport(nextReport)
      showNotice.success('默认 DNS runtime limited execution 已完成')
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPending(null)
    }
  })

  const handleRollback = useLockFn(async () => {
    setPending('rollback')
    try {
      const nextReport = await dnsDefaultRuntimeLimitedRollback()
      setRollbackReport(nextReport)
      showNotice.success('默认 DNS runtime rollback 已完成')
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
          title="默认 DNS runtime limited execution"
          icon={<Power className="h-3 w-3" />}
        />
        <div className="flex gap-2">
          <Button
            size="small"
            variant="outlined"
            onClick={handleExecute}
            disabled={pending !== null}
          >
            {pending === 'execute' ? '执行中...' : '显式 limited execution'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            onClick={handleRollback}
            disabled={pending !== null}
          >
            <RotateCcw className="mr-1 h-3 w-3" />
            {pending === 'rollback' ? '回滚中...' : 'Rollback'}
          </Button>
        </div>
      </div>

      <div className="mb-2 text-xs text-muted-foreground">
        实验性危险区：只在 execution guard ready 且持久化 metadata 可验证后，写入
        Rust-owned default DNS runtime active state；不写 active profile、不 reload
        内核、不碰 TUN/协议栈。
      </div>

      {executionReport ? (
        <div className="space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Execution"
              chipLabel={executionReport.status}
              chipColor={executionStatusColor(executionReport.status)}
            />
            <DnsTextRow
              label="结果"
              value={executionReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={executionReport.reason}
            />
          </div>

          <div className="grid gap-2 sm:grid-cols-2">
            <DnsTextRow
              label="Guard"
              value={executionReport.guard.status}
            />
            <DnsTextRow
              label="Active runtime"
              value={executionReport.activeState?.activeRuntime ?? '未激活'}
            />
            <DnsTextRow
              label="Rollback"
              value={executionReport.rollbackAvailable ? '可回滚' : '不可回滚'}
            />
            <DnsTextRow
              label="Reload 内核"
              value={executionReport.reloadMihomo ? '会 reload' : '不会 reload'}
            />
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={executionReport.metadataVerified ? 'success' : 'error'}
              label={`metadataVerified=${String(executionReport.metadataVerified)}`}
            />
            <Chip
              size="small"
              color={executionReport.mutatesRuntime ? 'warning' : 'default'}
              label={`mutatesRuntime=${String(executionReport.mutatesRuntime)}`}
            />
            <Chip
              size="small"
              color={executionReport.executed ? 'success' : 'default'}
              label={`executed=${String(executionReport.executed)}`}
            />
          </div>

          {executionReport.activeStatePath ? (
            <div className="rounded-md border border-gray-200 px-3 py-2 text-xs text-muted-foreground dark:border-gray-700">
              Active state: {executionReport.activeStatePath}
            </div>
          ) : null}

          {executionReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Blockers: {executionReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}

      {rollbackReport ? (
        <div className="mt-2 space-y-2">
          <div className="grid gap-2 sm:grid-cols-2">
            <DnsChipRow
              label="Rollback"
              chipLabel={rollbackReport.status}
              chipColor={rollbackStatusColor(rollbackReport.status)}
            />
            <DnsTextRow
              label="结果"
              value={rollbackReport.reason}
              valueClassName="max-w-[260px] truncate text-right text-xs font-bold"
              valueTitle={rollbackReport.reason}
            />
            <DnsTextRow
              label="Restored runtime"
              value={rollbackReport.restoredState?.activeRuntime ?? '未恢复'}
            />
            <DnsTextRow
              label="Reload 内核"
              value={rollbackReport.reloadMihomo ? '会 reload' : '不会 reload'}
            />
          </div>

          {rollbackReport.blockers.length > 0 ? (
            <div className="rounded-md border border-error/40 bg-error/5 px-3 py-2 text-xs text-muted-foreground">
              Rollback blockers: {rollbackReport.blockers.join('；')}
            </div>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}
