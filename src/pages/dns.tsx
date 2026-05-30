/**
 * DNS 配置页面
 * 独立的 DNS 配置管理页面
 */

import { Network } from 'lucide-react'

import { BasePage } from '@/components/base'
import { EnhancedCard } from '@/components/home/enhanced-card'
import SettingDns from '@/components/setting/setting-dns'

const DnsPage = () => {
  return (
    <BasePage title="DNS 配置">
      <EnhancedCard
        title="DNS 配置"
        icon={<Network className="h-5 w-5" />}
        iconColor="primary"
      >
        <SettingDns />
      </EnhancedCard>
    </BasePage>
  )
}

export default DnsPage
