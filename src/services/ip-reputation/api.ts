import { invoke } from '@tauri-apps/api/core'

import type { ResidentialProxy } from '@/services/coordinator'

import type {
  IpMetadataProviderConfig,
  IpMetadataProviderHealthReport,
  IpReputation,
  IpReputationConfig,
  ResidentialProxyVerification,
  RiskRoutingRule,
} from './model'
import {
  normalizeIpReputation,
  normalizeIpReputationConfig,
  normalizeMetadataProviderHealthReport,
  normalizeRiskRoutingRule,
  normalizeResidentialProxyVerification,
  serializeIpReputationConfig,
} from './normalizers'

export async function ipReputationGetConfig(): Promise<IpReputationConfig> {
  return normalizeIpReputationConfig(await invoke('ip_reputation_get_config'))
}

export async function ipReputationUpdateConfig(
  config: IpReputationConfig,
): Promise<void> {
  await invoke('ip_reputation_update_config', {
    config: serializeIpReputationConfig(config),
  })
}

export async function ipReputationCheckIp(ip: string): Promise<IpReputation> {
  return normalizeIpReputation(await invoke('ip_reputation_check_ip', { ip }))
}

export async function ipReputationProbeMetadataProvider(
  providerConfig: IpMetadataProviderConfig,
  targetIp?: string,
): Promise<IpMetadataProviderHealthReport> {
  return normalizeMetadataProviderHealthReport(
    await invoke('ip_reputation_probe_metadata_provider', {
      providerConfig: {
        kind: providerConfig.kind,
        options: providerConfig.options,
      },
      targetIp: targetIp || null,
    }),
  )
}

export async function ipReputationGetPredefinedRules(): Promise<RiskRoutingRule[]> {
  const rules = await invoke<unknown[]>('ip_reputation_get_predefined_rules')
  return rules.map(normalizeRiskRoutingRule)
}

export async function ipReputationSelectNodeForDomain(
  domain: string,
  availableNodes: [string, string][],
): Promise<string> {
  return invoke<string>('ip_reputation_select_node_for_domain', {
    domain,
    availableNodes,
  })
}

export async function ipReputationClearCache(): Promise<void> {
  await invoke('ip_reputation_clear_cache')
}

export async function ipReputationGetCacheStats(): Promise<[number, number]> {
  return invoke<[number, number]>('ip_reputation_get_cache_stats')
}

export async function ipReputationGetCacheEntries(): Promise<IpReputation[]> {
  const entries = await invoke<unknown[]>('ip_reputation_get_cache_entries')
  return entries.map(normalizeIpReputation)
}

export async function ipReputationVerifyResidentialProxy(
  proxy: ResidentialProxy,
): Promise<ResidentialProxyVerification> {
  return normalizeResidentialProxyVerification(
    await invoke('ip_reputation_verify_residential_proxy', { proxy }),
  )
}
