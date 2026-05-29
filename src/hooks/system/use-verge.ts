import { useQuery, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'

import { applyDnsConfig, getVergeConfig, patchVergeConfig } from '@/services/cmds'
import { getPreloadConfig, setPreloadConfig } from '@/services/preload'

export const useVerge = () => {
  const qc = useQueryClient()
  const initialVergeConfig = getPreloadConfig()

  const { data: verge, refetch } = useQuery({
    queryKey: ['getVergeConfig'],
    queryFn: async () => {
      const config = await getVergeConfig()
      setPreloadConfig(config)
      return config
    },
    initialData: initialVergeConfig ?? undefined,
    staleTime: 5000,
  })

  const mutateVerge = (
    updaterOrData?:
      | IVergeConfig
      | ((prev: IVergeConfig | undefined) => IVergeConfig | undefined)
      | undefined,
    _revalidate?: boolean,
  ) => {
    if (updaterOrData === undefined) {
      void refetch()
      return
    }
    if (typeof updaterOrData === 'function') {
      const prev = qc.getQueryData<IVergeConfig>(['getVergeConfig'])
      const next = updaterOrData(prev)
      qc.setQueryData(['getVergeConfig'], next)
    } else {
      qc.setQueryData(['getVergeConfig'], updaterOrData)
    }
  }

  const patchVerge = useCallback(
    async (value: Partial<IVergeConfig>) => {
      await patchVergeConfig(value)
      await refetch()
    },
    [refetch],
  )

  const setDnsRuntimeEnabled = useCallback(
    async (enable: boolean) => {
      const previous = qc.getQueryData<IVergeConfig>(['getVergeConfig'])
      const previousEnabled = previous?.enable_dns_settings ?? false

      qc.setQueryData<IVergeConfig | undefined>(['getVergeConfig'], (current) =>
        current ? { ...current, enable_dns_settings: enable } : current,
      )

      try {
        await patchVergeConfig({ enable_dns_settings: enable })
        await applyDnsConfig(enable)
        await refetch()
      } catch (error) {
        qc.setQueryData<IVergeConfig | undefined>(['getVergeConfig'], (current) =>
          current
            ? { ...current, enable_dns_settings: previousEnabled }
            : current,
        )

        await patchVergeConfig({ enable_dns_settings: previousEnabled }).catch(
          () => {},
        )
        await refetch().catch(() => {})
        throw error
      }
    },
    [qc, refetch],
  )

  return {
    verge,
    mutateVerge,
    patchVerge,
    setDnsRuntimeEnabled,
  }
}
