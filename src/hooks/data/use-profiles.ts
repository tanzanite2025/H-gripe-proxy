import { useQuery } from '@tanstack/react-query'

import {
  getProfiles,
  patchProfile,
  patchProfilesConfig,
} from '@/services/cmds'
import {
  isAuxiliarySelectionName,
  pickPreferredProxyNameFromGroup,
} from '@/services/proxy-display'
import { calcuProxies } from '@/services/proxy-runtime'
import { applyProxyRuntimeSelection } from '@/services/proxy-runtime-selection'
import { queryClient } from '@/services/query-client'
import { debugLog } from '@/utils/misc'

export const useProfiles = () => {
  const {
    data: profiles,
    refetch,
    error,
    isFetching: isValidating,
  } = useQuery({
    queryKey: ['getProfiles'],
    queryFn: async () => {
      const data = await getProfiles()
      debugLog(
        '[useProfiles] profile data refreshed',
        data?.items?.length || 0,
      )
      return data
    },
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    staleTime: 500,
    retry: 3,
    retryDelay: 1000,
    refetchInterval: false,
  })

  const mutateProfiles = async () => {
    await refetch()
  }

  const patchProfiles = async (
    value: Partial<IProfilesConfig>,
    signal?: AbortSignal,
    options?: { deferRefreshOnSuccess?: boolean },
  ) => {
    try {
      if (signal?.aborted) {
        throw new DOMException('Operation was aborted', 'AbortError')
      }

      const success = await patchProfilesConfig(value)

      if (signal?.aborted) {
        throw new DOMException('Operation was aborted', 'AbortError')
      }

      if (!options?.deferRefreshOnSuccess || !success) {
        await mutateProfiles()
      }

      return success
    } catch (error) {
      if (error instanceof DOMException && error.name === 'AbortError') {
        throw error
      }

      await mutateProfiles()
      throw error
    }
  }

  const patchCurrent = async (value: Partial<IProfileItem>) => {
    if (profiles?.current) {
      await patchProfile(profiles.current, value)
      if (!value.selected) {
        mutateProfiles()
      }
    }
  }

  const activateSelected = async (profileOverride?: IProfilesConfig) => {
    try {
      debugLog('[ActivateSelected] start restoring proxy selections')

      const proxiesData = await calcuProxies()
      const profileData = profileOverride ?? profiles

      if (!profileData || !proxiesData || !profileData.items) {
        debugLog('[ActivateSelected] profiles or proxies unavailable, skip')
        return
      }

      const current = profileData.items?.find(
        (item) => item && item.uid === profileData.current,
      )

      if (!current) {
        debugLog('[ActivateSelected] current profile not found')
        return
      }

      const { selected = [] } = current
      if (selected.length === 0) {
        debugLog('[ActivateSelected] no saved proxy selection for current profile')
        return
      }

      debugLog(
        `[ActivateSelected] restore ${selected.length} saved proxy selections`,
      )

      type SelectedEntry = { name?: string; now?: string }
      const selectedMap = Object.fromEntries(
        (selected as SelectedEntry[])
          .filter(
            (entry): entry is SelectedEntry & { name: string; now: string } =>
              entry.name != null && entry.now != null,
          )
          .map((entry) => [entry.name, entry.now]),
      )

      let hasChange = false
      const newSelected: typeof selected = []
      const { global, groups, records } = proxiesData
      const selectableTypes = new Set(['Selector', 'URLTest', 'LoadBalance'])

      for (const group of [global, ...groups]) {
        if (!group) continue

        const { type, name, now } = group
        const savedProxy = selectedMap[name]
        const availableProxies = Array.isArray(group.all) ? group.all : []

        if (type === 'Fallback') {
          if (savedProxy != null) {
            hasChange = true
          }
          continue
        }

        if (!selectableTypes.has(type)) {
          if (savedProxy != null || now != null) {
            newSelected.push({ name, now: now || savedProxy })
          }
          continue
        }

        const desiredProxy = pickPreferredProxyNameFromGroup(
          group,
          records,
          savedProxy ?? now,
        )
        const effectiveDesiredProxy =
          desiredProxy || savedProxy || now || ''

        if (!effectiveDesiredProxy) {
          continue
        }

        const existsInGroup = availableProxies.some((proxy) => {
          if (typeof proxy === 'string') {
            return proxy === effectiveDesiredProxy
          }

          return proxy?.name === effectiveDesiredProxy
        })

        if (!existsInGroup) {
          console.warn(
            `[ActivateSelected] saved proxy ${effectiveDesiredProxy} missing in group ${name}`,
          )
          hasChange = true
          if (now != null) {
            newSelected.push({ name, now })
          }
          continue
        }

        if (
          savedProxy !== effectiveDesiredProxy ||
          now !== effectiveDesiredProxy ||
          isAuxiliarySelectionName(now, records)
        ) {
          debugLog(
            `[ActivateSelected] switch group ${name}: ${now} -> ${effectiveDesiredProxy}`,
          )
          hasChange = true

          try {
            await applyProxyRuntimeSelection(name, effectiveDesiredProxy)
          } catch (error: unknown) {
            console.warn(
              `[ActivateSelected] failed to switch group ${name}:`,
              error instanceof Error ? error.message : String(error),
            )
          }
        }

        newSelected.push({ name, now: effectiveDesiredProxy })
      }

      if (!hasChange) {
        debugLog('[ActivateSelected] proxy selections already sanitized')
        return
      }

      debugLog('[ActivateSelected] persist sanitized proxy selections')

      try {
        await patchProfile(current.uid, { selected: newSelected })
        debugLog('[ActivateSelected] sanitized selections saved')
        queryClient.setQueryData(['getProxies'], await calcuProxies())
      } catch (error: unknown) {
        console.error(
          '[ActivateSelected] failed to save proxy selections:',
          error instanceof Error ? error.message : String(error),
        )
      }
    } catch (error: unknown) {
      console.error(
        '[ActivateSelected] failed to restore proxy selections:',
        error instanceof Error ? error.message : String(error),
      )
    }
  }

  return {
    profiles,
    current: (profiles?.primaryItems ?? profiles?.items)?.find(
      (profile) =>
        profile &&
        profile.uid === (profiles?.currentPrimaryUid ?? profiles?.current),
    ),
    activateSelected,
    patchProfiles,
    patchCurrent,
    mutateProfiles,
    isLoading: isValidating,
    error,
    isStale: !profiles && !error && !isValidating,
  }
}
