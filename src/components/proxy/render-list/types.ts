import type { ProxyDisplayGroupKind } from '@/services/proxy-display'
import type { IProxyGroupItem, IProxyItem } from '@/types/proxy'

import type { HeadState } from '../use-head-state'

export type ProxyItem = IProxyItem & Record<string, any>
export type ProxyGroup = IProxyGroupItem
export type VisibleSectionKind = Exclude<ProxyDisplayGroupKind, 'auxiliary'>

export interface IRenderItem {
  type: 0 | 1 | 2 | 3 | 4 | 5
  key: string
  group?: ProxyGroup
  proxy?: ProxyItem
  memberCount?: number
  col?: number
  proxyCol?: ProxyItem[]
  headState?: HeadState
  pathText?: string
  sectionTitle?: string
  sectionDescription?: string
  sectionKind?: VisibleSectionKind | 'runtime'
  runtimePath?: string[]
  runtimeObserved?: boolean
  runtimeDescription?: string
}

export type GroupCache = {
  now: string | undefined
  all: IProxyItem[]
  headState: HeadState
  col: number
  latencyTimeout: number | undefined
  pathSignature: string
  strategyMembersSignature?: string
  managedStrategySignature?: string
  items: IRenderItem[]
}
