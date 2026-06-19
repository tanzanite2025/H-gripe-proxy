import { useCallback, useEffect, useState } from 'react'

import type { IProxyItem } from '@/types/proxy'

import { clearProxyChainRuntimeConfig } from '../../proxy-chain-runtime'
import {
  clearProxyChainStorage,
  loadProxyChainStorage,
  saveProxyChainStorage,
  type ProxyChainItem,
} from '../../proxy-chain-types'

interface DuplicateWarningState {
  open: boolean
  message: string
}

interface UsePersistedProxyChainOptions {
  duplicateWarningMessage: string
}

const getProxyDelay = (proxy: IProxyItem) => {
  if (!proxy.history || proxy.history.length === 0) {
    return undefined
  }

  return proxy.history[proxy.history.length - 1].delay
}

const createProxyChainItem = (proxy: IProxyItem): ProxyChainItem => ({
  id: `${proxy.name}_${Date.now()}`,
  name: proxy.name,
  type: proxy.type,
  delay: getProxyDelay(proxy),
})

export function usePersistedProxyChain({
  duplicateWarningMessage,
}: UsePersistedProxyChainOptions) {
  const [proxyChain, setProxyChain] = useState<ProxyChainItem[]>(
    loadProxyChainStorage,
  )
  const [duplicateWarning, setDuplicateWarning] = useState<DuplicateWarningState>(
    {
      open: false,
      message: '',
    },
  )

  useEffect(() => {
    saveProxyChainStorage(proxyChain)
  }, [proxyChain])

  const addProxyToChain = useCallback(
    (proxy: IProxyItem) => {
      setProxyChain((prev) => {
        if (prev.some((item) => item.name === proxy.name)) {
          setDuplicateWarning({
            open: true,
            message: duplicateWarningMessage,
          })
          return prev
        }

        return [...prev, createProxyChainItem(proxy)]
      })
    },
    [duplicateWarningMessage],
  )

  const handleCloseDuplicateWarning = useCallback(() => {
    setDuplicateWarning({ open: false, message: '' })
  }, [])

  const resetProxyChain = useCallback(() => {
    void clearProxyChainRuntimeConfig()
    clearProxyChainStorage()
    setProxyChain([])
  }, [])

  return {
    addProxyToChain,
    duplicateWarning,
    handleCloseDuplicateWarning,
    proxyChain,
    resetProxyChain,
    setProxyChain,
  }
}
