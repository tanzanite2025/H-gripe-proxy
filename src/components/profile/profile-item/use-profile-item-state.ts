import { useCallback, useEffect, useReducer } from 'react'

import { useLoadingCache, useSetLoadingCache } from '@/services/states'

import { formatExpireDate, parseProfileUrl } from './shared'
import { useNextUpdateDisplay } from './use-next-update-display'

interface UseProfileItemStateParams {
  itemData: IProfileItem
  mutateProfiles: () => Promise<void>
}

export function useProfileItemState({
  itemData,
  mutateProfiles,
}: UseProfileItemStateParams) {
  const loadingCache = useLoadingCache()
  const setLoadingCache = useSetLoadingCache()

  const { uid, name = 'Profile', extra, updated = 0, option } = itemData

  const {
    showNextUpdate,
    nextUpdateTime,
    toggleUpdateTimeDisplay,
    refreshNextUpdateTime,
  } = useNextUpdateDisplay({
    uid,
    updateInterval: itemData.option?.update_interval,
    updated,
  })

  const hasUrl = !!itemData.url
  const hasExtra = !!extra
  const hasHome = !!itemData.home
  const { upload = 0, download = 0, total = 0 } = extra ?? {}
  const from = parseProfileUrl(itemData.url)
  const description = itemData.desc
  const expire = formatExpireDate(extra?.expire)
  const progress = Math.min(
    Math.round(((download + upload) * 100) / (total + 0.01)) + 1,
    100,
  )
  const loading = loadingCache[uid] ?? false

  const setProfileLoading = useCallback(
    (nextLoading: boolean) => {
      setLoadingCache((cache) => ({ ...cache, [uid]: nextLoading }))
    },
    [setLoadingCache, uid],
  )

  const [, forceRefresh] = useReducer((value: number) => value + 1, 0)

  useEffect(() => {
    if (!hasUrl) return

    let timer: ReturnType<typeof setTimeout> | undefined

    const scheduleRefresh = () => {
      const now = Date.now()
      const lastUpdate = updated * 1000

      if (now - lastUpdate >= 24 * 36e5) return

      const wait = now - lastUpdate >= 36e5 ? 30e5 : 5e4

      timer = setTimeout(() => {
        forceRefresh()
        scheduleRefresh()
      }, wait)
    }

    scheduleRefresh()

    return () => {
      if (timer) {
        clearTimeout(timer)
        timer = undefined
      }
    }
  }, [hasUrl, updated])

  useEffect(() => {
    const handleUpdateStarted = (event: Event) => {
      const customEvent = event as CustomEvent<{ uid?: string }>
      if (customEvent.detail?.uid === uid) {
        setProfileLoading(true)
      }
    }

    const handleUpdateCompleted = (event: Event) => {
      const customEvent = event as CustomEvent<{ uid?: string }>
      if (customEvent.detail?.uid === uid) {
        setProfileLoading(false)
        void mutateProfiles()

        if (showNextUpdate) {
          void refreshNextUpdateTime()
        }
      }
    }

    window.addEventListener('profile-update-started', handleUpdateStarted)
    window.addEventListener('profile-update-completed', handleUpdateCompleted)

    return () => {
      window.removeEventListener('profile-update-started', handleUpdateStarted)
      window.removeEventListener(
        'profile-update-completed',
        handleUpdateCompleted,
      )
    }
  }, [
    mutateProfiles,
    refreshNextUpdateTime,
    setProfileLoading,
    showNextUpdate,
    uid,
  ])

  return {
    uid,
    name,
    option,
    hasUrl,
    hasExtra,
    hasHome,
    upload,
    download,
    total,
    from,
    description,
    expire,
    progress,
    updated,
    loading,
    setProfileLoading,
    showNextUpdate,
    nextUpdateTime,
    toggleUpdateTimeDisplay,
  }
}
