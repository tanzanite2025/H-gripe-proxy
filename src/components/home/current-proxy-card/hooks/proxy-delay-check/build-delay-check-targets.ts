import { extractProxyNames } from '../current-proxy-data/shared'

import type {
  BuildDelayCheckTargetsOptions,
  DelayCheckTargets,
} from './shared'

export function buildDelayCheckTargets({
  currentGroup,
  groupMap,
  proxyRecords,
}: BuildDelayCheckTargetsOptions): DelayCheckTargets {
  const targetGroup = groupMap[currentGroup]
  if (!targetGroup) {
    return {
      providerNames: [],
      proxyNames: [],
    }
  }

  const providerNames = new Set<string>()
  const proxyNames: string[] = []
  const groupNames = extractProxyNames(targetGroup.all).filter(
    (name) => name !== 'DIRECT' && name !== 'REJECT',
  )

  groupNames.forEach((name) => {
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
