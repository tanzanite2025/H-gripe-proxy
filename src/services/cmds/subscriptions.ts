import { invoke } from '@tauri-apps/api/core'

import type {
  SubscriptionArtifactCleanupResult,
  SubscriptionArtifactContent,
  SubscriptionArtifactContentKind,
  SubscriptionArtifactDiagnostics,
  SubscriptionArtifactMetadata,
  SubscriptionArtifactSummary,
  SubscriptionSource,
  SubscriptionSourceState,
  SubscriptionStateDocument,
  SubscriptionTransportPlan,
  SubscriptionUpdateEvent,
} from '@/types/subscription-update'

export async function getSubscriptionState() {
  return invoke<SubscriptionStateDocument>('get_subscription_state')
}

export async function listSubscriptionSources() {
  return invoke<SubscriptionSource[]>('list_subscription_sources')
}

export async function getSubscriptionSource(sourceId: string) {
  return invoke<SubscriptionSource | null>('get_subscription_source', {
    sourceId,
  })
}

export async function getSubscriptionSourceState(sourceId: string) {
  return invoke<SubscriptionSourceState | null>('get_subscription_source_state', {
    sourceId,
  })
}

export async function getSubscriptionSourceUpdateEvents(sourceId: string) {
  return invoke<SubscriptionUpdateEvent[]>(
    'get_subscription_source_update_events',
    { sourceId },
  )
}

export async function planSubscriptionUpdateTransport(sourceId: string) {
  return invoke<SubscriptionTransportPlan>('plan_subscription_update_transport', {
    sourceId,
  })
}

export async function getSubscriptionArtifactDiagnostics(
  sourceId: string,
  version: string,
) {
  return invoke<SubscriptionArtifactDiagnostics | null>(
    'get_subscription_artifact_diagnostics',
    { sourceId, version },
  )
}

export async function getSubscriptionArtifactMetadata(
  sourceId: string,
  version: string,
) {
  return invoke<SubscriptionArtifactMetadata | null>(
    'get_subscription_artifact_metadata',
    { sourceId, version },
  )
}

export async function getSubscriptionArtifactContent(
  sourceId: string,
  version: string,
  contentKind: SubscriptionArtifactContentKind,
) {
  return invoke<SubscriptionArtifactContent | null>(
    'get_subscription_artifact_content',
    { sourceId, version, contentKind },
  )
}

export async function listSubscriptionArtifacts(sourceId: string) {
  return invoke<SubscriptionArtifactMetadata[]>('list_subscription_artifacts', {
    sourceId,
  })
}

export async function listSubscriptionArtifactSummaries(sourceId: string) {
  return invoke<SubscriptionArtifactSummary[]>(
    'list_subscription_artifact_summaries',
    { sourceId },
  )
}

export async function cleanupSubscriptionArtifactsByRetention(
  sourceId: string,
  retainCount?: number,
) {
  return invoke<SubscriptionArtifactCleanupResult>(
    'cleanup_subscription_artifacts_by_retention',
    { sourceId, retainCount },
  )
}
