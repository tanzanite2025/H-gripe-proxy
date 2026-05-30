/**
 * 安全配置页面
 * 集中所有安全相关功能
 */

import { Shield } from 'lucide-react'

import { BasePage } from '@/components/base'
import { EnhancedCard } from '@/components/home/enhanced-card'
import SecurityConfig from '@/components/security'

const SecurityPage = () => {
  return (
    <BasePage title="安全配置">
      <EnhancedCard
        title="安全配置"
        icon={<Shield className="h-5 w-5" />}
        iconColor="error"
      >
        <SecurityConfig />
      </EnhancedCard>
    </BasePage>
  )
}

export default SecurityPage
