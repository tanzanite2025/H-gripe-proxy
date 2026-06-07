import { invoke } from '@tauri-apps/api/core'

export async function securityPolicyApply(name: string) {
  return invoke<number[]>('security_policy_apply', { name })
}

export async function securityPolicyRevoke(name: string) {
  return invoke<void>('security_policy_revoke', { name })
}

export async function securityPolicyApplyAll() {
  return invoke<string[]>('security_policy_apply_all')
}

export async function securityPolicyRevokeAll() {
  return invoke<string[]>('security_policy_revoke_all')
}

export async function securityPolicyGetStates() {
  return invoke<IAppliedPolicyState[]>('security_policy_get_states')
}

export async function securityPolicyGetState(name: string) {
  return invoke<IAppliedPolicyState | null>('security_policy_get_state', {
    name,
  })
}
