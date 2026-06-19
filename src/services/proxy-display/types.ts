import type { IProxyItem } from '@/types/proxy'
export type ProxyDisplayGroupKind = 'manual' | 'strategy' | 'auxiliary'
export type ProxyDisplayTargetKind = ProxyDisplayGroupKind

export interface ProxyDisplaySplit {
  manual: IProxyItem[]
  strategy: IProxyItem[]
  auxiliary: IProxyItem[]
}

export interface ProxyDisplayOption {
  name: string
  kind: ProxyDisplayTargetKind
}

export interface ProxyPathAnalysis {
  path: string[]
  leafName: string | null
  hasStrategyGroup: boolean
  hasAuxiliaryGroup: boolean
  cycleDetected: boolean
}

