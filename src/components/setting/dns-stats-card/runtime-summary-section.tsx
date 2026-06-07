import type { DnsRuntimeViewModel } from '../dns-runtime-view-model'
import { DnsSectionHeading, DnsTextRow } from './shared'

interface RuntimeSummarySectionProps {
  runtimeView: DnsRuntimeViewModel
}

export function RuntimeSummarySection({
  runtimeView,
}: RuntimeSummarySectionProps) {
  return (
    <div className="mb-2">
      <DnsSectionHeading title="DNS 运行态摘要" />
      <div className="space-y-1">
        <DnsTextRow label="Nameserver 数量" value={runtimeView.nameserverCount} />
        <DnsTextRow
          label="Fallback 数量"
          value={runtimeView.fallbackCount}
          valueClassName="text-sm font-bold text-green-600 dark:text-green-400"
        />
        <DnsTextRow
          label="Policy 组数量"
          value={runtimeView.routing.policyCount}
          valueClassName="text-sm font-bold text-yellow-600 dark:text-yellow-400"
        />
        <DnsTextRow
          label="Default Nameserver 数量"
          value={runtimeView.defaultNameserverCount}
        />
        <DnsTextRow label="当前运行态" value={runtimeView.runtimeDnsInjectedLabel} />
      </div>
    </div>
  )
}
