import { AlertTriangle, CheckCircle, Shield } from 'lucide-react'

import { Chip } from '@/components/tailwind/Chip'

import type { DnsRuntimeViewModel } from '../dns-runtime-view-model'

import { DnsChipRow, DnsSectionHeading } from './shared'

interface LeakProtectionSectionProps {
  runtimeView: DnsRuntimeViewModel
}

export function LeakProtectionSection({
  runtimeView,
}: LeakProtectionSectionProps) {
  return (
    <div>
      <DnsSectionHeading
        title="泄漏防护"
        icon={<Shield className="h-3.5 w-3.5" />}
      />
      <div className="space-y-1">
        <DnsChipRow
          label="防护级别"
          chipLabel={runtimeView.leak.levelLabel}
          chipColor={runtimeView.leak.levelColor}
        />
        <DnsChipRow
          label="安全等级"
          chipLabel={runtimeView.leak.securityLabel}
          chipColor={runtimeView.leak.securityColor}
        />
        <div className="flex items-center justify-between">
          <div className="text-sm">安全状态</div>
          {runtimeView.leak.safe === null ? (
            <Chip label="未知" size="small" color="default" />
          ) : runtimeView.leak.safe ? (
            <Chip
              icon={<CheckCircle className="h-3 w-3" />}
              label="安全"
              size="small"
              color="success"
            />
          ) : (
            <Chip
              icon={<AlertTriangle className="h-3 w-3" />}
              label="不安全"
              size="small"
              color="error"
            />
          )}
        </div>
      </div>
    </div>
  )
}
