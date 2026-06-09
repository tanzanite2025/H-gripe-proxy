import type { CurrentProxySource } from '../current-proxy-data/shared'

export interface DelayCheckTargets {
  providerNames: string[]
  proxyNames: string[]
}

export interface BuildDelayCheckTargetsOptions {
  isGlobalMode: boolean
  proxies: CurrentProxySource | undefined
  proxyRecords: Record<string, any>
}
