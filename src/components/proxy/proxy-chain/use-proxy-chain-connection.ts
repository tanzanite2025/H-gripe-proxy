import { useCallback, useState } from 'react'

import { type ProxyChainItem } from '../proxy-chain-types'
import type { ProxyChainCopy } from './proxy-chain-copy'
import {
  clearProxyChainSelection,
  connectProxyChain,
  disconnectProxyChain,
} from './proxy-chain-connection-runtime'

interface UseProxyChainConnectionOptions {
  isConnected: boolean
  proxyChain: ProxyChainItem[]
  mode?: string
  selectedGroup?: string | null
  onUpdateChain: (chain: ProxyChainItem[]) => void
  refreshProxy: () => Promise<any>
  copy: Pick<
    ProxyChainCopy,
    'connectFailedMessage' | 'disconnectFailedMessage' | 'minimumNodesMessage'
  >
}

export const useProxyChainConnection = ({
  isConnected,
  proxyChain,
  mode,
  selectedGroup,
  onUpdateChain,
  refreshProxy,
  copy,
}: UseProxyChainConnectionOptions) => {
  const [isConnecting, setIsConnecting] = useState(false)

  const handleClearChain = useCallback(() => {
    clearProxyChainSelection(onUpdateChain)
  }, [onUpdateChain])

  const runConnectionTask = useCallback(async (task: () => Promise<void>) => {
    setIsConnecting(true)
    try {
      await task()
    } finally {
      setIsConnecting(false)
    }
  }, [])

  const handleConnect = useCallback(async () => {
    if (isConnected) {
      try {
        await runConnectionTask(() =>
          disconnectProxyChain({
            proxyChain,
            mode,
            selectedGroup,
            refreshProxy,
            onUpdateChain,
          }),
        )
      } catch (error) {
        console.error('Failed to disconnect from proxy chain:', error)
        alert(copy.disconnectFailedMessage)
      }
      return
    }

    if (proxyChain.length < 2) {
      alert(copy.minimumNodesMessage)
      return
    }

    try {
      await runConnectionTask(() =>
        connectProxyChain({
          proxyChain,
          mode,
          selectedGroup,
          refreshProxy,
        }),
      )
    } catch (error) {
      console.error('Failed to connect to proxy chain:', error)
      alert(copy.connectFailedMessage)
    }
  }, [
    copy.connectFailedMessage,
    copy.disconnectFailedMessage,
    copy.minimumNodesMessage,
    isConnected,
    mode,
    onUpdateChain,
    proxyChain,
    refreshProxy,
    runConnectionTask,
    selectedGroup,
  ])

  return {
    isConnecting,
    handleClearChain,
    handleConnect,
  }
}
