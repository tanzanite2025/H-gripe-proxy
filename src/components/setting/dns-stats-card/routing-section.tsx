import { Router } from 'lucide-react'

import type { DnsRuntimeViewModel } from '../dns-runtime-view-model'
import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

interface RoutingSectionProps {
  runtimeView: DnsRuntimeViewModel
}

export function RoutingSection({ runtimeView }: RoutingSectionProps) {
  return (
    <div className="mb-2">
      <DnsSectionHeading
        title="DNS 智能分流"
        icon={<Router className="h-3.5 w-3.5" />}
      />
      <div className="space-y-1">
        <DnsChipRow
          label="分流模式"
          chipLabel={runtimeView.routing.modeLabel}
          chipColor={runtimeView.routing.modeColor}
        />
        <DnsTextRow
          label="国内 DNS"
          value={runtimeView.routing.domesticDns}
          valueTitle={runtimeView.routing.domesticDns}
          valueClassName="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
        />
        <DnsTextRow
          label="海外 DNS"
          value={runtimeView.routing.foreignDns}
          valueTitle={runtimeView.routing.foreignDns}
          valueClassName="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
        />
        <DnsTextRow label="策略组数量" value={runtimeView.routing.policyCount} />
      </div>
    </div>
  )
}
