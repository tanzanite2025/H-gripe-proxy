import { extractProxyNames } from '../current-proxy-data/shared'

import type {
  BuildDelayCheckTargetsOptions,
  DelayCheckTargets,
} from './shared'

export function buildDelayCheckTargets({
  isGlobalMode,
  proxies,
  proxyRecords,
}: BuildDelayCheckTargetsOptions): DelayCheckTargets {
  if (!isGlobalMode || !proxies?.global) {
    return {
      providerNames: [],
      proxyNames: [],
    }
  }

  const providerNames = new Set<string>()
  const proxyNames: string[] = []
  const globalNames = extractProxyNames(proxies.global.all).filter(
    (name) => name !== 'DIRECT' && name !== 'REJECT',
  )

  globalNames.forEach((name) => {
    const proxy = proxyRecords[name]
    if (proxy?.provider) {
      providerNames.add(proxy.provider)
    } else {
      proxyNames.push(name)
    }
  })

  return {
    providerNames: [...providerNames],
    proxyNames,
  }
}
