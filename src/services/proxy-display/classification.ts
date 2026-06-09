import type {
  ProxyDisplayGroupKind,
  ProxyDisplaySplit,
  ProxyDisplayTargetKind,
} from './types'
import {
  isBuiltinPolicyName,
  isHiddenProxyName,
  normalizeName,
  normalizeType,
} from './names'

const STRATEGY_GROUP_TYPES = new Set(['urltest', 'loadbalance'])
const AUXILIARY_GROUP_TYPES = new Set(['fallback'])

export const isProxyGroupItem = (
  proxy?: IProxyItem | IProxyGroupItem | null,
): proxy is IProxyGroupItem => Array.isArray(proxy?.all)

export const isStrategyGroupType = (type?: string) =>
  STRATEGY_GROUP_TYPES.has(normalizeType(type))

export const isAuxiliaryGroupType = (type?: string) =>
  AUXILIARY_GROUP_TYPES.has(normalizeType(type))

export const categorizeProxyGroup = (
  group?: Pick<IProxyItem, 'type'> | null,
): ProxyDisplayGroupKind => {
  if (isAuxiliaryGroupType(group?.type)) return 'auxiliary'
  if (isStrategyGroupType(group?.type)) return 'strategy'
  return 'manual'
}

export const categorizeProxyTarget = (
  proxy?: IProxyItem | IProxyGroupItem | null,
): ProxyDisplayTargetKind => {
  if (!proxy) return 'manual'
  return categorizeProxyGroup(proxy)
}

export const splitProxyGroupTargets = (
  group?: IProxyGroupItem | null,
): ProxyDisplaySplit => {
  const split: ProxyDisplaySplit = {
    manual: [],
    strategy: [],
    auxiliary: [],
  }

  if (!group?.all?.length) {
    return split
  }

  group.all.forEach((proxy) => {
    if (isHiddenProxyName(proxy.name) || isBuiltinPolicyName(proxy.name)) {
      return
    }

    const kind = categorizeProxyTarget(proxy)
    split[kind].push(proxy)
  })

  if (split.manual.length || split.strategy.length) {
    const currentAuxiliary = split.auxiliary.find(
      (proxy) => proxy.name === group.now,
    )

    split.auxiliary = currentAuxiliary ? [currentAuxiliary] : []
  }

  return split
}

export const getVisibleTopLevelGroups = (
  groups: IProxyGroupItem[] = [],
): {
  manual: IProxyGroupItem[]
  strategy: IProxyGroupItem[]
  auxiliary: IProxyGroupItem[]
} =>
  groups.reduce(
    (result, group) => {
      result[categorizeProxyGroup(group)].push(group)
      return result
    },
    {
      manual: [] as IProxyGroupItem[],
      strategy: [] as IProxyGroupItem[],
      auxiliary: [] as IProxyGroupItem[],
    },
  )

const dedupeProxyGroupsByName = (
  groups: Array<IProxyGroupItem | null | undefined>,
) => {
  const seen = new Set<string>()

  return groups.filter((group): group is IProxyGroupItem => {
    const name = normalizeName(group?.name)
    if (!name || seen.has(name)) {
      return false
    }

    seen.add(name)
    return true
  })
}

export const getDisplayableTopLevelGroups = ({
  groups = [],
  global,
}: {
  groups?: IProxyGroupItem[]
  global?: IProxyGroupItem | null
}) => {
  const preferredGroups = dedupeProxyGroupsByName(
    groups.filter((group) => normalizeName(group?.name) !== 'GLOBAL'),
  )
  const { manual, strategy } = getVisibleTopLevelGroups(preferredGroups)
  const topLevelGroups = manual.concat(strategy)

  if (topLevelGroups.length > 0) {
    return topLevelGroups
  }

  return dedupeProxyGroupsByName(global ? [global] : [])
}

