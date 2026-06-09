import { useQuery } from '@tanstack/react-query'
import { useCallback, useState } from 'react'

import {
  getAdvancedConfig,
  saveAdvancedConfig,
  type ResidentialProxy,
  type ResidentialProxyPool,
} from '@/services/coordinator'

import { type ProxyChainItem } from '../proxy-chain-types'

const DEFAULT_RESIDENTIAL_POOL: ResidentialProxyPool = {
  enabled: false,
  proxies: [],
}

export function useResidentialPoolState(
  proxyChain: ProxyChainItem[],
  onUpdateChain: (chain: ProxyChainItem[]) => void,
  onMarkUnsavedChanges?: () => void,
) {
  const [residentialConfigOpen, setResidentialConfigOpen] = useState(false)
  const { data: advancedConfig } = useQuery({
    queryKey: ['advancedConfig'],
    queryFn: getAdvancedConfig,
    staleTime: 30_000,
  })

  const residentialPool =
    advancedConfig?.residential_pool ?? DEFAULT_RESIDENTIAL_POOL
  const enabledResidentialProxies = residentialPool.enabled
    ? residentialPool.proxies.filter((proxy) => proxy.enabled)
    : []

  const [localResidentialPool, setLocalResidentialPool] =
    useState<ResidentialProxyPool>(residentialPool)

  const addResidentialExit = useCallback(
    (proxy: ResidentialProxy) => {
      const residentialName = `VERGE-RES-${proxy.name}`
      if (proxyChain.some((item) => item.name === residentialName)) {
        return
      }

      onUpdateChain([
        ...proxyChain,
        {
          id: `${residentialName}_${Date.now()}`,
          name: residentialName,
          type: proxy.proxyType,
        },
      ])
      onMarkUnsavedChanges?.()
    },
    [onMarkUnsavedChanges, onUpdateChain, proxyChain],
  )

  const openResidentialConfig = useCallback(() => {
    setLocalResidentialPool(residentialPool)
    setResidentialConfigOpen(true)
  }, [residentialPool])

  const saveResidentialPool = useCallback(async () => {
    try {
      const fullConfig = await getAdvancedConfig()
      fullConfig.residential_pool = localResidentialPool
      await saveAdvancedConfig(fullConfig)
      setResidentialConfigOpen(false)
    } catch (error) {
      console.error('Failed to save residential pool config:', error)
    }
  }, [localResidentialPool])

  return {
    residentialPool,
    enabledResidentialProxies,
    localResidentialPool,
    residentialConfigOpen,
    setLocalResidentialPool,
    setResidentialConfigOpen,
    addResidentialExit,
    openResidentialConfig,
    saveResidentialPool,
  }
}
