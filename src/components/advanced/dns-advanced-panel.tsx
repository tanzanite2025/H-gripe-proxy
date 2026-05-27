/**
 * DNS 高级功能面板
 * 包含 DNS 统计、DNS 智能分流、DNS 零泄漏防护
 */

import { DnsStatsCard } from '@/components/setting/dns-stats-card'
import { DnsRoutingCard } from '@/components/setting/dns-routing-card'
import { DnsLeakProtectionCard } from '@/components/setting/dns-leak-protection-card'

export function DnsAdvancedPanel() {
  return (
    <div>
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* DNS 统计 */}
        <div>
          <DnsStatsCard />
        </div>

        {/* DNS 智能分流 + DNS 零泄漏防护 */}
        <div className="flex flex-col gap-4">
          <div className="bg-card border border-border rounded-lg p-4">
            <DnsRoutingCard />
          </div>

          <div className="bg-card border border-border rounded-lg p-4">
            <DnsLeakProtectionCard />
          </div>
        </div>
      </div>
    </div>
  )
}
