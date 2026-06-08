import { type DragEndEvent } from '@dnd-kit/core'
import { TauriEvent } from '@tauri-apps/api/event'
import { readText } from '@tauri-apps/plugin-clipboard-manager'
import { useLockFn } from 'ahooks'
import { throttle } from 'lodash-es'
import {
  useEffect,
  useState,
  type Dispatch,
  type RefObject,
  type SetStateAction,
} from 'react'

import { useListen } from '@/hooks/system'
import {
  createProfileFromLocalPath,
  deleteProfile,
  enhanceProfiles,
  getProfiles,
  importProfile,
  reorderProfile,
  updateProfile,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { queryClient } from '@/services/query-client'
import { useSetLoadingCache } from '@/services/states'
import { debugLog } from '@/utils/misc'

import { isRemotePrimaryProfileItem } from './profile-item-utils'

interface UseProfilesPageControllerParams {
  profiles?: IProfilesView
  profileItems: IProfileItem[]
  mutateProfiles: () => Promise<unknown>
  mutateLogs: () => Promise<unknown>
  switchingProfileRef: RefObject<string | null>
  getCurrentActivatings: () => string[]
  setActivatings: Dispatch<SetStateAction<string[]>>
}

export function useProfilesPageController({
  profiles,
  profileItems,
  mutateProfiles,
  mutateLogs,
  switchingProfileRef,
  getCurrentActivatings,
  setActivatings,
}: UseProfilesPageControllerParams) {
  const { addListener } = useListen()

  const [url, setUrl] = useState('')
  const [disabled, setDisabled] = useState(false)
  const [loading, setLoading] = useState(false)

  useEffect(() => {
    const handleFileDrop = async () => {
      const unlisten = await addListener(
        TauriEvent.DRAG_DROP,
        async (event: any) => {
          const paths = event.payload.paths

          for (const file of paths) {
            if (!file.endsWith('.yaml') && !file.endsWith('.yml')) {
              showNotice.error('profiles.page.feedback.errors.onlyYaml')
              continue
            }

            const item = {
              type: 'local',
              name: file.split(/\/|\\/).pop() ?? 'New Profile',
              desc: '',
              url: '',
              option: {
                with_proxy: false,
                self_proxy: false,
              },
            } as IProfileItem

            await createProfileFromLocalPath(item, file)
            await mutateProfiles()
          }

          await enhanceProfiles()
        },
      )

      return unlisten
    }

    const unsubscribe = handleFileDrop()

    return () => {
      unsubscribe.then((cleanup) => cleanup())
    }
  }, [addListener, mutateProfiles])

  const onEnhance = useLockFn(async (notifySuccess: boolean) => {
    if (switchingProfileRef.current) {
      debugLog(
        `[Profile] Switch in progress (${switchingProfileRef.current}), skip enhance`,
      )
      return
    }

    const currentProfiles = getCurrentActivatings()
    setActivatings((prev) => [...new Set([...prev, ...currentProfiles])])

    try {
      if (!(await enhanceProfiles())) return
      void mutateLogs()

      if (notifySuccess) {
        showNotice.success(
          'profiles.page.feedback.notifications.profileReactivated',
          1000,
        )
      }
    } catch (error: any) {
      showNotice.error(error, 3000)
    } finally {
      setActivatings((prev) =>
        prev.filter((id) => id === switchingProfileRef.current),
      )
    }
  })

  const onEmergencyRefresh = useLockFn(async () => {
    debugLog('[Emergency Refresh] Start force refreshing profile data')

    try {
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: ['getProfiles'] }),
        queryClient.invalidateQueries({ queryKey: ['getRuntimeLogs'] }),
      ])

      await mutateProfiles()
      await new Promise((resolve) => setTimeout(resolve, 500))
      await onEnhance(false)

      showNotice.success(
        'profiles.page.feedback.notices.forceRefreshCompleted',
        2000,
      )
    } catch (error) {
      console.error('[Emergency Refresh] Failed:', error)
      showNotice.error(
        'profiles.page.feedback.notices.emergencyRefreshFailed',
        { message: String(error) },
        4000,
      )
    }
  })

  const performRobustRefresh = async () => {
    let retryCount = 0
    const maxRetries = 1
    const baseDelay = 200

    while (retryCount < maxRetries) {
      try {
        debugLog(
          `[Import Refresh] Attempt ${retryCount + 1} to refresh profiles`,
        )

        await mutateProfiles()
        await new Promise((resolve) =>
          setTimeout(resolve, baseDelay * (retryCount + 1)),
        )

        await onEnhance(false)
        return
      } catch (error) {
        console.error(
          `[Import Refresh] Attempt ${retryCount + 1} failed`,
          error,
        )
        retryCount += 1
        await new Promise((resolve) =>
          setTimeout(resolve, baseDelay * retryCount),
        )
      }
    }

    console.warn(
      '[Import Refresh] Standard refresh failed, retrying with direct fetch',
    )

    try {
      await queryClient.fetchQuery({
        queryKey: ['getProfiles'],
        queryFn: getProfiles,
      })
      await onEnhance(false)
      showNotice.error(
        'profiles.page.feedback.notifications.importNeedsRefresh',
        3000,
      )
    } catch (finalError) {
      console.error('[Import Refresh] Final refresh attempt failed', finalError)
      showNotice.error(
        'profiles.page.feedback.notifications.importSuccess',
        5000,
      )
    }
  }

  const onImport = useLockFn(async () => {
    if (!url) return

    if (!/^https?:\/\//i.test(url)) {
      showNotice.error('profiles.page.feedback.errors.invalidUrl')
      return
    }

    setDisabled(true)
    setLoading(true)

    const handleImportSuccess = async (noticeKey: string) => {
      showNotice.success(noticeKey)
      setUrl('')
      await performRobustRefresh()
    }

    try {
      await importProfile(url)
      await handleImportSuccess('shared.feedback.notifications.importSuccess')
    } catch (initialError) {
      console.warn('[Profile Import] Primary import failed:', initialError)

      showNotice.info('profiles.page.feedback.notifications.importRetry')

      try {
        await importProfile(url, {
          with_proxy: false,
          self_proxy: true,
        })
        await handleImportSuccess(
          'shared.feedback.notifications.importWithClashProxy',
        )
      } catch (retryError) {
        showNotice.error(
          'profiles.page.feedback.notifications.importFail',
          String(retryError),
        )
      }
    } finally {
      setDisabled(false)
      setLoading(false)
    }
  })

  const onDragEnd = useLockFn(async (event: DragEndEvent) => {
    const { active, over } = event
    if (!over || active.id === over.id) return

    await reorderProfile(active.id.toString(), over.id.toString())
    void mutateProfiles()
  })

  const onDelete = useLockFn(async (uid: string) => {
    const currentProfileId = profiles?.currentPrimaryUid ?? profiles?.current
    const isCurrent = currentProfileId === uid

    try {
      setActivatings([...(isCurrent ? getCurrentActivatings() : []), uid])
      await deleteProfile(uid)
      void mutateProfiles()
      void mutateLogs()

      if (isCurrent) {
        await onEnhance(false)
      }
    } catch (error: any) {
      showNotice.error(error)
    } finally {
      setActivatings([])
    }
  })

  const setLoadingCache = useSetLoadingCache()
  const onUpdateAll = useLockFn(async () => {
    const throttleMutate = throttle(mutateProfiles, 2000, {
      trailing: true,
    })

    const updateOne = async (uid: string) => {
      try {
        await updateProfile(uid)
        void throttleMutate()
      } catch (error: any) {
        console.error(`Failed to update subscription ${uid}:`, error)
      } finally {
        setLoadingCache((cache) => ({ ...cache, [uid]: false }))
      }
    }

    return new Promise((resolve) => {
      setLoadingCache((cache) => {
        const items = profileItems.filter(
          (item) => isRemotePrimaryProfileItem(item) && !cache[item.uid],
        )
        const change = Object.fromEntries(items.map((item) => [item.uid, true]))

        Promise.allSettled(items.map((item) => updateOne(item.uid))).then(
          resolve,
        )
        return { ...cache, ...change }
      })
    })
  })

  const onCopyLink = useLockFn(async () => {
    const text = await readText()
    if (text) {
      setUrl(text)
    }
  })

  return {
    url,
    setUrl,
    disabled,
    loading,
    onEnhance,
    onEmergencyRefresh,
    onImport,
    onDragEnd,
    onDelete,
    onUpdateAll,
    onCopyLink,
  }
}
