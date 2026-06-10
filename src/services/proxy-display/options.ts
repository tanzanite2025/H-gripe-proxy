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

export const getPreferredProxyGroupName = ({
  proxies,
}: {
  proxies: any
}): string => {
  if (!proxies) return ''

  const manualGroup = (proxies.groups || []).find(
    (group: IProxyGroupItem) => categorizeProxyGroup(group) === 'manual',
  )
  const strategyGroup = (proxies.groups || []).find(
    (group: IProxyGroupItem) => categorizeProxyGroup(group) === 'strategy',
  )

  return (
    normalizeName(manualGroup?.name) ||
    normalizeName(strategyGroup?.name) ||
    normalizeName(proxies.global?.name)
  )
}
