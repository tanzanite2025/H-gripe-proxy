import { selectNodeForGroup } from 'tauri-plugin-mihomo-api'

import { syncTrayProxySelection } from '@/services/cmds'

interface ProxyRuntimeSelectionOptions {
  syncTray?: boolean
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
