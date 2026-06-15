import { invoke } from '@tauri-apps/api/core'

import type {
  SubscriptionArtifactContent,
  SubscriptionArtifactContentKind,
  SubscriptionArtifactDiagnostics,
  SubscriptionArtifactMetadata,
  SubscriptionSourceState,
  SubscriptionStateDocument,
} from '@/types/subscription-update'

export async function getSubscriptionState() {
  return invoke<SubscriptionStateDocument>('get_subscription_state')
}

export async function getSubscriptionSourceState(sourceId: string) {
  return invoke<SubscriptionSourceState | null>('get_subscription_source_state', {
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
