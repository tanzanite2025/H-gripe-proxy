import { useLockFn } from 'ahooks'
import { useCallback, useState, type Dispatch, type SetStateAction } from 'react'

import { deleteProfile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import type { ProfileSelectionState } from './shared'

interface UseProfileBatchSelectionParams {
  profileItems: IProfileItem[]
  currentProfileId?: string
  setActivatings: Dispatch<SetStateAction<string[]>>
  mutateProfiles: () => Promise<unknown>
  mutateLogs: () => Promise<unknown>
  onEnhance: (notifySuccess: boolean) => Promise<void>
}

export function useProfileBatchSelection({
  profileItems,
  currentProfileId,
  setActivatings,
  mutateProfiles,
  mutateLogs,
  onEnhance,
}: UseProfileBatchSelectionParams) {
  const [batchMode, setBatchMode] = useState(false)
  const [selectedProfiles, setSelectedProfiles] = useState<Set<string>>(
    () => new Set(),
  )

  const toggleBatchMode = useCallback(() => {
    setBatchMode((current) => {
      if (!current) {
        setSelectedProfiles(new Set())
      }

      return !current
    })
  }, [])

  const toggleProfileSelection = useCallback((uid: string) => {
    setSelectedProfiles((prev) => {
      const next = new Set(prev)
      if (next.has(uid)) {
        next.delete(uid)
      } else {
        next.add(uid)
      }
      return next
    })
  }, [])

  const selectAllProfiles = useCallback(() => {
    setSelectedProfiles(new Set(profileItems.map((item) => item.uid)))
  }, [profileItems])

  const clearAllSelections = useCallback(() => {
    setSelectedProfiles(new Set())
  }, [])

  const isAllSelected = useCallback(
    () =>
      profileItems.length > 0 && profileItems.length === selectedProfiles.size,
    [profileItems.length, selectedProfiles.size],
  )

  const getSelectionState = useCallback((): ProfileSelectionState => {
    if (selectedProfiles.size === 0) {
      return 'none'
    }

    if (selectedProfiles.size === profileItems.length) {
      return 'all'
    }

    return 'partial'
  }, [profileItems.length, selectedProfiles.size])

  const deleteSelectedProfiles = useLockFn(async () => {
    if (selectedProfiles.size === 0) return

    try {
      const currentActivating =
        currentProfileId && selectedProfiles.has(currentProfileId)
          ? [currentProfileId]
          : []

      setActivatings((prev) => [...new Set([...prev, ...currentActivating])])

      for (const uid of selectedProfiles) {
        await deleteProfile(uid)
      }

      await mutateProfiles()
      await mutateLogs()

      if (currentActivating.length > 0) {
        await onEnhance(false)
      }

      setSelectedProfiles(new Set())
      setBatchMode(false)

      showNotice.success('profiles.page.feedback.notifications.batchDeleted')
    } catch (error: any) {
      showNotice.error(error)
    } finally {
      setActivatings([])
    }
  })

  return {
    batchMode,
    selectedProfiles,
    toggleBatchMode,
    toggleProfileSelection,
    selectAllProfiles,
    clearAllSelections,
    isAllSelected,
    getSelectionState,
    deleteSelectedProfiles,
  }
}
