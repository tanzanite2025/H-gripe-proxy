import { useCallback, useRef } from 'react'

import { syncTrayProxySelection } from '@/services/cmds'
import { closeConnectionsForProxy } from '@/services/connection-runtime'
import { applyProxyRuntimeSelection } from '@/services/proxy-runtime-selection'
import { debugLog } from '@/utils/misc'

import { useProfiles } from './use-profiles'

const cleanupConnections = async (previousProxy: string) => {
  try {
    const cleanupCount = await closeConnectionsForProxy(previousProxy)

    if (cleanupCount > 0) {
      debugLog(`[ProxySelection] cleaned ${cleanupCount} connections`)
    }
  } catch (error) {
    console.warn('[ProxySelection] failed to clean connections:', error)
  }
}

interface ProxySelectionOptions {
  onSuccess?: () => void
  onError?: (error: any) => void
  enableConnectionCleanup?: boolean
}

interface ProxyChangeRequest {
  groupName: string
  proxyName: string
  previousProxy?: string
  skipConfigSave: boolean
}

export const useProxySelection = (options: ProxySelectionOptions = {}) => {
  const { current, patchCurrent } = useProfiles()
  const pendingRequestRef = useRef<ProxyChangeRequest | null>(null)
  const isProcessingRef = useRef(false)

  const { onSuccess, onError, enableConnectionCleanup = true } = options

  const syncTraySelection = useCallback(() => {
    syncTrayProxySelection().catch((error) => {
      console.error('[ProxySelection] failed to sync tray state:', error)
    })
  }, [])

  const persistSelection = useCallback(
    (groupName: string, proxyName: string, skipConfigSave: boolean) => {
      if (!current || skipConfigSave) return

      const selected = current.selected ? [...current.selected] : []
      const index = selected.findIndex((item) => item.name === groupName)

      if (index < 0) {
        selected.push({ name: groupName, now: proxyName })
      } else {
        selected[index] = { name: groupName, now: proxyName }
      }

      patchCurrent({ selected }).catch((error) => {
        console.error('[ProxySelection] failed to persist selection:', error)
      })
    },
    [current, patchCurrent],
  )

  const executeChange = useCallback(
    async (request: ProxyChangeRequest) => {
      const { groupName, proxyName, previousProxy, skipConfigSave } = request
      debugLog(`[ProxySelection] change proxy: ${groupName} -> ${proxyName}`)

      try {
        await applyProxyRuntimeSelection(groupName, proxyName, {
          syncTray: false,
        })
        onSuccess?.()
        syncTraySelection()
        persistSelection(groupName, proxyName, skipConfigSave)
        debugLog(
          `[ProxySelection] proxy and state synced: ${groupName} -> ${proxyName}`,
        )

        if (enableConnectionCleanup && previousProxy) {
          setTimeout(() => cleanupConnections(previousProxy), 0)
        }
      } catch (error) {
        console.error(
          `[ProxySelection] failed to change proxy: ${groupName} -> ${proxyName}`,
          error,
        )

        try {
          await applyProxyRuntimeSelection(groupName, proxyName, {
            syncTray: false,
          })
          onSuccess?.()
          syncTraySelection()
          persistSelection(groupName, proxyName, skipConfigSave)
          debugLog(
            `[ProxySelection] retry succeeded: ${groupName} -> ${proxyName}`,
          )
        } catch (fallbackError) {
          console.error(
            `[ProxySelection] retry also failed: ${groupName} -> ${proxyName}`,
            fallbackError,
          )
          onError?.(fallbackError)
        }
      }
    },
    [
      enableConnectionCleanup,
      onError,
      onSuccess,
      persistSelection,
      syncTraySelection,
    ],
  )

  const flushChangeQueue = useCallback(async () => {
    if (isProcessingRef.current) return
    isProcessingRef.current = true

    try {
      while (pendingRequestRef.current) {
        const request = pendingRequestRef.current
        pendingRequestRef.current = null
        await executeChange(request)
      }
    } finally {
      isProcessingRef.current = false
      if (pendingRequestRef.current) {
        void flushChangeQueue()
      }
    }
  }, [executeChange])

  const changeProxy = useCallback(
    (
      groupName: string,
      proxyName: string,
      previousProxy?: string,
      skipConfigSave: boolean = false,
    ) => {
      pendingRequestRef.current = {
        groupName,
        proxyName,
        previousProxy,
        skipConfigSave,
      }
      void flushChangeQueue()
    },
    [flushChangeQueue],
  )

  const handleSelectChange = useCallback(
    (
      groupName: string,
      previousProxy?: string,
      skipConfigSave: boolean = false,
    ) =>
      (event: { target: { value: string } }) => {
        const newProxy = event.target.value
        changeProxy(groupName, newProxy, previousProxy, skipConfigSave)
      },
    [changeProxy],
  )

  const handleProxyGroupChange = useCallback(
    (group: { name: string; now?: string }, proxy: { name: string }) => {
      changeProxy(group.name, proxy.name, group.now)
    },
    [changeProxy],
  )

  return {
    changeProxy,
    handleSelectChange,
    handleProxyGroupChange,
  }
}
