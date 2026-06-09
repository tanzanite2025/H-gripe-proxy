import { useQuery } from '@tanstack/react-query'

import { normalizeNames, readOverrideGroups } from './strategy-group-data'

export function useStrategyGroupOverrides(groupsOverridePath: string) {
  return useQuery({
    queryKey: ['proxy-strategy-group-overrides', groupsOverridePath],
    enabled: !!groupsOverridePath,
    staleTime: 3_000,
    refetchOnWindowFocus: false,
    queryFn: async () => {
      const overrideGroups = await readOverrideGroups(groupsOverridePath)
      const overrides: Record<string, string[]> = {}

      overrideGroups.forEach((group) => {
        const name = group?.name?.trim()
        if (!name) return

        overrides[name] = Array.isArray(group.proxies)
          ? normalizeNames(group.proxies)
          : []
      })

      return overrides
    },
  })
}
