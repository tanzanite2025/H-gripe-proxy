import { AlertCircle, AlertTriangle, CheckCircle } from 'lucide-react'

import type { DnsRuntimeViewModel } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

interface RuntimeOptionsSectionProps {
  runtimeView: DnsRuntimeViewModel
}

export function RuntimeOptionsSection({
  runtimeView,
}: RuntimeOptionsSectionProps) {
  return (
    <div className="mb-2">
      <DnsSectionHeading title="DNS 运行态选项" />
      <div className="space-y-1">
        <DnsTextRow label="增强模式" value={runtimeView.enhancedModeLabel} />
        <DnsChipRow
          label="IPv6"
          chipLabel={runtimeView.options.ipv6.label}
          chipColor={runtimeView.options.ipv6.color}
          chipIcon={<CheckCircle className="h-3 w-3" />}
        />
        <DnsChipRow
          label="优先 H3"
          chipLabel={runtimeView.options.preferH3.label}
          chipColor={runtimeView.options.preferH3.color}
          chipIcon={<AlertTriangle className="h-3 w-3" />}
        />
        <DnsChipRow
          label="使用 Hosts"
          chipLabel={runtimeView.options.useHosts.label}
          chipColor={runtimeView.options.useHosts.color}
          chipIcon={<AlertCircle className="h-3 w-3" />}
        />
        <DnsTextRow
          label="使用系统 Hosts"
          value={runtimeView.options.useSystemHosts.label}
          valueClassName="text-sm font-bold text-primary-600 dark:text-primary-400"
        />
        <DnsTextRow
          label="遵循规则"
          value={runtimeView.options.respectRules.label}
          valueTitle={runtimeView.options.respectRules.label}
          valueClassName="max-w-[200px] overflow-hidden text-ellipsis whitespace-nowrap text-sm font-bold"
        />
      </div>
    </div>
  )
}
