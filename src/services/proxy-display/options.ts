import {
  categorizeProxyGroup,
  categorizeProxyTarget,
} from './classification'
import {
  collectProxyNames,
  isBuiltinPolicyName,
  isHiddenProxyName,
  normalizeName,
} from './names'
import type { ProxyDisplayOption } from './types'

export const buildProxyDisplayOptions = (
  group?: IProxyGroupItem | null,
): ProxyDisplayOption[] => {
  const split = {
    manual: [] as ProxyDisplayOption[],
    strategy: [] as ProxyDisplayOption[],
  }

  if (!group?.all?.length) {
    return []
  }

  group.all.forEach((proxy) => {
    if (isHiddenProxyName(proxy.name) || isBuiltinPolicyName(proxy.name)) {
      return
    }

    const kind = categorizeProxyTarget(proxy)
    if (kind === 'manual') {
      split.manual.push({ name: proxy.name, kind })
    }

    if (kind === 'strategy') {
      split.strategy.push({ name: proxy.name, kind })
    }
  })

  return split.manual.concat(split.strategy)
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
    if (isHiddenProxyName(name) || isBuiltinPolicyName(name)) {
      return
    }

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
    names: Array.from(
      new Set(names.map((name) => normalizeName(name)).filter(Boolean)),
    ),
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
  group?:
    | IProxyGroupItem
    | { all?: Array<string | { name?: string }>; now?: string }
    | null,
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

