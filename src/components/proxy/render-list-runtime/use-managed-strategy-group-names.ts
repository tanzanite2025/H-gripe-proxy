import { useQuery } from '@tanstack/react-query'

import {
  isManagedStrategyGroupConfig,
  readOverrideGroups,
  readProfileProxyGroups,
} from './strategy-group-data'

export function useManagedStrategyGroupNames(
  profileUid: string,
  groupsOverridePath: string,
) {
  return useQuery({
    queryKey: [
      'proxy-managed-strategy-groups',
      profileUid,
      groupsOverridePath,
    ],
    enabled: !!profileUid || !!groupsOverridePath,
    staleTime: 3_000,
    refetchOnWindowFocus: false,
    queryFn: async () => {
      const names = new Set<string>()

      if (profileUid) {
        try {
          const profileGroups = await readProfileProxyGroups(profileUid)

          profileGroups.forEach((group) => {
            if (!isManagedStrategyGroupConfig(group)) return
            names.add(group.name.trim())
          })
        } catch {}
      }

      if (groupsOverridePath) {
        try {
          const overrideGroups = await readOverrideGroups(groupsOverridePath)

          overrideGroups.forEach((group) => {
            if (!isManagedStrategyGroupConfig(group)) return
            names.add(group.name.trim())
          })
        } catch {}
      }

      return Array.from(names)
    },
  })
}
