import { useQuery } from '@tanstack/react-query'
import yaml from 'js-yaml'
import { useCallback, useEffect, useRef, useState } from 'react'

import {
  getAdvancedConfig,
  saveAdvancedConfig,
  type ResidentialProxy,
  type ResidentialProxyPool,
} from '@/services/coordinator'

import { type ProxyChainItem } from '../proxy-chain-types'

type ProxyRecord = {
  history?: Array<{
    delay: number
  }>
}

type ProxyRecords = Record<string, ProxyRecord>

interface ParsedChainConfig {
  proxies?: Array<{
    name: string
    type: string
    [key: string]: unknown
  }>
}

const DEFAULT_RESIDENTIAL_POOL: ResidentialProxyPool = {
  enabled: false,
  proxies: [],
}

const toChainItems = (
  parsedConfig: ParsedChainConfig | null | undefined,
): ProxyChainItem[] => {
  const timestamp = Date.now()

  return (
    parsedConfig?.proxies?.map((proxy, index) => ({
      id: `${proxy.name}_${timestamp}_${index}`,
      name: proxy.name,
      type: proxy.type,
      delay: undefined,
    })) || []
  )
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

export function useProxyChainLengthDirtyMarker(
  proxyChainLength: number,
  onMarkUnsavedChanges?: () => void,
) {
  const chainLengthRef = useRef(proxyChainLength)

  useEffect(() => {
    if (
      chainLengthRef.current !== proxyChainLength &&
      chainLengthRef.current !== 0
    ) {
      onMarkUnsavedChanges?.()
    }

    chainLengthRef.current = proxyChainLength
  }, [onMarkUnsavedChanges, proxyChainLength])
}

export function useProxyChainConfigLoader(
  chainConfigData: string | null | undefined,
  onUpdateChain: (chain: ProxyChainItem[]) => void,
) {
  useEffect(() => {
    if (!chainConfigData) {
      return
    }

    try {
      // JSON is valid YAML, so one parser covers both persisted formats.
      const parsedConfig = yaml.load(chainConfigData) as ParsedChainConfig
      const chainItems = toChainItems(parsedConfig)

      if (chainItems.length > 0) {
        onUpdateChain(chainItems)
      }
    } catch (error) {
      console.error('Failed to process chain config data:', error)
    }
  }, [chainConfigData, onUpdateChain])
}

export function useProxyChainDelayUpdater(
  proxyRecords: ProxyRecords | undefined,
  proxyChain: ProxyChainItem[],
  onUpdateChain: (chain: ProxyChainItem[]) => void,
) {
  const proxyChainRef = useRef(proxyChain)
  const onUpdateChainRef = useRef(onUpdateChain)

  useEffect(() => {
    proxyChainRef.current = proxyChain
    onUpdateChainRef.current = onUpdateChain
  }, [onUpdateChain, proxyChain])

  useEffect(() => {
    if (!proxyRecords) {
      return
    }

    const updateDelays = () => {
      const currentChain = proxyChainRef.current
      if (currentChain.length === 0) {
        return
      }

      const updatedChain = currentChain.map((item) => {
        const proxyRecord = proxyRecords[item.name]
        if (proxyRecord?.history && proxyRecord.history.length > 0) {
          const latestDelay =
            proxyRecord.history[proxyRecord.history.length - 1].delay
          return { ...item, delay: latestDelay }
        }
        return item
      })

      const hasChanged = updatedChain.some(
        (item, index) => item.delay !== currentChain[index]?.delay,
      )

      if (hasChanged) {
        onUpdateChainRef.current(updatedChain)
      }
    }

    updateDelays()
    const interval = window.setInterval(updateDelays, 5000)

    return () => window.clearInterval(interval)
  }, [proxyRecords])
}
