import { closeAllRuntimeConnections } from '@/services/connection-runtime'
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
  selectedGroup,
  refreshProxy,
  onUpdateChain,
}: DisconnectProxyChainOptions) => {
  await clearProxyChainRuntimeConfig()

  const targetGroup = proxyChainTargetGroup(
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
  await closeAllRuntimeConnections()
  await refreshProxy()
  onUpdateChain([])
}

export const connectProxyChain = async ({
  proxyChain,
  selectedGroup,
  refreshProxy,
}: ProxyChainRuntimeOptions) => {
  const intent = buildProxyChainRuntimeIntent(proxyChain, selectedGroup)
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
