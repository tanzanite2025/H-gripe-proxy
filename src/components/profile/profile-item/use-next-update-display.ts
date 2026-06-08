import { useLockFn } from 'ahooks'
import dayjs from 'dayjs'
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type MouseEvent,
} from 'react'
import { useTranslation } from 'react-i18next'

import { getNextUpdateTime } from '@/services/cmds'
import { debugLog } from '@/utils/misc'

const OVERDUE_GRACE_SECONDS = 5 * 60

interface UseNextUpdateDisplayParams {
  uid: string
  updateInterval?: number
  updated: number
}

export function useNextUpdateDisplay({
  uid,
  updateInterval,
  updated,
}: UseNextUpdateDisplayParams) {
  const { t } = useTranslation()
  const [showNextUpdate, setShowNextUpdate] = useState(false)
  const [nextUpdateTime, setNextUpdateTime] = useState('')
  const showNextUpdateRef = useRef(false)
  const refreshTimeoutRef = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  )

  const refreshNextUpdateTime = useLockFn(async (forceRefresh = false) => {
    if (updateInterval && updateInterval > 0) {
      try {
        debugLog(`[ProfileItem] Fetch next update time: ${uid}`)

        if (forceRefresh) {
          debugLog(`[ProfileItem] Force refresh next update time: ${uid}`)
        }

        const nextUpdate = await getNextUpdateTime(uid)
        debugLog('[ProfileItem] Next update time fetched', nextUpdate)

        if (nextUpdate) {
          const nextUpdateDate = dayjs(nextUpdate * 1000)
          const now = dayjs()
          const diffSeconds = nextUpdateDate.diff(now, 'second')

          if (diffSeconds <= 0) {
            const overdueSeconds = now.diff(nextUpdateDate, 'second')

            if (overdueSeconds <= OVERDUE_GRACE_SECONDS) {
              setNextUpdateTime(
                `${t('profiles.components.profileItem.status.nextUp')} <1m`,
              )
              return
            }

            debugLog(
              `[ProfileItem] Next update schedule is overdue without explicit failure signal: ${uid}, overdue=${overdueSeconds}s`,
            )
            setNextUpdateTime(t('profiles.components.profileItem.status.unknown'))
            return
          }

          const diffMinutes = nextUpdateDate.diff(now, 'minute')

          if (diffMinutes < 60) {
            if (diffMinutes <= 0) {
              setNextUpdateTime(
                `${t('profiles.components.profileItem.status.nextUp')} <1m`,
              )
            } else {
              setNextUpdateTime(
                `${t('profiles.components.profileItem.status.nextUp')} ${diffMinutes}m`,
              )
            }
            return
          }

          const hours = Math.floor(diffMinutes / 60)
          const minutes = diffMinutes % 60

          setNextUpdateTime(
            `${t('profiles.components.profileItem.status.nextUp')} ${hours}h ${minutes}m`,
          )
          return
        }

        setNextUpdateTime(t('profiles.components.profileItem.status.noSchedule'))
      } catch (error) {
        console.error('[ProfileItem] Failed to fetch next update time:', error)
        setNextUpdateTime(t('profiles.components.profileItem.status.unknown'))
      }
      return
    }

    debugLog(`[ProfileItem] Auto update disabled: ${uid}`)
    setNextUpdateTime(
      t('profiles.components.profileItem.status.autoUpdateDisabled'),
    )
  })

  const toggleUpdateTimeDisplay = useCallback(
    (event: MouseEvent) => {
      event.stopPropagation()

      if (!showNextUpdate) {
        void refreshNextUpdateTime()
      }

      setShowNextUpdate((current) => !current)
    },
    [refreshNextUpdateTime, showNextUpdate],
  )

  useEffect(() => {
    showNextUpdateRef.current = showNextUpdate
  }, [showNextUpdate])

  useEffect(() => {
    if (showNextUpdate) {
      void refreshNextUpdateTime()
    }
  }, [refreshNextUpdateTime, showNextUpdate, updateInterval, updated])

  useEffect(() => {
    const handleTimerUpdate = (event: Event) => {
      const source = event as CustomEvent<string> & { payload?: string }
      const updatedUid = source.detail ?? source.payload

      if (updatedUid === uid && showNextUpdateRef.current) {
        debugLog(`[ProfileItem] Timer updated event received: ${updatedUid}`)

        if (refreshTimeoutRef.current !== undefined) {
          clearTimeout(refreshTimeoutRef.current)
        }

        refreshTimeoutRef.current = window.setTimeout(() => {
          void refreshNextUpdateTime(true)
        }, 1000)
      }
    }

    window.addEventListener('verge://timer-updated', handleTimerUpdate)

    return () => {
      if (refreshTimeoutRef.current !== undefined) {
        clearTimeout(refreshTimeoutRef.current)
      }

      window.removeEventListener('verge://timer-updated', handleTimerUpdate)
    }
  }, [refreshNextUpdateTime, uid])

  return {
    showNextUpdate,
    nextUpdateTime,
    toggleUpdateTimeDisplay,
    refreshNextUpdateTime,
  }
}
