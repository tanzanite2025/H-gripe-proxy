import { useQuery } from '@tanstack/react-query'
import dayjs from 'dayjs'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog } from '@/components/base'
import { Button } from '@/components/tailwind/Button'
import { CircularProgress } from '@/components/tailwind/CircularProgress'
import {
  getSubscriptionArtifactContent,
  getSubscriptionArtifactDiagnostics,
  getSubscriptionSourceUpdateEvents,
  listSubscriptionArtifactSummaries,
} from '@/services/cmds/subscriptions'
import { showNotice } from '@/services/notice-service'
import type {
  SubscriptionArtifactContentKind,
  SubscriptionArtifactSummary,
  SubscriptionUpdateEvent,
} from '@/types/subscription-update'
import {
  getSubscriptionStageLabel,
  getSubscriptionTransportLabel,
} from '@/utils/subscription-update-labels'

interface SubscriptionUpdateHistoryDialogProps {
  open: boolean
  sourceId: string
  profileName: string
  onClose: () => void
}

type ArtifactPreview = {
  title: string
  content: string
}

const formatTimestamp = (timestamp?: number | null) =>
  timestamp ? dayjs(timestamp).format('YYYY-MM-DD HH:mm:ss') : '-'

const shortVersion = (version: string) => version.slice(0, 12)

export function SubscriptionUpdateHistoryDialog({
  open,
  sourceId,
  profileName,
  onClose,
}: SubscriptionUpdateHistoryDialogProps) {
  const { t } = useTranslation()
  const [preview, setPreview] = useState<ArtifactPreview | null>(null)
  const [previewLoading, setPreviewLoading] = useState(false)

  const { data: events = [], isLoading: eventsLoading } = useQuery({
    queryKey: ['getSubscriptionSourceUpdateEvents', sourceId],
    queryFn: () => getSubscriptionSourceUpdateEvents(sourceId),
    enabled: open,
  })
  const { data: artifacts = [], isLoading: artifactsLoading } = useQuery({
    queryKey: ['listSubscriptionArtifactSummaries', sourceId],
    queryFn: () => listSubscriptionArtifactSummaries(sourceId),
    enabled: open,
  })

  const sortedArtifacts = useMemo(
    () =>
      [...artifacts].sort(
        (left, right) => right.artifact.fetched_at - left.artifact.fetched_at,
      ),
    [artifacts],
  )

  const loadArtifactPreview = async (
    artifact: SubscriptionArtifactSummary,
    kind: SubscriptionArtifactContentKind | 'diagnostics',
  ) => {
    setPreviewLoading(true)
    try {
      if (kind === 'diagnostics') {
        const diagnostics = await getSubscriptionArtifactDiagnostics(
          sourceId,
          artifact.artifact.version,
        )
        setPreview({
          title: `${shortVersion(artifact.artifact.version)} diagnostics`,
          content: diagnostics
            ? JSON.stringify(diagnostics, null, 2)
            : 'No diagnostics found.',
        })
        return
      }

      const content = await getSubscriptionArtifactContent(
        sourceId,
        artifact.artifact.version,
        kind,
      )
      setPreview({
        title: `${shortVersion(artifact.artifact.version)} ${kind}`,
        content: content?.content ?? 'No artifact content found.',
      })
    } catch (error) {
      showNotice.error(error)
    } finally {
      setPreviewLoading(false)
    }
  }

  return (
    <BaseDialog
      title={t('profiles.components.profileItem.updateHistory.title', {
        name: profileName,
        defaultValue: '{{name}} update history',
      })}
      open={open}
      cancelBtn={t('shared.actions.close', { defaultValue: 'Close' })}
      disableOk
      panelStyle={{ width: 'min(960px, calc(100vw - 56px))' }}
      contentClassName="max-h-[72vh] overflow-auto"
      onCancel={onClose}
      onClose={onClose}
    >
      <div className="space-y-4 text-sm">
        <section className="space-y-2">
          <h3 className="font-semibold">
            {t('profiles.components.profileItem.updateHistory.latestAttempt', {
              defaultValue: 'Latest attempt timeline',
            })}
          </h3>
          {eventsLoading ? (
            <CircularProgress size={18} />
          ) : events.length > 0 ? (
            <div className="space-y-2">
              {events.map((event) => (
                <SubscriptionEventRow
                  key={`${event.attempt_id}-${event.kind}-${eventTime(event)}`}
                  event={event}
                />
              ))}
            </div>
          ) : (
            <p className="text-text-secondary">
              {t('profiles.components.profileItem.updateHistory.noEvents', {
                defaultValue: 'No update events recorded yet.',
              })}
            </p>
          )}
        </section>

        <section className="space-y-2">
          <h3 className="font-semibold">
            {t('profiles.components.profileItem.updateHistory.artifacts', {
              defaultValue: 'Artifacts and diagnostics',
            })}
          </h3>
          {artifactsLoading ? (
            <CircularProgress size={18} />
          ) : sortedArtifacts.length > 0 ? (
            <div className="space-y-2">
              {sortedArtifacts.map((artifact) => (
                <ArtifactRow
                  key={artifact.artifact.version}
                  artifact={artifact}
                  previewLoading={previewLoading}
                  onPreview={loadArtifactPreview}
                />
              ))}
            </div>
          ) : (
            <p className="text-text-secondary">
              {t('profiles.components.profileItem.updateHistory.noArtifacts', {
                defaultValue: 'No artifacts recorded yet.',
              })}
            </p>
          )}
        </section>

        {preview && (
          <section className="space-y-2">
            <div className="flex items-center justify-between gap-2">
              <h3 className="font-semibold">{preview.title}</h3>
              <Button
                variant="text"
                size="small"
                onClick={() => setPreview(null)}
              >
                {t('shared.actions.close', { defaultValue: 'Close' })}
              </Button>
            </div>
            <pre className="max-h-80 overflow-auto whitespace-pre-wrap break-words rounded-lg bg-black/10 p-3 text-xs">
              {preview.content}
            </pre>
          </section>
        )}
      </div>
    </BaseDialog>
  )
}

