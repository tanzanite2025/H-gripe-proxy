import { useLockFn } from 'ahooks'
import { useState } from 'react'

import { Card } from '@/components/tailwind/Card'
import { Switch } from '@/components/tailwind/Switch'
import { Button, Stack } from '@/components/tailwind'

import {
  type BlackholeBreakerConfig,
  type BreakerRuntimeState,
  blackholeBreakerGetStates,
  blackholeBreakerResetRule,
  blackholeBreakerTripRule,
  blackholeBreakerUpdateConfig,
  getBreakerStateText,
  getBreakerStateColor,
  getBreakerStateBg,
} from '@/services/blackhole-breaker'

interface Props {
  config: BlackholeBreakerConfig
  onChange: (config: BlackholeBreakerConfig) => void
}

export function BlackholeBreakerPanel({ config, onChange }: Props) {
  const [states, setStates] = useState<BreakerRuntimeState[]>([])
  const [loading, setLoading] = useState(false)

  const refreshStates = useLockFn(async () => {
    setLoading(true)
    try {
      const s = await blackholeBreakerGetStates()
      setStates(s)
    } finally {
      setLoading(false)
    }
  })

  const handleToggleEnabled = (enabled: boolean) => {
    onChange({ ...config, enabled })
  }

  const handleToggleRule = (ruleId: string, enabled: boolean) => {
    const rules = config.rules.map((r) =>
      r.id === ruleId ? { ...r, enabled } : r
    )
    onChange({ ...config, rules })
  }

  const handleResetRule = useLockFn(async (ruleId: string) => {
    await blackholeBreakerResetRule(ruleId)
    await refreshStates()
  })

  const handleTripRule = useLockFn(async (ruleId: string) => {
    await blackholeBreakerTripRule(ruleId)
    await refreshStates()
  })

  const handleSaveConfig = useLockFn(async () => {
    await blackholeBreakerUpdateConfig(config)
    await refreshStates()
  })

  const getState = (ruleId: string): BreakerRuntimeState | undefined =>
    states.find((s) => s.ruleId === ruleId)

  const formatTime = (ts: number | null): string => {
    if (!ts) return '-'
    return new Date(ts * 1000).toLocaleTimeString()
  }

  return (
    <Stack spacing={2}>
      {/* 全局开关 */}
      <Card>
        <div className="p-4">
          <div className="text-base font-semibold">黑洞熔断器</div>
          <div className="text-xs text-gray-500 mb-3">
            异常流量自动熔断至 REJECT-DROP（黑洞）
          </div>
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium">启用熔断器</div>
              <div className="text-xs text-gray-500">
                当出口节点/域名异常指标超阈值时，自动将流量导向黑洞
              </div>
            </div>
            <Switch
              checked={config.enabled}
              onCheckedChange={handleToggleEnabled}
            />
          </div>
        </div>
      </Card>

      {/* 熔断规则列表 */}
      <Card>
        <div className="p-4">
          <div className="text-base font-semibold mb-1">熔断规则</div>
          <div className="text-xs text-gray-500 mb-3">
            按域名/节点匹配，触发条件可自定义
          </div>
          <div className="space-y-3">
          {config.rules.map((rule) => {
            const state = getState(rule.id)
            const stateVal = state?.state ?? 'Closed'

            return (
              <div
                key={rule.id}
                className={`rounded-lg border p-3 ${getBreakerStateBg(stateVal)}`}
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2">
                    <Switch
                      checked={rule.enabled}
                      onCheckedChange={(v: boolean) => handleToggleRule(rule.id, v)}
                    />
                    <span className="text-sm font-medium">{rule.id}</span>
                    <span className={`text-xs font-bold ${getBreakerStateColor(stateVal)}`}>
                      {getBreakerStateText(stateVal)}
                    </span>
                  </div>
                  <Stack direction="row" spacing={1}>
                    <Button
                      variant="outlined"
                      size="small"
                      onClick={() => handleTripRule(rule.id)}
                    >
                      手动熔断
                    </Button>
                    <Button
                      variant="outlined"
                      size="small"
                      onClick={() => handleResetRule(rule.id)}
                    >
                      重置
                    </Button>
                  </Stack>
                </div>

                <div className="mt-2 text-xs text-gray-400">{rule.description}</div>

                {/* 匹配目标 */}
                <div className="mt-2 flex flex-wrap gap-1">
                  {rule.domain_patterns.map((p) => (
                    <span
                      key={p}
                      className="rounded bg-blue-500/10 px-1.5 py-0.5 text-xs text-blue-400"
                    >
                      {p}
                    </span>
                  ))}
                  {rule.node_patterns.map((p) => (
                    <span
                      key={p}
                      className="rounded bg-purple-500/10 px-1.5 py-0.5 text-xs text-purple-400"
                    >
                      node:{p}
                    </span>
                  ))}
                </div>

                {/* 触发条件 */}
                <div className="mt-2 grid grid-cols-2 gap-x-4 gap-y-1 text-xs text-gray-400">
                  <span>连续失败 ≥ {rule.trigger.consecutive_failures}</span>
                  <span>失败率 ≥ {(rule.trigger.failure_rate * 100).toFixed(0)}%</span>
                  <span>窗口 {rule.trigger.window_secs}s / 最少 {rule.trigger.min_requests} 次</span>
                  <span>冷却 {rule.cooldown_secs}s / 探测 {rule.probe_success_count} 次</span>
                  {rule.trigger.max_fraud_score != null && (
                    <span>欺诈评分 ≥ {rule.trigger.max_fraud_score}</span>
                  )}
                </div>

                {/* 运行时状态 */}
                {state && (
                  <div className="mt-2 grid grid-cols-2 gap-x-4 gap-y-1 text-xs text-gray-500">
                    <span>连续失败: {state.consecutiveFailures}</span>
                    <span>
                      窗口: {state.windowFailures}/{state.windowTotal}
                    </span>
                    <span>触发次数: {state.tripCount}</span>
                    <span>上次变更: {formatTime(state.lastStateChange)}</span>
                    {state.openedAt && (
                      <span>熔断于: {formatTime(state.openedAt)}</span>
                    )}
                    {stateVal === 'HalfOpen' && (
                      <span>
                        探测: {state.probeSuccesses}/{rule.probe_success_count}
                      </span>
                    )}
                  </div>
                )}
              </div>
            )
          })}
          </div>
        </div>
      </Card>

      {/* 操作按钮 */}
      <div className="flex gap-2">
        <Button variant="outlined" onClick={refreshStates} loading={loading}>
          刷新状态
        </Button>
        <Button variant="outlined" onClick={handleSaveConfig}>
          保存配置
        </Button>
      </div>
    </Stack>
  )
}
