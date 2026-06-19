import { useLockFn } from 'ahooks'
import { useCallback, useMemo, useState } from 'react'
import type { ProxyProvider } from 'tauri-plugin-mihomo-api'

import { useAppRefreshers, useProxiesData } from '@/providers/app-data-context'
import { showNotice } from '@/services/notice-service'
import {
  getRuntimeProviderHealthState,
  healthcheckRuntimeProxyProvider,
  updateRuntimeProxyProvider,
  type RuntimeProviderHealthRecord,
} from '@/services/proxy-runtime'

import { buildUpdatingMap } from './utils'

export const useProviderButtonController = () => {
  const [open, setOpen] = useState(false)
  const [updating, setUpdating] = useState<Record<string, boolean>>({})
  const [checking, setChecking] = useState<Record<string, boolean>>({})
  const [health, setHealth] = useState<
    Record<string, RuntimeProviderHealthRecord>
  >({})
  const { proxyProviders } = useProxiesData()
  const { refreshProxy, refreshProxyProviders } = useAppRefreshers()

  const refreshHealth = useCallback(async () => {
    try {
      const { records } = await getRuntimeProviderHealthState()
      setHealth(
        Object.fromEntries(
          records.map((record) => [record.providerName, record]),
        ),
      )
    } catch (error) {
      console.warn('failed to load provider health state', error)
    }
  }, [])

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
      await updateRuntimeProxyProvider(name)
      await refreshProviderData()
      await refreshHealth()

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
          await updateRuntimeProxyProvider(name)
          setUpdating((prev) => ({ ...prev, [name]: false }))
        } catch (error) {
          console.error(`failed to update provider: ${name}`, error)
        }
      }

      await refreshProviderData()
      await refreshHealth()
      showNotice.success('proxies.feedback.notifications.provider.allUpdated')
    } catch (error) {
      showNotice.error('proxies.feedback.notifications.provider.genericError', {
        message: String(error),
      })
    } finally {
      setUpdating({})
    }
  })

  const checkProvider = useLockFn(async (name: string) => {
    try {
      setChecking((prev) => ({ ...prev, [name]: true }))
      await healthcheckRuntimeProxyProvider(name)
      await refreshHealth()
    } catch (error) {
      showNotice.error('proxies.feedback.notifications.provider.genericError', {
        message: String(error),
      })
    } finally {
      setChecking((prev) => ({ ...prev, [name]: false }))
    }
  })

  const openDialog = () => {
    setOpen(true)
    void refreshHealth()
  }

  return {
    open,
    hasProviders,
    providers,
    updating,
    checking,
    health,
    openDialog,
    closeDialog: () => setOpen(false),
    updateProvider,
    updateAllProviders,
    checkProvider,
  }
}
