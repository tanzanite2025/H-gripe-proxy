import { type DragEndEvent } from '@dnd-kit/core'
import { useQuery } from '@tanstack/react-query'
import { TauriEvent } from '@tauri-apps/api/event'
import { readText } from '@tauri-apps/plugin-clipboard-manager'
import { useLockFn } from 'ahooks'
import { throttle } from 'lodash-es'
import { useEffect, useMemo, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useLocation } from 'react-router'

import { BasePage, DialogRef } from '@/components/base'
import { ProfileHeader } from '@/components/profile/profile-header'
import { ProfileMore } from '@/components/profile/profile-more'
import { ProfileRulesPanel } from '@/components/profile/profile-rules-panel'
import {
  ProfileViewer,
  ProfileViewerRef,
} from '@/components/profile/profile-viewer'
import { ConfigViewer } from '@/components/setting/components/misc/config-editor'
import { Box } from '@/components/tailwind'
import { useProfiles } from '@/hooks/data'
import { useListen } from '@/hooks/system'
import { useAppRefreshers } from '@/providers/app-data-context'
import {
  createProfileFromLocalPath,
  deleteProfile,
  enhanceProfiles,
  getProfiles,
  getRuntimeLogs,
  importProfile,
  reorderProfile,
  updateProfile,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { queryClient } from '@/services/query-client'
import { useSetLoadingCache } from '@/services/states'
import { debugLog } from '@/utils/misc'

import { ProfileCardsSection } from './profiles-page/profile-cards-section'
import { useProfileActivation } from './profiles-page/use-profile-activation'
import { useProfileBatchSelection } from './profiles-page/use-profile-batch-selection'
import { useProfileChangeListener } from './profiles-page/use-profile-change-listener'

const ProfilePage = () => {
  const { t } = useTranslation()
  const location = useLocation()
  const { addListener } = useListen()
  const { refreshRules, refreshRuleProviders } = useAppRefreshers()

  const [url, setUrl] = useState('')
  const [disabled, setDisabled] = useState(false)
  const [loading, setLoading] = useState(false)
  const [mergeOpen, setMergeOpen] = useState(false)
  const [scriptOpen, setScriptOpen] = useState(false)

  const { current } = location.state || {}

  const {
    profiles = {},
    activateSelected,
    patchProfiles,
    mutateProfiles,
    error,
    isStale,
  } = useProfiles()

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

  const { data: chainLogs = {}, refetch: mutateLogs } = useQuery({
    queryKey: ['getRuntimeLogs'],
    queryFn: getRuntimeLogs,
  })

  const viewerRef = useRef<ProfileViewerRef>(null)
  const configRef = useRef<DialogRef>(null)

  const profileItems = useMemo(() => {
    const items = profiles.items || []
    return items.filter((item) => item && ['local', 'remote'].includes(item.type!))
  }, [profiles])

  const {
    activatings,
    setActivatings,
    switchingProfileRef,
    getCurrentActivatings,
    onSelect,
  } = useProfileActivation({
    currentProfileId: current,
    profiles,
    activateSelected,
    patchProfiles,
    mutateProfiles,
    refreshRules,
    refreshRuleProviders,
    mutateLogs,
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

  const onImport = async () => {
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
  }

  const onDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event
    if (over && active.id !== over.id) {
      await reorderProfile(active.id.toString(), over.id.toString())
      void mutateProfiles()
    }
  }

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

  const onDelete = useLockFn(async (uid: string) => {
    const isCurrent = profiles.current === uid

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
          (item) => item.type === 'remote' && !cache[item.uid],
        )
        const change = Object.fromEntries(items.map((item) => [item.uid, true]))

        Promise.allSettled(items.map((item) => updateOne(item.uid))).then(resolve)
        return { ...cache, ...change }
      })
    })
  })

  const onCopyLink = async () => {
    const text = await readText()
    if (text) setUrl(text)
  }

  const {
    batchMode,
    selectedProfiles,
    toggleBatchMode,
    toggleProfileSelection,
    selectAllProfiles,
    clearAllSelections,
    isAllSelected,
    getSelectionState,
    deleteSelectedProfiles,
  } = useProfileBatchSelection({
    profileItems,
    currentProfileId: profiles.current,
    setActivatings,
    mutateProfiles,
    mutateLogs,
    onEnhance,
  })

  useProfileChangeListener({ mutateProfiles })

  return (
    <BasePage
      full
      title={t('profiles.page.title')}
      contentStyle={{ height: '100%' }}
    >
      <Box className="flex h-full flex-col overflow-hidden">
        <Box className="shrink-0 px-[10px] pb-1 pt-2">
          <ProfileHeader
            batchMode={batchMode}
            error={error}
            isStale={isStale}
            selectedCount={selectedProfiles.size}
            isAllSelected={isAllSelected}
            getSelectionState={getSelectionState}
            clearAllSelections={clearAllSelections}
            selectAllProfiles={selectAllProfiles}
            toggleBatchMode={toggleBatchMode}
            onUpdateAll={onUpdateAll}
            onOpenConfig={() => configRef.current?.open()}
            onReactivate={() => onEnhance(true)}
            onEmergencyRefresh={onEmergencyRefresh}
            onDeleteSelectedProfiles={deleteSelectedProfiles}
            onOpenMerge={() => setMergeOpen(true)}
            onOpenScript={() => setScriptOpen(true)}
            url={url}
            setUrl={setUrl}
            disabled={disabled}
            loading={loading}
            onImport={onImport}
            onCopyLink={onCopyLink}
            onCreate={() => viewerRef.current?.create()}
          />
        </Box>

        <ProfileCardsSection
          profileItems={profileItems}
          currentProfileId={profiles.current}
          activatings={activatings}
          batchMode={batchMode}
          selectedProfiles={selectedProfiles}
          mutateProfiles={mutateProfiles}
          onDragEnd={onDragEnd}
          onSelect={onSelect}
          onEdit={(item) => viewerRef.current?.edit(item)}
          onSave={async (item, prev, curr) => {
            if (prev !== curr && profiles.current === item.uid) {
              await onEnhance(false)
            }
          }}
          onDelete={onDelete}
          onToggleSelection={toggleProfileSelection}
        />

        <Box className="flex-[2_0_0] min-h-0">
          <ProfileRulesPanel />
        </Box>
      </Box>

      <ProfileViewer
        ref={viewerRef}
        onChange={async (isActivating) => {
          void mutateProfiles()
          if (isActivating) {
            await onEnhance(false)
          }
        }}
      />
      <ConfigViewer ref={configRef} />

      <ProfileMore
        id="Merge"
        open={mergeOpen}
        onClose={() => setMergeOpen(false)}
        onSave={async (prev, curr) => {
          if (prev !== curr) {
            await onEnhance(false)
          }
        }}
      />
      <ProfileMore
        id="Script"
        open={scriptOpen}
        onClose={() => setScriptOpen(false)}
        logInfo={chainLogs['Script']}
        onSave={async (prev, curr) => {
          if (prev !== curr) {
            await onEnhance(false)
          }
        }}
      />
    </BasePage>
  )
}

export default ProfilePage
