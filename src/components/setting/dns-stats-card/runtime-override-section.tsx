import { Shield } from 'lucide-react'

import type { DnsRuntimeViewModel } from '../dns-runtime-view-model'
import { DnsChipRow, DnsSectionHeading, DnsTextRow } from './shared'

interface RuntimeOverrideSectionProps {
  runtimeView: DnsRuntimeViewModel
}

export function RuntimeOverrideSection({
  runtimeView,
}: RuntimeOverrideSectionProps) {
  return (
    <div className="mb-2">
      <DnsSectionHeading
        title="运行态覆盖"
        icon={<Shield className="h-3.5 w-3.5" />}
      />
      <div className="space-y-1">
        <DnsChipRow
          label="覆盖开关"
          chipLabel={runtimeView.runtimeOverride.label}
          chipColor={runtimeView.runtimeOverride.color}
        />
        <DnsTextRow
          label="当前来源"
          value={runtimeView.runtimeSource}
          valueTitle={runtimeView.runtimeSource}
          valueClassName="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
        />
        <DnsChipRow
          label="当前生效情况"
          chipLabel={runtimeView.runtimeEffect.label}
          chipColor={runtimeView.runtimeEffect.color}
        />
        <DnsChipRow
          label="已保存产物"
          chipLabel={runtimeView.savedArtifact.label}
          chipColor={runtimeView.savedArtifact.color}
        />
      </div>
    </div>
  )
}
