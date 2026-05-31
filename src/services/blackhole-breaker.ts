/**
 * 黑洞熔断器服务
 */

import { invoke } from '@tauri-apps/api/core'

// ── 类型定义 ──────────────────────────────────────────────────────

export type BreakerState = 'Closed' | 'Open' | 'HalfOpen'

export interface BreakerTrigger {
  consecutive_failures: number
  failure_rate: number
  window_secs: number
  min_requests: number
  max_fraud_score: number | null
}

export interface BreakerRule {
  id: string
  enabled: boolean
  domain_patterns: string[]
  node_patterns: string[]
  trigger: BreakerTrigger
  cooldown_secs: number
  probe_success_count: number
  description: string
}

export interface BlackholeBreakerConfig {
  enabled: boolean
  rules: BreakerRule[]
  default_cooldown_secs: number
  default_probe_success_count: number
}

export interface BreakerRuntimeState {
  ruleId: string
  state: BreakerState
  consecutiveFailures: number
  windowTotal: number
  windowFailures: number
  windowStart: number | null
  openedAt: number | null
  probeSuccesses: number
  probeFailures: number
  tripCount: number
  lastStateChange: number
}

// ── 命令绑定 ──────────────────────────────────────────────────────

export async function blackholeBreakerGetConfig(): Promise<BlackholeBreakerConfig> {
  return invoke('blackhole_breaker_get_config')
}

export async function blackholeBreakerUpdateConfig(config: BlackholeBreakerConfig): Promise<void> {
  return invoke('blackhole_breaker_update_config', { config })
}

export async function blackholeBreakerGetStates(): Promise<BreakerRuntimeState[]> {
  return invoke('blackhole_breaker_get_states')
}

export async function blackholeBreakerRecordResult(ruleId: string, success: boolean): Promise<void> {
  return invoke('blackhole_breaker_record_result', { ruleId, success })
}

export async function blackholeBreakerShouldBlockDomain(domain: string): Promise<boolean> {
  return invoke('blackhole_breaker_should_block_domain', { domain })
}

export async function blackholeBreakerResetRule(ruleId: string): Promise<void> {
  return invoke('blackhole_breaker_reset_rule', { ruleId })
}

export async function blackholeBreakerTripRule(ruleId: string): Promise<void> {
  return invoke('blackhole_breaker_trip_rule', { ruleId })
}

export async function blackholeBreakerRecordFraudScore(
  domain: string,
  fraudScore: number
): Promise<void> {
  return invoke('blackhole_breaker_record_fraud_score', { domain, fraudScore })
}

// ── 显示辅助 ──────────────────────────────────────────────────────

export function getBreakerStateText(state: BreakerState): string {
  switch (state) {
    case 'Closed': return '闭合（正常）'
    case 'Open': return '熔断（黑洞）'
    case 'HalfOpen': return '半开（探测）'
  }
}

export function getBreakerStateColor(state: BreakerState): string {
  switch (state) {
    case 'Closed': return 'text-green-500'
    case 'Open': return 'text-red-500'
    case 'HalfOpen': return 'text-yellow-500'
  }
}

export function getBreakerStateBg(state: BreakerState): string {
  switch (state) {
    case 'Closed': return 'bg-green-500/10 border-green-500/30'
    case 'Open': return 'bg-red-500/10 border-red-500/30'
    case 'HalfOpen': return 'bg-yellow-500/10 border-yellow-500/30'
  }
}
