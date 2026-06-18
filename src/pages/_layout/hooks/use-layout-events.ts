import { listen } from '@tauri-apps/api/event'
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow'
import { useEffect } from 'react'

import { useListen } from '@/hooks/system'
import { queryClient } from '@/services/query-client'
import type { SubscriptionUpdateEvent } from '@/types/subscription-update'

const getSafeCurrentWebviewWindow = () => {
  try {
    return getCurrentWebviewWindow()
  } catch {
    return null
  }
}

export const useLayoutEvents = (
  handleNotice: (payload: [string, string]) => void,
  handleSubscriptionUpdate?: (event: SubscriptionUpdateEvent) => void,
) => {
  const { addListener } = useListen()

  useEffect(() => {
    const unlisteners: Array<() => void> = []
    let disposed = false
    const revalidateKeys = (keys: readonly string[]) => {
      keys.forEach((key) => {
        queryClient.invalidateQueries({ queryKey: [key] })
      })
    }

    const register = (
      maybeUnlisten: void | (() => void) | Promise<void | (() => void)>,
    ) => {
      if (!maybeUnlisten) return

      if (typeof maybeUnlisten === 'function') {
        unlisteners.push(maybeUnlisten)
        return
      }

      maybeUnlisten
        .then((unlisten) => {
          if (!unlisten) return
          if (disposed) {
            unlisten()
          } else {
            unlisteners.push(unlisten)
          }
        })
        .catch((error) =>
          console.error('[Event Listener] Registration failed:', error),
        )
    }

    register(
      addListener('verge://refresh-clash-config', async () => {
        revalidateKeys([
          'getRuntimeProxyTopology',
          'getVersion',
          'getClashConfig',
          'getRuntimeProxyProviders',
          'getRuntimeRules',
          'getRuntimeRuleProviders',
          'current-egress-identity',
        ])
      }),
    )

    register(
      addListener('verge://refresh-proxy-config', async () => {
        revalidateKeys(['getRuntimeProxyTopology', 'current-egress-identity'])
      }),
    )

    register(
      addListener('verge://refresh-verge-config', () => {
        revalidateKeys([
          'getVergeConfig',
          'getSystemProxy',
          'getAutotemProxy',
          'getRunningMode',
          'isServiceAvailable',
          'getSystemState',
        ])
      }),
    )

    register(
      addListener('verge://notice-message', ({ payload }) =>
        handleNotice(payload as [string, string]),
      ),
    )

    register(
      addListener('verge://subscription-update', ({ payload }) => {
        const event = payload as SubscriptionUpdateEvent
        queryClient.invalidateQueries({ queryKey: ['getSubscriptionState'] })
        queryClient.invalidateQueries({
          queryKey: ['getSubscriptionSourceState', event.source_id],
        })
        queryClient.invalidateQueries({
          queryKey: ['getSubscriptionSourceUpdateEvents', event.source_id],
        })
        window.dispatchEvent(
          new CustomEvent<SubscriptionUpdateEvent>('subscription-update', {
            detail: event,
          }),
        )
        handleSubscriptionUpdate?.(event)
      }),
    )

    const appWindow = getSafeCurrentWebviewWindow()
    if (appWindow) {
      register(
        (async () => {
          const [hideUnlisten, showUnlisten] = await Promise.all([
            listen('verge://hide-window', () => appWindow.hide()),
            listen('verge://show-window', () => appWindow.show()),
          ])
          return () => {
            hideUnlisten()
            showUnlisten()
          }
        })(),
      )
    }

    return () => {
      disposed = true
      const errors: Error[] = []

      unlisteners.forEach((unlisten) => {
        try {
          unlisten()
        } catch (error) {
          errors.push(error instanceof Error ? error : new Error(String(error)))
        }
      })

      if (errors.length > 0) {
        console.error(
          `[Event Listener] Encountered ${errors.length} errors during cleanup:`,
          errors,
        )
      }

      unlisteners.length = 0
    }
  }, [addListener, handleNotice, handleSubscriptionUpdate])
}
