import type { CurrentProxySource } from '../current-proxy-data/shared'

export interface DelayCheckTargets {
  providerNames: string[]
  proxyNames: string[]
}

export interface BuildDelayCheckTargetsOptions {
  currentGroup: string
  groupMap: Record<string, { all: string[] } | undefined>
  proxyRecords: Record<string, any>
}
