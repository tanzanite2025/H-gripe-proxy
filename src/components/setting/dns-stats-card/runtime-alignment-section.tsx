import type { DnsRuntimeViewModel } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading } from './shared'

interface RuntimeAlignmentSectionProps {
  runtimeView: DnsRuntimeViewModel
}

export function RuntimeAlignmentSection({
  runtimeView,
}: RuntimeAlignmentSectionProps) {
  return (
    <div className="mb-2">
      <DnsSectionHeading title="运行态对齐状态" />
      <div className="space-y-1">
        <DnsChipRow
          label="dns_config.yaml"
          chipLabel={runtimeView.dnsConfig.label}
          chipColor={runtimeView.dnsConfig.color}
        />
        <DnsChipRow
          label="DNS 段对齐"
          chipLabel={runtimeView.runtimeDnsAlignment.label}
          chipColor={runtimeView.runtimeDnsAlignment.color}
        />
        <DnsChipRow
          label="Hosts 段对齐"
          chipLabel={runtimeView.runtimeHostsAlignment.label}
          chipColor={runtimeView.runtimeHostsAlignment.color}
        />
        <DnsChipRow
          label="整体运行态"
          chipLabel={runtimeView.runtimeAlignment.label}
          chipColor={runtimeView.runtimeAlignment.color}
        />
      </div>
    </div>
  )
}
