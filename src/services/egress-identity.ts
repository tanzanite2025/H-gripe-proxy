import { invoke } from '@tauri-apps/api/core'

export interface EgressPreviewRequest {
  process_name?: string
  exe_path?: string
  shortcut_id?: string
  domain?: string
  source_ip?: string
  source_port?: number
  available_nodes?: string[]
}

export interface ResolvedEgressIdentity {
  assignmentKey?: string | null
  profileId: string
  selectedNode: string
  dnsMode: 'Inherit' | 'Hijack' | 'Remote'
  tlsFingerprint?: string | null
  matchedBy: string
}

export async function egressIdentityPreviewMatch(
  request: EgressPreviewRequest,
): Promise<ResolvedEgressIdentity> {
  return await invoke('egress_identity_preview_match', {
    ...request,
  })
}

export async function egressIdentityAssignMatch(
  request: EgressPreviewRequest,
): Promise<ResolvedEgressIdentity> {
  return await invoke('egress_identity_assign_match', {
    ...request,
  })
}

export async function egressIdentityGetActiveAssignments(): Promise<ResolvedEgressIdentity[]> {
  return await invoke('egress_identity_get_active_assignments')
}

export async function egressIdentityClearAssignment(key: string): Promise<void> {
  await invoke('egress_identity_clear_assignment', { key })
}
