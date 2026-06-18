import { invoke } from '@tauri-apps/api/core'
import type { RuleProviders, Rules } from 'tauri-plugin-mihomo-api'

export async function getRuntimeRules() {
  return invoke<Rules>('get_runtime_rules')
}

export async function getRuntimeRuleProviders() {
  return invoke<RuleProviders>('get_runtime_rule_providers')
}
