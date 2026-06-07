import type {
  AppEgressRule,
  EgressIdentityProfile,
  ShortcutEgressRule,
} from '@/services/coordinator'

export const starterProfile: EgressIdentityProfile = {
  id: 'stable-default',
  name: '稳定默认画像',
  enabled: true,
  preferred_nodes: [],
  preferred_pools: ['通用池'],
  required_ip_type: null,
  max_fraud_score: 70,
  dns_policy: {
    mode: 'Inherit',
    force_remote_dns: false,
  },
  tls_fingerprint: null,
  session_policy: {
    strict_affinity: false,
    ttl_override: null,
  },
  failover_policy: 'Manual',
  allowed_nodes: [],
  strict_node_scope: false,
  use_residential_chain: false,
  residential_proxy_name: null,
  description: '默认的稳定出口身份画像。',
}

export const starterAppRule: AppEgressRule = {
  process_name: 'Steam.exe',
  exe_path: null,
  domains: [],
  profile_id: 'stable-default',
  priority: 100,
  enabled: true,
}

export const starterShortcutRule: ShortcutEgressRule = {
  shortcut_id: 'chatgpt',
  profile_id: 'stable-default',
  enabled: true,
}

export const buildProfileId = (existingIds: string[]) => {
  let index = existingIds.length + 1
  let candidate = `profile-${index}`

  while (existingIds.includes(candidate)) {
    index += 1
    candidate = `profile-${index}`
  }

  return candidate
}
