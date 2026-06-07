import { RefreshCw } from 'lucide-react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import type { DnsRuntimeStatus } from '@/services/cmds'

import { buildDnsRuntimeViewModel } from '../dns-runtime-view-model'
import { LeakProtectionSection } from './leak-protection-section'
import { PerformanceMetricsSection } from './performance-metrics-section'
import { RoutingSection } from './routing-section'
import { RuntimeAlignmentSection } from './runtime-alignment-section'
import { RuntimeOptionsSection } from './runtime-options-section'
import { RuntimeOverrideSection } from './runtime-override-section'
import { RuntimeSummarySection } from './runtime-summary-section'
import { DnsCardState, DnsDivider } from './shared'

interface Props {
  runtimeStatus?: DnsRuntimeStatus
  runtimeStatusPending: boolean
  onRefresh: () => void
}

const cardTitle = 'DNS 当前运行态统计'

export const DnsStatsCard = ({
  runtimeStatus,
  runtimeStatusPending,
  onRefresh,
}: Props) => {
  if (runtimeStatusPending && !runtimeStatus) {
    return (
      <DnsCardState
        title={cardTitle}
        icon={<RefreshCw className="h-4 w-4" />}
        message="正在加载 DNS 运行态数据..."
        loading
      />
    )
  }

  if (!runtimeStatus) {
    return (
      <DnsCardState
        title={cardTitle}
        icon={<RefreshCw className="h-4 w-4" />}
        message="暂时无法读取后端 DNS 运行态统计。"
      />
    )
  }

  const runtimeView = buildDnsRuntimeViewModel(runtimeStatus)

  return (
    <Card>
      <div className="p-4">
        <div className="mb-2 flex items-center justify-between">
          <div className="flex items-center gap-1 text-sm font-semibold">
            <RefreshCw className="h-4 w-4" />
            {cardTitle}
          </div>
          <Button
            size="small"
            startIcon={<RefreshCw className="h-4 w-4" />}
            onClick={onRefresh}
            disabled={runtimeStatusPending}
          >
            刷新
          </Button>
        </div>

        <div className="mb-2 text-xs text-gray-500 dark:text-gray-400">
          这里展示的是 Rust 后端确认的当前 DNS 运行态统计，不会随着未保存的表单编辑即时变化。
        </div>

        <RuntimeSummarySection runtimeView={runtimeView} />
        <DnsDivider />
        <RuntimeOptionsSection runtimeView={runtimeView} />
        <DnsDivider />
        <RuntimeAlignmentSection runtimeView={runtimeView} />
        <DnsDivider />
        <RoutingSection runtimeView={runtimeView} />
        <DnsDivider />
        <RuntimeOverrideSection runtimeView={runtimeView} />
        <DnsDivider />
        <LeakProtectionSection runtimeView={runtimeView} />
        <DnsDivider />
        <PerformanceMetricsSection />
      </div>
    </Card>
  )
}
