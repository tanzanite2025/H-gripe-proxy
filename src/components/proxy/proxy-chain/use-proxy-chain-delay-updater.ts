import { useEffect, useRef } from 'react'

import { type ProxyChainItem } from '../proxy-chain-types'

type ProxyRecord = {
  history?: Array<{
    delay: number
  }>
}

type ProxyRecords = Record<string, ProxyRecord>

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
