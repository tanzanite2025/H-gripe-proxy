import yaml from 'js-yaml'
import { useEffect } from 'react'

import { type ProxyChainItem } from '../proxy-chain-types'

interface ParsedChainConfig {
  proxies?: Array<{
    name: string
    type: string
    [key: string]: unknown
  }>
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
