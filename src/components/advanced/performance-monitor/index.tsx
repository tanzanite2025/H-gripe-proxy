import { RefreshCw } from 'lucide-react'

import { Alert, Button } from '@/components/tailwind'
import type { CoordinatorStatus } from '@/services/coordinator'

import { buildMonitorCards, buildRecommendations } from './helpers'
import { StatusCard } from './status-card'

interface Props {
  status: CoordinatorStatus | null
  onRefresh: () => Promise<CoordinatorStatus | null>
}

export function PerformanceMonitor({ status, onRefresh }: Props) {
  if (!status) {
    return (
      <Alert severity="info" className="text-sm">
        正在加载运行态状态...
      </Alert>
    )
  }

  const cards = buildMonitorCards(status)
  const recommendations = buildRecommendations(status)

  return (
    <div className="space-y-4">
      {status.securityCompromised ? (
        <Alert severity="error" className="text-sm">
          检测到安全状态异常，可能存在调试器、异常扫描或其他风险信号。建议优先检查系统环境、配置来源和当前运行节点。
        </Alert>
      ) : null}

      <div className="flex justify-end">
        <Button
          variant="outline"
          size="sm"
          onClick={() => void onRefresh()}
          startIcon={<RefreshCw className="h-4 w-4" />}
        >
          刷新状态
        </Button>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        {cards.map((card) => (
          <StatusCard key={card.title} card={card} />
        ))}
      </div>

      <section className="rounded-lg border border-border bg-card p-4">
        <h3 className="mb-3 text-sm font-semibold">运行建议</h3>

        <div className="space-y-2">
          {recommendations.map((recommendation, index) => (
            <Alert
              key={`${recommendation.tone}-${index}`}
              severity={recommendation.tone}
              className="text-sm"
            >
              {recommendation.message}
            </Alert>
          ))}
        </div>
      </section>
    </div>
  )
}
