import { useQuery } from '@tanstack/react-query'
import dayjs from 'dayjs'
import { useCallback, useEffect, useMemo, useReducer, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { getSubscriptionSourceState } from '@/services/cmds/subscriptions'
import { useLoadingCache, useSetLoadingCache } from '@/services/states'
import type {
  SubscriptionAttemptRecord,
  SubscriptionSourceState,
  SubscriptionUpdateEvent,
} from '@/types/subscription-update'
import {
  getSubscriptionStageLabel,
  getSubscriptionTransportLabel,
} from '@/utils/subscription-update-labels'

import { formatExpireDate, parseProfileUrl } from './shared'
import { useNextUpdateDisplay } from './use-next-update-display'

interface UseProfileItemStateParams {
  itemData: IProfileItem
  mutateProfiles: () => Promise<void>
}

type SubscriptionStatusTone = 'info' | 'success' | 'error' | 'muted'

export interface SubscriptionStatusBadge {
  tone: SubscriptionStatusTone
  label: string
  title: string
}

const formatTimestamp = (timestamp?: number | null) =>
  timestamp ? dayjs(timestamp).format('YYYY-MM-DD HH:mm:ss') : undefined

const getArtifactShortVersion = (version?: string | null) =>
  version ? version.slice(0, 12) : undefined

export function useProfileItemState({
  itemData,
  mutateProfiles,
}: UseProfileItemStateParams) {
  const { t } = useTranslation()
  const loadingCache = useLoadingCache()
  const setLoadingCache = useSetLoadingCache()

  const { uid, name = 'Profile', extra, updated = 0, option } = itemData
  const hasUrl = !!itemData.url

  const { data: subscriptionSourceState, refetch: refetchSubscriptionState } =
    useQuery({
      queryKey: ['getSubscriptionSourceState', uid],
      queryFn: () => getSubscriptionSourceState(uid),
      enabled: hasUrl,
      staleTime: 15_000,
    })
  const [liveSubscriptionEvent, setLiveSubscriptionEvent] =
    useState<SubscriptionUpdateEvent | null>(null)

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
  const subscriptionStatus = useMemo(
    () =>
      buildSubscriptionStatusBadge({
        liveEvent: liveSubscriptionEvent,
        sourceState: subscriptionSourceState,
        t,
      }),
    [liveSubscriptionEvent, subscriptionSourceState, t],
  )

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
    const handleSubscriptionUpdate = (event: Event) => {
      const customEvent = event as CustomEvent<SubscriptionUpdateEvent>
      const detail = customEvent.detail

      if (!detail || detail.source_id !== uid) {
        return
      }

      if (detail.kind === 'attempt_started') {
        setLiveSubscriptionEvent(detail)
        setProfileLoading(true)
        return
      }

      if (detail.kind === 'stage_changed') {
        setLiveSubscriptionEvent(detail)
        setProfileLoading(true)
        return
      }

      if (detail.kind === 'update_finished') {
        setLiveSubscriptionEvent(detail)
        setProfileLoading(false)
        void refetchSubscriptionState()
        void mutateProfiles()

        if (showNextUpdate) {
          void refreshNextUpdateTime()
        }
      }
    }

    window.addEventListener('subscription-update', handleSubscriptionUpdate)

    return () => {
      window.removeEventListener(
        'subscription-update',
        handleSubscriptionUpdate,
      )
    }
  }, [
    mutateProfiles,
    refetchSubscriptionState,
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
    subscriptionStatus,
    setProfileLoading,
    showNextUpdate,
    nextUpdateTime,
    toggleUpdateTimeDisplay,
  }
}

