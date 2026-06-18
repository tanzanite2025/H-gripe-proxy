import { invoke } from '@tauri-apps/api/core'

import type { RuleProviders, Rules } from '@/types/mihomo'

export async function getRuntimeRules() {
  return invoke<Rules>('get_runtime_rules')
}

export async function getRuntimeRuleProviders() {
  return invoke<RuleProviders>('get_runtime_rule_providers')
}

export async function disableRuntimeRules(payload: Record<number, boolean>) {
  await invoke<void>('disable_runtime_rules', { payload })
}

export async function deleteRuntimeRule(index: number) {
  await invoke<void>('delete_runtime_rule', { index })
}

export async function createRuntimeRule(
  ruleType: string,
  payload: string,
  proxy: string,
  source?: string,
  subRule?: string,
  position?: string,
) {
  return invoke<number>('create_runtime_rule', {
    ruleType,
    payload,
    proxy,
    source,
    subRule,
    position,
  })
}

export async function updateRuntimeRuleProvider(providerName: string) {
  await invoke<void>('update_runtime_rule_provider', { providerName })
}
