import {
  isBuiltinPolicyName,
  isHiddenProxyName,
} from '@/services/proxy-display'
import type { IProxyGroupItem } from '@/types/proxy'

import type { StrategyPoolGroupRef } from './types'

export const MANAGED_STRATEGY_POOL_CONFIG_TYPES = new Set([
  'url-test',
  'load-balance',
])

export const DEFAULT_STRATEGY_POOL_CONFIG_TYPE: IProxyGroupConfig['type'] =
  'url-test'

export const STRATEGY_POOL_NAME_PREFIX = '策略池'

export const normalizeStrategyPoolName = (value?: string | null) =>
  value?.trim() || ''

export const normalizeStrategyPoolNames = (
  names: Array<string | null | undefined>,
) =>
  Array.from(
    new Set(
      names
        .map((name) => normalizeStrategyPoolName(name))
        .filter((name) => name.length > 0),
    ),
  )

export const normalizeStrategyPoolConfigType = (
  type?: string | null,
): IProxyGroupConfig['type'] => {
  const normalizedType = normalizeStrategyPoolName(type).toLowerCase()

  if (normalizedType === 'load-balance' || normalizedType === 'loadbalance') {
    return 'load-balance'
  }

  return 'url-test'
}

export const isManagedStrategyPoolConfig = (
  group?: IProxyGroupConfig | null,
): group is IProxyGroupConfig => {
  const name = normalizeStrategyPoolName(group?.name)
  const type = normalizeStrategyPoolName(group?.type).toLowerCase()

  return Boolean(name) && MANAGED_STRATEGY_POOL_CONFIG_TYPES.has(type)
}

export const filterStrategyPoolMemberNames = (names?: string[]) =>
  normalizeStrategyPoolNames(names || []).filter(
    (name) => !isHiddenProxyName(name) && !isBuiltinPolicyName(name),
  )

export const dedupeManagedStrategyPoolConfigs = (groups: IProxyGroupConfig[]) => {
  const configsByName = new Map<string, IProxyGroupConfig>()

  groups.forEach((group) => {
    if (!isManagedStrategyPoolConfig(group)) return

    const name = normalizeStrategyPoolName(group.name)
    const normalizedGroup: IProxyGroupConfig = {
      ...group,
      name,
      type: normalizeStrategyPoolConfigType(group.type),
      proxies: filterStrategyPoolMemberNames(group.proxies),
    }

    if (configsByName.has(name)) {
      configsByName.delete(name)
    }

    configsByName.set(name, normalizedGroup)
  })

  return Array.from(configsByName.values())
}

export const buildStrategyPoolGroupRef = ({
  configGroup,
  runtimeGroup,
}: {
  configGroup: IProxyGroupConfig
  runtimeGroup: IProxyGroupItem | null
}): StrategyPoolGroupRef => ({
  name: configGroup.name,
  configType: normalizeStrategyPoolConfigType(configGroup.type),
  displayType: runtimeGroup?.type || configGroup.type,
  testUrl: runtimeGroup?.testUrl || configGroup.url,
  hidden: runtimeGroup?.hidden ?? configGroup.hidden,
  icon: runtimeGroup?.icon ?? configGroup.icon,
})

export const buildNextStrategyPoolName = (names: string[]) => {
  const usedNames = new Set(
    names
      .map((name) => normalizeStrategyPoolName(name))
      .filter((name) => name.length > 0),
  )

  let index = 1
  while (usedNames.has(`${STRATEGY_POOL_NAME_PREFIX}${index}`)) {
    index += 1
  }

  return `${STRATEGY_POOL_NAME_PREFIX}${index}`
}

export const createStrategyPoolGroupRef = ({
  configType = DEFAULT_STRATEGY_POOL_CONFIG_TYPE,
  name,
}: {
  configType?: IProxyGroupConfig['type']
  name: string
}): StrategyPoolGroupRef => ({
  name,
  configType,
  displayType: configType,
})
