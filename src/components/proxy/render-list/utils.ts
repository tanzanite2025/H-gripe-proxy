import type { HeadState } from '../use-head-state'

import type { IRenderItem, ProxyGroup, ProxyItem, VisibleSectionKind } from './types'

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
