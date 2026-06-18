import { useCallback, useEffect, useRef, useState } from 'react'

import { closeAllRuntimeConnections } from '@/services/connection-runtime'
import { showNotice } from '@/services/notice-service'
import { debugLog } from '@/utils/misc'

import {
  debugProfileSwitch,
  isOperationAborted,
  isRequestOutdated,
} from './shared'

interface UseProfileActivationParams {
  currentProfileId?: string
  profiles?: IProfilesConfig
  activateSelected: (profileOverride?: IProfilesConfig) => Promise<void>
  patchProfiles: (
    value: Partial<IProfilesConfig>,
    signal?: AbortSignal,
    options?: { deferRefreshOnSuccess?: boolean },
  ) => Promise<boolean | undefined>
  mutateProfiles: () => Promise<unknown>
  refreshRules: () => Promise<unknown>
  refreshRuleProviders: () => Promise<unknown>
  mutateLogs: () => Promise<unknown>
}

export function useProfileActivation({
  currentProfileId,
  profiles,
  activateSelected,
  patchProfiles,
  mutateProfiles,
  refreshRules,
  refreshRuleProviders,
  mutateLogs,
}: UseProfileActivationParams) {
  const [activatings, setActivatings] = useState<string[]>([])
  const activeProfileId = profiles?.current

  const switchingProfileRef = useRef<string | null>(null)
  const abortControllerRef = useRef<AbortController | null>(null)
  const requestSequenceRef = useRef<number>(0)
  const pendingRequestRef = useRef<Promise<unknown> | null>(null)

  const getCurrentActivatings = useCallback(
    () => [...new Set([activeProfileId ?? ''])].filter(Boolean),
    [activeProfileId],
  )

  const handleProfileInterrupt = useCallback(
    (previousSwitching: string, newProfile: string) => {
      debugProfileSwitch(
        'INTERRUPT_PREVIOUS',
        previousSwitching,
        `interrupted_by=${newProfile}`,
      )

      if (abortControllerRef.current) {
        abortControllerRef.current.abort()
        debugProfileSwitch(
          'ABORT_CONTROLLER_TRIGGERED',
          previousSwitching,
        )
      }

      if (pendingRequestRef.current) {
        debugProfileSwitch('CANCEL_PENDING_REQUEST', previousSwitching)
      }

      setActivatings((prev) => prev.filter((id) => id !== previousSwitching))
      showNotice.info(
        'profiles.page.feedback.notifications.switchInterrupted',
        `${previousSwitching} -> ${newProfile}`,
        3000,
      )
    },
    [],
  )

  const cleanupSwitchState = useCallback(
    (profile: string, sequence: number) => {
      setActivatings((prev) => prev.filter((id) => id !== profile))
      switchingProfileRef.current = null
      abortControllerRef.current = null
      pendingRequestRef.current = null
      debugProfileSwitch('SWITCH_END', profile, `sequence=${sequence}`)
    },
    [],
  )

  const executeBackgroundTasks = useCallback(
    async (
      profile: string,
      sequence: number,
      abortController: AbortController,
    ) => {
      try {
        if (
          sequence === requestSequenceRef.current &&
          switchingProfileRef.current === profile &&
          !abortController.signal.aborted
        ) {
          await activateSelected(profiles)
          debugLog(
            `[Profile] Background activation finished, sequence=${sequence}`,
          )
        } else {
          debugProfileSwitch(
            'BACKGROUND_TASK_SKIPPED',
            profile,
            `sequence=${sequence}, latest=${requestSequenceRef.current}`,
          )
        }
      } catch (error: unknown) {
        console.warn('Failed to activate selected proxies:', error)
      }
    },
    [activateSelected, profiles],
  )

  const activateProfile = useCallback(
    async (profile: string, notifySuccess: boolean) => {
      if (activeProfileId === profile && !notifySuccess) {
        debugLog(`[Profile] ${profile} is already current, skipping switch`)
        return
      }

      const currentSequence = ++requestSequenceRef.current
      debugProfileSwitch('NEW_REQUEST', profile, `sequence=${currentSequence}`)

      const previousSwitching = switchingProfileRef.current
      if (previousSwitching && previousSwitching !== profile) {
        handleProfileInterrupt(previousSwitching, profile)
      }

      if (switchingProfileRef.current === profile) {
        debugProfileSwitch('DUPLICATE_SWITCH_BLOCKED', profile)
        return
      }

      switchingProfileRef.current = profile
      debugProfileSwitch('SWITCH_START', profile, `sequence=${currentSequence}`)

      const currentAbortController = new AbortController()
      abortControllerRef.current = currentAbortController

      setActivatings((prev) => {
        if (prev.includes(profile)) return prev
        return [...prev, profile]
      })

      try {
        debugLog(
          `[Profile] Switching to ${profile}, sequence=${currentSequence}`,
        )

        if (
          isRequestOutdated(currentSequence, requestSequenceRef, profile) ||
          isOperationAborted(currentAbortController, profile)
        ) {
          return
        }

        const requestPromise = patchProfiles(
          { current: profile },
          currentAbortController.signal,
          {
            deferRefreshOnSuccess: true,
          },
        )
        pendingRequestRef.current = requestPromise

        const success = await requestPromise

        if (pendingRequestRef.current === requestPromise) {
          pendingRequestRef.current = null
        }

        if (
          isRequestOutdated(currentSequence, requestSequenceRef, profile) ||
          isOperationAborted(currentAbortController, profile)
        ) {
          return
        }

        await refreshRules()
        await refreshRuleProviders()
        await mutateLogs()
        closeAllRuntimeConnections()

        if (notifySuccess && success) {
          showNotice.success(
            'profiles.page.feedback.notifications.profileSwitched',
            1000,
          )
        }

        debugLog(
          `[Profile] Switched to ${profile}, scheduling background activation`,
        )

        window.setTimeout(
          () =>
            void executeBackgroundTasks(
              profile,
              currentSequence,
              currentAbortController,
            ),
          50,
        )
      } catch (error: any) {
        if (pendingRequestRef.current) {
          pendingRequestRef.current = null
        }

        if (
          isOperationAborted(currentAbortController, profile) ||
          isRequestOutdated(currentSequence, requestSequenceRef, profile)
        ) {
          return
        }

        console.error('[Profile] Switch failed:', error)
        showNotice.error(error, 4000)
      } finally {
        if (
          switchingProfileRef.current === profile &&
          currentSequence === requestSequenceRef.current
        ) {
          cleanupSwitchState(profile, currentSequence)
        } else {
          debugProfileSwitch(
            'CLEANUP_SKIPPED',
            profile,
            `sequence=${currentSequence}, latest=${requestSequenceRef.current}`,
          )
        }
      }
    },
    [
      cleanupSwitchState,
      executeBackgroundTasks,
      handleProfileInterrupt,
      mutateLogs,
      patchProfiles,
      activeProfileId,
      refreshRuleProviders,
      refreshRules,
    ],
  )

  const onSelect = useCallback(
    async (profile: string, force: boolean) => {
      if (switchingProfileRef.current === profile) {
        debugProfileSwitch('DUPLICATE_CLICK_IGNORED', profile)
        return
      }

      if (!force && profile === activeProfileId) {
        debugProfileSwitch('ALREADY_CURRENT_IGNORED', profile)
        return
      }

      await activateProfile(profile, true)
    },
    [activateProfile, activeProfileId],
  )

  useEffect(() => {
    void (async () => {
      if (currentProfileId) {
        void mutateProfiles()
        await activateProfile(currentProfileId, false)
      }
    })()
  }, [activateProfile, currentProfileId, mutateProfiles])

  useEffect(() => {
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort()
        debugProfileSwitch('COMPONENT_UNMOUNT_CLEANUP', 'all')
      }
    }
  }, [])

  return {
    activatings,
    setActivatings,
    switchingProfileRef,
    getCurrentActivatings,
    activateProfile,
    onSelect,
  }
}
