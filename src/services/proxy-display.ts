import type { CurrentEgressIdentity } from '@/services/cmds/diagnostics'

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

const STRATEGY_GROUP_TYPES = new Set(['urltest', 'loadbalance'])
const AUXILIARY_GROUP_TYPES = new Set(['fallback'])

const normalizeType = (type?: string) => type?.trim().toLowerCase() || ''

const normalizeName = (value?: string | null) => value?.trim() || ''

const collectProxyNames = (
  group?: { all?: Array<string | { name?: string }> } | null,
): string[] =>
  Array.isArray(group?.all)
    ? group.all
        .map((item) =>
          typeof item === 'string'
            ? normalizeName(item)
            : normalizeName(item?.name),
        )
        .filter((name) => name.length > 0)
    : []

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

export const buildProxyDisplayOptions = (
  group?: IProxyGroupItem | null,
): ProxyDisplayOption[] => {
  const split = splitProxyGroupTargets(group)
  const options: ProxyDisplayOption[] = []

  split.manual.forEach((proxy) => {
    options.push({ name: proxy.name, kind: 'manual' })
  })

  split.strategy.forEach((proxy) => {
    options.push({ name: proxy.name, kind: 'strategy' })
  })

  return options
}

export const buildProxyDisplayOptionsFromNames = ({
  names,
  records,
}: {
  names: string[]
  records?: Record<string, IProxyItem>
}): ProxyDisplayOption[] => {
  const manual: ProxyDisplayOption[] = []
  const strategy: ProxyDisplayOption[] = []

  names.forEach((name) => {
    const option: ProxyDisplayOption = {
      name,
      kind: categorizeProxyTarget(records?.[name]),
    }

    if (option.kind === 'strategy') {
      strategy.push(option)
      return
    }

    if (option.kind === 'auxiliary') {
      return
    }

    manual.push(option)
  })

  return manual.concat(strategy)
}

export const isAuxiliarySelectionName = (
  name?: string | null,
  records?: Record<string, IProxyItem>,
) => {
  const normalizedName = normalizeName(name)
  if (!normalizedName) return false
  return categorizeProxyTarget(records?.[normalizedName]) === 'auxiliary'
}

export const pickPreferredProxyNameFromNames = ({
  names,
  records,
  candidateName,
}: {
  names: string[]
  records?: Record<string, IProxyItem>
  candidateName?: string | null
}) => {
  const normalizedCandidateName = normalizeName(candidateName)
  const options = buildProxyDisplayOptionsFromNames({
    names: Array.from(new Set(names.map((name) => normalizeName(name)).filter(Boolean))),
    records,
  })

  if (
    normalizedCandidateName &&
    options.some((option) => option.name === normalizedCandidateName)
  ) {
    return normalizedCandidateName
  }

  return options[0]?.name || ''
}

export const pickPreferredProxyNameFromGroup = (
  group?: IProxyGroupItem | { all?: Array<string | { name?: string }>; now?: string } | null,
  records?: Record<string, IProxyItem>,
  candidateName?: string | null,
) =>
  pickPreferredProxyNameFromNames({
    names: collectProxyNames(group),
    records,
    candidateName: candidateName ?? group?.now,
  })

export const findMatchPolicyName = (rules?: any[]): string => {
  if (!Array.isArray(rules)) return ''

  for (let index = rules.length - 1; index >= 0; index -= 1) {
    const rule = rules[index]
    if (!rule) continue

    if (
      typeof rule?.type === 'string' &&
      rule.type.toUpperCase() === 'MATCH' &&
      typeof rule?.proxy === 'string'
    ) {
      return normalizeName(rule.proxy)
    }
  }

  return ''
}

export const getPreferredProxyGroupName = ({
  proxies,
  rules,
  isGlobalMode,
}: {
  proxies: any
  rules?: any[]
  isGlobalMode?: boolean
}): string => {
  if (isGlobalMode) return 'GLOBAL'
  if (!proxies) return ''

  const matchPolicyName = findMatchPolicyName(rules)
  if (matchPolicyName) {
    const matchGroup =
      proxies.groups?.find((group: { name?: string }) => group?.name === matchPolicyName) ||
      (proxies.global?.name === matchPolicyName ? proxies.global : null) ||
      proxies.records?.[matchPolicyName]

    if (matchGroup && categorizeProxyGroup(matchGroup) !== 'auxiliary') {
      return matchPolicyName
    }
  }

  const manualGroup = (proxies.groups || []).find(
    (group: IProxyGroupItem) => categorizeProxyGroup(group) === 'manual',
  )
  if (manualGroup?.name) {
    return manualGroup.name
  }

  const strategyGroup = (proxies.groups || []).find(
    (group: IProxyGroupItem) => categorizeProxyGroup(group) === 'strategy',
  )
  return normalizeName(strategyGroup?.name)
}

export const getIdentityProxyChain = (
  identity?: CurrentEgressIdentity | null,
): string[] => {
  if (!identity || identity.source !== 'mihomoEgressStatus') {
    return []
  }

  const chain = Array.isArray(identity.proxy_chain)
    ? identity.proxy_chain
        .map((item) => normalizeName(item))
        .filter((item) => item.length > 0)
    : []

  if (chain.length > 0) {
    return chain
  }

  const proxyName = normalizeName(identity.proxy_name)
  return proxyName ? [proxyName] : []
}

export const analyzeProxyPath = (
  path: string[],
  records?: Record<string, IProxyItem>,
): ProxyPathAnalysis => {
  const uniquePath = path
    .map((item) => normalizeName(item))
    .filter((item) => item.length > 0)

  return uniquePath.reduce<ProxyPathAnalysis>(
    (analysis, name) => {
      const record = records?.[name]

      if (record) {
        if (isStrategyGroupType(record.type)) {
          analysis.hasStrategyGroup = true
        }

        if (isAuxiliaryGroupType(record.type)) {
          analysis.hasAuxiliaryGroup = true
        }
      }

      if (analysis.path.includes(name)) {
        analysis.cycleDetected = true
        return analysis
      }

      analysis.path.push(name)
      analysis.leafName = name
      return analysis
    },
    {
      path: [],
      leafName: null,
      hasStrategyGroup: false,
      hasAuxiliaryGroup: false,
      cycleDetected: false,
    },
  )
}

export const resolveProxyPath = (
  records: Record<string, IProxyItem> | undefined,
  startName?: string | null,
): ProxyPathAnalysis => {
  const path: string[] = []
  const visited = new Set<string>()
  let currentName = normalizeName(startName)
  let hasStrategyGroup = false
  let hasAuxiliaryGroup = false
  let cycleDetected = false

  while (currentName) {
    if (visited.has(currentName)) {
      cycleDetected = true
      break
    }

    path.push(currentName)
    visited.add(currentName)

    const currentRecord = records?.[currentName]
    if (currentRecord) {
      if (isStrategyGroupType(currentRecord.type)) {
        hasStrategyGroup = true
      }

      if (isAuxiliaryGroupType(currentRecord.type)) {
        hasAuxiliaryGroup = true
      }
    }

    if (!isProxyGroupItem(currentRecord) || !currentRecord.now) {
      break
    }

    const nextName = normalizeName(currentRecord.now)
    if (!nextName || nextName === currentName) {
      break
    }

    currentName = nextName
  }

  return {
    path,
    leafName: path[path.length - 1] || null,
    hasStrategyGroup,
    hasAuxiliaryGroup,
    cycleDetected,
  }
}