function SubscriptionEventRow({ event }: { event: SubscriptionUpdateEvent }) {
  const { t } = useTranslation()
  const stage =
    event.kind === 'attempt_started'
      ? t('profiles.components.profileItem.updateHistory.attemptStarted', {
          defaultValue: 'Attempt started',
        })
      : getSubscriptionStageLabel(event.stage, t)
  const transport =
    event.kind !== 'attempt_started' && event.transport
      ? getSubscriptionTransportLabel(event.transport, t)
      : undefined
  const status =
    event.kind === 'update_finished'
      ? event.final_status
      : event.kind === 'stage_changed'
        ? 'running'
        : 'started'

  return (
    <div className="rounded-lg border border-white/10 bg-white/5 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span
          className={[
            'rounded-full px-2 py-0.5 text-[11px] font-medium',
            status === 'succeeded'
              ? 'bg-green-500/15 text-green-500'
              : status === 'failed'
                ? 'bg-red-500/15 text-red-500'
                : 'bg-blue-500/15 text-blue-500',
          ].join(' ')}
        >
          {status}
        </span>
        <span className="font-medium">{stage}</span>
        <span className="text-text-secondary">
          {formatTimestamp(eventTime(event))}
        </span>
        {transport && <span className="text-text-secondary">{transport}</span>}
      </div>
      {event.kind === 'update_finished' && event.error?.message && (
        <p className="mt-2 break-words text-xs text-red-500">
          {event.error.message}
        </p>
      )}
      {event.kind === 'update_finished' && event.artifact_version && (
        <p className="mt-1 break-words text-xs text-text-secondary">
          artifact: {shortVersion(event.artifact_version)}
        </p>
      )}
    </div>
  )
}

function ArtifactRow({
  artifact,
  previewLoading,
  onPreview,
}: {
  artifact: SubscriptionArtifactSummary
  previewLoading: boolean
  onPreview: (
    artifact: SubscriptionArtifactSummary,
    kind: SubscriptionArtifactContentKind | 'diagnostics',
  ) => Promise<void>
}) {
  const { t } = useTranslation()

  return (
    <div className="rounded-lg border border-white/10 bg-white/5 p-3">
      <div className="flex flex-wrap items-center gap-2">
        <span className="font-medium">
          {shortVersion(artifact.artifact.version)}
        </span>
        {artifact.is_active && (
          <span className="rounded-full bg-green-500/15 px-2 py-0.5 text-[11px] font-medium text-green-500">
            {t('profiles.components.profileItem.updateHistory.active', {
              defaultValue: 'Active',
            })}
          </span>
        )}
        <span className="text-text-secondary">
          {formatTimestamp(artifact.artifact.fetched_at)}
        </span>
        {artifact.artifact.detected_format && (
          <span className="text-text-secondary">
            {artifact.artifact.detected_format}
          </span>
        )}
      </div>
      <div className="mt-2 flex flex-wrap gap-2">
        <Button
          variant="outlined"
          size="small"
          disabled={!artifact.has_raw_body || previewLoading}
          onClick={() => void onPreview(artifact, 'raw_body')}
        >
          {t('profiles.components.profileItem.updateHistory.rawBody', {
            defaultValue: 'Raw body',
          })}
        </Button>
        <Button
          variant="outlined"
          size="small"
          disabled={!artifact.has_normalized_yaml || previewLoading}
          onClick={() => void onPreview(artifact, 'normalized_yaml')}
        >
          {t('profiles.components.profileItem.updateHistory.normalizedYaml', {
            defaultValue: 'Normalized YAML',
          })}
        </Button>
        <Button
          variant="outlined"
          size="small"
          disabled={!artifact.has_diagnostics || previewLoading}
          onClick={() => void onPreview(artifact, 'diagnostics')}
        >
          {t('profiles.components.profileItem.updateHistory.diagnostics', {
            defaultValue: 'Diagnostics',
          })}
        </Button>
      </div>
    </div>
  )
}

function eventTime(event: SubscriptionUpdateEvent) {
  switch (event.kind) {
    case 'attempt_started':
      return event.started_at
    case 'stage_changed':
      return event.changed_at
    case 'update_finished':
      return event.finished_at
  }
}
