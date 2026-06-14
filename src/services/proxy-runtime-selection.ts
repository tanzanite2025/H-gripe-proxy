import { invoke } from '@tauri-apps/api/core'
import { selectNodeForGroup } from 'tauri-plugin-mihomo-api'

import { syncTrayProxySelection } from '@/services/cmds'

interface ProxyRuntimeSelectionOptions {
  syncTray?: boolean
}

export type NodeSelectionPlanStatus = 'ready' | 'noop' | 'rejected'

export interface NodeSelectionCandidateInput {
  name: string
  proxyType?: string | null
  alive?: boolean | null
  delayMs?: number | null
}

export interface NodeSelectionPlanRequest {
  groupName: string
  groupType?: string | null
  current?: string | null
  requested?: string | null
  toleranceMs?: number | null
  candidates?: NodeSelectionCandidateInput[]
}

export interface NodeSelectionCandidatePlan extends NodeSelectionCandidateInput {
  eligible: boolean
  reason: string
}

export interface NodeSelectionPlan {
  status: NodeSelectionPlanStatus
  reason: string
  groupName: string
  groupType: string
  selected: string | null
  current: string | null
  shouldApplyRuntime: boolean
  shouldSyncTray: boolean
  candidates: NodeSelectionCandidatePlan[]
}

export async function planNodeSelection(request: NodeSelectionPlanRequest) {
  return invoke<NodeSelectionPlan>('plan_node_selection', { request })
}

export async function applyProxyRuntimeSelection(
  groupName: string,
  proxyName: string,
  options: ProxyRuntimeSelectionOptions = {},
) {
  await selectNodeForGroup(groupName, proxyName)

  if (options.syncTray ?? true) {
    await syncTrayProxySelection()
  }
}

export async function tryApplyProxyRuntimeSelection(
  groupName: string,
  proxyName: string,
  options: ProxyRuntimeSelectionOptions = {},
) {
  try {
    await applyProxyRuntimeSelection(groupName, proxyName, options)
    return true
  } catch {
    return false
  }
}
