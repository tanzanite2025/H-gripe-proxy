import { FlaskConical, ShieldCheck } from 'lucide-react'

import { BasePage } from '@/components/base'
import { EnhancedCard } from '@/components/home/enhanced-card'
import { StrategyPoolRegressionCard } from '@/components/test/strategy-pool-regression-card'

const WebTestPage = () => {
  return (
    <BasePage full title="纯 Web 沙盒" contentStyle={{ padding: 8 }}>
      <div className="space-y-3">
        <EnhancedCard
          title="隔离说明"
          icon={<ShieldCheck className="h-4 w-4" />}
          iconColor="success"
        >
          <div className="text-sm leading-7 text-text-secondary">
            这里不走主应用的 Layout、WindowProvider、AppDataProvider，也不读取
            Verge 配置预加载，只保留纯前端组件回归环境。
          </div>
          <div className="mt-2 text-xs leading-6 text-text-secondary/80">
            适合做策略池、弹窗、卡片布局这类本地交互验证，避免被 Tauri
            状态、内核状态和全局数据链路干扰。
          </div>
        </EnhancedCard>

        <EnhancedCard
          title="当前回归项"
          icon={<FlaskConical className="h-4 w-4" />}
          iconColor="info"
        >
          <div className="text-sm leading-7 text-text-secondary">
            当前已接入策略池的纯内存回归卡片，支持创建、编辑、搜索、勾选和保存验证。
          </div>
        </EnhancedCard>

        <StrategyPoolRegressionCard />
      </div>
    </BasePage>
  )
}

export default WebTestPage
