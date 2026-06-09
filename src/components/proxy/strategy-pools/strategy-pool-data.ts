import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'

import { useProfiles } from '@/hooks/data'
import { useProxiesData } from '@/providers/app-data-context'
import { readProfileFile } from '@/services/cmds'
import {
  categorizeProxyGroup,
  isProxyGroupItem,
} from '@/services/proxy-display'

import { parseGroupsYaml } from '../../profile/groups-editor-viewer/utils/group-helpers'
import {
  buildNextStrategyPoolName,
  buildStrategyPoolGroupRef,
  createStrategyPoolGroupRef,
  dedupeManagedStrategyPoolConfigs,
  filterStrategyPoolMemberNames,
} from './strategy-pool-rules'
import type { ManagedStrategyPool } from './types'

export const MANAGED_STRATEGY_POOLS_QUERY_KEY = 'managed-strategy-pool-configs'

const readManagedStrategyPoolConfigs = async (groupsOverridePath: string) => {
  const groupsData = await readProfileFile(groupsOverridePath)
  const sequence = parseGroupsYaml(groupsData)

  return dedupeManagedStrategyPoolConfigs([
    ...sequence.prepend,
    ...sequence.append,
  ] as IProxyGroupConfig[])
}

export const useManagedStrategyPools = () => {
  const { current } = useProfiles()
  const { proxies: proxiesData } = useProxiesData()

  const groupsOverridePath = current?.option?.groups?.trim() || ''

  const { data: configGroups = [] } = useQuery({
    queryKey: [MANAGED_STRATEGY_POOLS_QUERY_KEY, groupsOverridePath],
    enabled: Boolean(groupsOverridePath),
    staleTime: 3_000,
    refetchOnWindowFocus: false,
    queryFn: async () => readManagedStrategyPoolConfigs(groupsOverridePath),
  })

  const pools = useMemo<ManagedStrategyPool[]>(() => {
    return configGroups.map((configGroup) => {
      const runtimeRecord = proxiesData?.records?.[configGroup.name]
      const runtimeGroup =
        isProxyGroupItem(runtimeRecord) &&
        categorizeProxyGroup(runtimeRecord) === 'strategy'
          ? runtimeRecord
          : null

      return {
        currentProxyName: runtimeGroup?.now?.trim() || '内核未载入',
        groupRef: buildStrategyPoolGroupRef({
          configGroup,
          runtimeGroup,
        }),
        memberCount: filterStrategyPoolMemberNames(configGroup.proxies).length,
        runtimeLoaded: Boolean(runtimeGroup),
      }
    })
  }, [configGroups, proxiesData?.records])

  const nextStrategyPoolName = useMemo(() => {
    const configNames = configGroups.map((group) => group.name)
    const runtimeNames = Object.keys(proxiesData?.records || {})

    return buildNextStrategyPoolName(configNames.concat(runtimeNames))
  }, [configGroups, proxiesData?.records])

  const createStrategyPoolCandidate = useMemo(
    () =>
      createStrategyPoolGroupRef({
        name: nextStrategyPoolName,
      }),
    [nextStrategyPoolName],
  )

  return {
    createStrategyPoolCandidate,
    groupsOverridePath,
    pools,
  }
}
