import { closeAllConnections } from 'tauri-plugin-mihomo-api'

import {
  applyProxyRuntimeSelection,
  tryApplyProxyRuntimeSelection,
} from '@/services/proxy-runtime-selection'
import { debugLog } from '@/utils/misc'

import {
  applyProxyChainRuntimeIntent,
  clearProxyChainRuntimeConfig,
} from '../proxy-chain-runtime'
import {
  buildProxyChainRuntimeIntent,
  clearProxyChainStorage,
  loadProxyChainRuntimeGroup,
  proxyChainEntryNode,
  proxyChainTargetGroup,
  saveProxyChainRuntimeSelection,
  type ProxyChainItem,
} from '../proxy-chain-types'

interface ProxyChainRuntimeOptions {
  proxyChain: ProxyChainItem[]
  mode?: string
  selectedGroup?: string | null
  refreshProxy: () => Promise<any>
}

interface DisconnectProxyChainOptions extends ProxyChainRuntimeOptions {
  onUpdateChain: (chain: ProxyChainItem[]) => void
}

export const clearProxyChainSelection = (
  onUpdateChain: (chain: ProxyChainItem[]) => void,
) => {
  void clearProxyChainRuntimeConfig()
  clearProxyChainStorage()
  onUpdateChain([])
}

export const disconnectProxyChain = async ({
  proxyChain,
  mode,
  selectedGroup,
  refreshProxy,
  onUpdateChain,
}: DisconnectProxyChainOptions) => {
  await clearProxyChainRuntimeConfig()

  const targetGroup = proxyChainTargetGroup(
    mode,
    selectedGroup,
    loadProxyChainRuntimeGroup(),
  )

  if (targetGroup) {
    const selectedDirect = await tryApplyProxyRuntimeSelection(
      targetGroup,
      'DIRECT',
    )

    if (!selectedDirect) {
      const entryNode = proxyChainEntryNode(proxyChain)
      if (entryNode) {
        await tryApplyProxyRuntimeSelection(targetGroup, entryNode.name)
      }
    }
  }

  clearProxyChainStorage()
  await closeAllConnections()
  await refreshProxy()
  onUpdateChain([])
}

export const connectProxyChain = async ({
  proxyChain,
  mode,
  selectedGroup,
  refreshProxy,
}: ProxyChainRuntimeOptions) => {
  const intent = buildProxyChainRuntimeIntent(proxyChain, mode, selectedGroup)
  if (!intent) {
    throw new Error('invalid proxy chain intent')
  }

  debugLog('Saving chain config:', intent.runtimePayload)
  await applyProxyChainRuntimeIntent(intent)
  debugLog('Chain configuration saved successfully')

  debugLog(`Connecting to proxy chain, last node: ${intent.exitNode}`)
  await applyProxyRuntimeSelection(intent.targetGroup, intent.exitNode)
  saveProxyChainRuntimeSelection(intent.targetGroup, intent.exitNode)

  void refreshProxy()
  debugLog('Successfully connected to proxy chain')
}
