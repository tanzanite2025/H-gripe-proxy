import type { ProxyDisplayGroupKind } from '@/services/proxy-display'

import type { HeadState } from './use-head-state'

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
  items: IRenderItem[]
}

export const GROUP_SECTION_COPY: Record<
  VisibleSectionKind,
  { title: string; description?: string }
> = {
  manual: {
    title: '手动节点',
  },
  strategy: {
    title: '策略池',
    description: '这里只显示你手动加入策略池的成员，不再在池内手动点选单节点。',
  },
}

export const GLOBAL_SELECTOR_SECTION_COPY: Record<
  VisibleSectionKind,
  { title: string; description?: string }
> = {
  manual: {
    title: '单选节点',
    description: '这里是可以直接选中的单一出口节点。',
  },
  strategy: {
    title: '策略池',
    description: '选择这里的策略池后，由池内策略自动决定当前出口节点。',
  },
}

export const MANUAL_PAGE_SECTION_COPY = {
  title: '节点组',
  description: '这里只显示你可以直接决定出口的主组。',
}

export const calculateColumns = (width: number, configCol: number): number => {
  if (configCol > 0 && configCol < 6) return configCol
  if (width > 1920) return 3
  if (width > 1450) return 2
  if (width > 1024) return 2
  if (width > 900) return 2
  if (width >= 600) return 2
  return 1
}

const groupProxies = <T = unknown>(list: T[], size: number): T[][] =>
  list.reduce((acc, item) => {
    const lastGroup = acc[acc.length - 1]
    if (!lastGroup || lastGroup.length >= size) {
      acc.push([item])
    } else {
      lastGroup.push(item)
    }
    return acc
  }, [] as T[][])

export const buildProxyItems = (
  group: ProxyGroup,
  proxies: ProxyItem[],
  col: number,
  sectionKind: VisibleSectionKind,
  headState: HeadState,
): IRenderItem[] => {
  if (!proxies.length) return []

  if (col > 1) {
    return groupProxies(proxies, col).map((proxyCol, colIndex) => ({
      type: 4 as const,
      key: `col-${group.name}-${sectionKind}-${proxyCol[0].name}-${colIndex}`,
      group,
      col,
      proxyCol,
      headState,
      sectionKind,
    }))
  }

  return proxies.map((proxy) => ({
    type: 2 as const,
    key: `${group.name}-${sectionKind}-${proxy.name}`,
    group,
    proxy,
    headState,
    sectionKind,
  }))
}

export const buildDisplayPath = (
  path: Array<string | null | undefined>,
  _records?: Record<string, IProxyItem>,
): string[] => path.map((name) => name?.trim() || '').filter(Boolean)
