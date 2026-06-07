import { listen } from '@tauri-apps/api/event'
import { useEffect } from 'react'

import { debugLog } from '@/utils/misc'

interface UseProfileChangeListenerParams {
  mutateProfiles: () => Promise<unknown>
}

export function useProfileChangeListener({
  mutateProfiles,
}: UseProfileChangeListenerParams) {
  useEffect(() => {
    let unlistenPromise: Promise<() => void> | undefined
    let lastProfileId: string | null = null
    let lastUpdateTime = 0
    const debounceDelay = 200

    let refreshTimer: number | null = null

    const setupListener = async () => {
      unlistenPromise = listen<string>('profile-changed', (event) => {
        const newProfileId = event.payload
        const now = Date.now()

        debugLog(`[Profile] Received profile change event: ${newProfileId}`)

        if (
          lastProfileId === newProfileId &&
          now - lastUpdateTime < debounceDelay
        ) {
          debugLog('[Profile] Duplicate profile change event ignored')
          return
        }

        lastProfileId = newProfileId
        lastUpdateTime = now

        if (refreshTimer !== null) {
          window.clearTimeout(refreshTimer)
        }

        refreshTimer = window.setTimeout(() => {
          mutateProfiles().catch((error) => {
            console.error('[Profile] Failed to refresh profile data:', error)
          })
          refreshTimer = null
        }, 0)
      })
    }

    void setupListener()

    return () => {
      if (refreshTimer !== null) {
        window.clearTimeout(refreshTimer)
      }

      void unlistenPromise?.then((unlisten) => unlisten()).catch(console.error)
    }
  }, [mutateProfiles])
}