function buildSubscriptionStatusBadge({
  liveEvent,
  sourceState,
  t,
}: {
  liveEvent: SubscriptionUpdateEvent | null
  sourceState?: SubscriptionSourceState | null
  t: (key: string, options?: Record<string, unknown>) => string
}): SubscriptionStatusBadge | undefined {
  if (liveEvent?.kind === 'attempt_started') {
    return {
      tone: 'info',
      label: t('profiles.components.profileItem.subscriptionStatus.updating', {
        defaultValue: 'Updating',
      }),
      title: t(
        'profiles.components.profileItem.subscriptionStatus.attemptStarted',
        {
          attempt: liveEvent.attempt_id,
          time: formatTimestamp(liveEvent.started_at),
          defaultValue: 'Update started\nAttempt: {{attempt}}\nTime: {{time}}',
        },
      ),
    }
  }

  if (liveEvent?.kind === 'stage_changed') {
    const stage = getSubscriptionStageLabel(liveEvent.stage, t)
    const transport = liveEvent.transport
      ? getSubscriptionTransportLabel(liveEvent.transport, t)
      : undefined

    return {
      tone: 'info',
      label: stage,
      title: [
        t('profiles.components.profileItem.subscriptionStatus.updating', {
          defaultValue: 'Updating',
        }),
        `${t('shared.labels.stage', { defaultValue: 'Stage' })}: ${stage}`,
        transport
          ? `${t('shared.labels.transport', { defaultValue: 'Transport' })}: ${transport}`
          : undefined,
        `${t('shared.labels.time', { defaultValue: 'Time' })}: ${formatTimestamp(liveEvent.changed_at)}`,
      ]
        .filter(Boolean)
        .join('\n'),
    }
  }

  if (liveEvent?.kind === 'update_finished') {
    const artifactVersion = getArtifactShortVersion(liveEvent.artifact_version)
    const stage = getSubscriptionStageLabel(liveEvent.stage, t)

    return buildFinishedSubscriptionStatusBadge({
      attempt: {
        attempt_id: liveEvent.attempt_id,
        trigger: liveEvent.trigger,
        started_at: liveEvent.finished_at,
        finished_at: liveEvent.finished_at,
        final_status: liveEvent.final_status,
        stage: liveEvent.stage,
        transport: liveEvent.transport,
        artifact_version: liveEvent.artifact_version,
        error: liveEvent.error?.message,
        runtime_activated: liveEvent.runtime_activated,
        active_artifact_unchanged: liveEvent.active_artifact_unchanged,
        stage_history: [],
      },
      artifactVersion,
      stage,
      t,
    })
  }

  const latestAttempt = sourceState?.latest_attempt
  if (latestAttempt?.final_status === 'failed') {
    return buildFinishedSubscriptionStatusBadge({
      attempt: latestAttempt,
      artifactVersion: getArtifactShortVersion(
        latestAttempt.artifact_version ?? sourceState?.active_artifact_version,
      ),
      stage: getSubscriptionStageLabel(latestAttempt.stage, t),
      t,
    })
  }

  const latestSuccess = sourceState?.latest_success
  if (latestSuccess) {
    return buildFinishedSubscriptionStatusBadge({
      attempt: latestSuccess,
      artifactVersion: getArtifactShortVersion(
        latestSuccess.artifact_version ?? sourceState?.active_artifact_version,
      ),
      stage: getSubscriptionStageLabel(latestSuccess.stage, t),
      t,
    })
  }

  const activeArtifactVersion = getArtifactShortVersion(
    sourceState?.active_artifact_version,
  )
  if (activeArtifactVersion) {
    return {
      tone: 'muted',
      label: t('profiles.components.profileItem.subscriptionStatus.active', {
        defaultValue: 'Active',
      }),
      title: `${t('shared.labels.artifact', { defaultValue: 'Artifact' })}: ${activeArtifactVersion}`,
    }
  }

  return undefined
}

function buildFinishedSubscriptionStatusBadge({
  attempt,
  artifactVersion,
  stage,
  t,
}: {
  attempt: SubscriptionAttemptRecord
  artifactVersion?: string
  stage: string
  t: (key: string, options?: Record<string, unknown>) => string
}): SubscriptionStatusBadge {
  const transport = attempt.transport
    ? getSubscriptionTransportLabel(attempt.transport, t)
    : undefined
  const finishedAt = formatTimestamp(attempt.finished_at)
  const tone: SubscriptionStatusTone =
    attempt.final_status === 'failed' ? 'error' : 'success'
  const label =
    attempt.final_status === 'failed'
      ? t('profiles.components.profileItem.subscriptionStatus.failedAt', {
          stage,
          defaultValue: 'Failed: {{stage}}',
        })
      : t('profiles.components.profileItem.subscriptionStatus.updated', {
          defaultValue: 'Updated',
        })

  return {
    tone,
    label,
    title: [
      `${t('shared.labels.status', { defaultValue: 'Status' })}: ${label}`,
      `${t('shared.labels.stage', { defaultValue: 'Stage' })}: ${stage}`,
      transport
        ? `${t('shared.labels.transport', { defaultValue: 'Transport' })}: ${transport}`
        : undefined,
      artifactVersion
        ? `${t('shared.labels.artifact', { defaultValue: 'Artifact' })}: ${artifactVersion}`
        : undefined,
      finishedAt
        ? `${t('shared.labels.time', { defaultValue: 'Time' })}: ${finishedAt}`
        : undefined,
      attempt.error
        ? `${t('shared.labels.error', { defaultValue: 'Error' })}: ${attempt.error}`
        : undefined,
    ]
      .filter(Boolean)
      .join('\n'),
  }
}
