import { useLockFn } from 'ahooks'
import { useMemo, useState } from 'react'
import { type ProxyProvider, updateProxyProvider } from 'tauri-plugin-mihomo-api'

import { useAppRefreshers, useProxiesData } from '@/providers/app-data-context'
import { showNotice } from '@/services/notice-service'

import { buildUpdatingMap } from './utils'

export const useProviderButtonController = () => {
  const [open, setOpen] = useState(false)
  const [updating, setUpdating] = useState<Record<string, boolean>>({})
  const { proxyProviders } = useProxiesData()
  const { refreshProxy, refreshProxyProviders } = useAppRefreshers()

  const providers = useMemo(
    () =>
      Object.entries(proxyProviders || {}).sort(([left], [right]) =>
        left.localeCompare(right),
      ) as Array<[string, ProxyProvider]>,
    [proxyProviders],
  )

  const hasProviders = providers.length > 0

  const refreshProviderData = async () => {
    await refreshProxy()
    await refreshProxyProviders()
  }

  const updateProvider = useLockFn(async (name: string) => {
    try {
      setUpdating((prev) => ({ ...prev, [name]: true }))
      await updateProxyProvider(name)
      await refreshProviderData()

      showNotice.success(
        'proxies.feedback.notifications.provider.updateSuccess',
        { name },
      )
    } catch (error) {
      showNotice.error('proxies.feedback.notifications.provider.updateFailed', {
        name,
        message: String(error),
      })
    } finally {
      setUpdating((prev) => ({ ...prev, [name]: false }))
    }
  })

  const updateAllProviders = useLockFn(async () => {
    try {
      const allProviders = providers.map(([name]) => name)
      if (allProviders.length === 0) {
        showNotice.info('proxies.feedback.notifications.provider.none')
        return
      }

      setUpdating(buildUpdatingMap(allProviders))

      for (const name of allProviders) {
        try {
          await updateProxyProvider(name)
          setUpdating((prev) => ({ ...prev, [name]: false }))
        } catch (error) {
          console.error(`failed to update provider: ${name}`, error)
        }
      }

      await refreshProviderData()
      showNotice.success('proxies.feedback.notifications.provider.allUpdated')
    } catch (error) {
      showNotice.error('proxies.feedback.notifications.provider.genericError', {
        message: String(error),
      })
    } finally {
      setUpdating({})
    }
  })

  return {
    open,
    hasProviders,
    providers,
    updating,
    openDialog: () => setOpen(true),
    closeDialog: () => setOpen(false),
    updateProvider,
    updateAllProviders,
  }
}
