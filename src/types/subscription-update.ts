export type SubscriptionUpdateTrigger = 'manual' | 'automatic'

export type SubscriptionUpdateStage =
  | 'resolve_source'
  | 'resolve_transport_plan'
  | 'fetch_payload'
  | 'decode_payload'
  | 'materialize_artifact'
  | 'activate_runtime'
  | 'emit_final_result'

export type SubscriptionUpdateTransportKind =
  | 'direct'
  | 'local_proxy'
  | 'system_proxy'

export type SubscriptionUpdateFinalStatus = 'succeeded' | 'failed'

export type SubscriptionFormat =
  | 'clash_yaml'
  | 'base64_links'
  | 'sing_box'
  | 'html'
  | 'unknown_text'

export interface SubscriptionUpdateErrorView {
  message: string
}

export interface SubscriptionStageRecord {
  stage: SubscriptionUpdateStage
  changed_at: number
  transport?: SubscriptionUpdateTransportKind | null
}

export interface SubscriptionArtifactRecord {
  version: string
  content_hash: string
  fetched_at: number
  content_length: number
  content_type?: string | null
  detected_format?: SubscriptionFormat | null
}

export interface SubscriptionArtifactMetadata {
  source_id: string
  artifact: SubscriptionArtifactRecord
}

export interface SubscriptionArtifactSummary {
  source_id: string
  artifact: SubscriptionArtifactRecord
  has_diagnostics: boolean
  has_raw_body: boolean
  has_normalized_yaml: boolean
  is_active: boolean
}

export interface SubscriptionArtifactCleanupResult {
  source_id: string
  retain_count: number
  removed_versions: string[]
  kept_versions: string[]
  active_version_preserved: boolean
}

export type SubscriptionArtifactContentKind = 'raw_body' | 'normalized_yaml'

export interface SubscriptionArtifactContent {
  source_id: string
  version: string
  content_kind: SubscriptionArtifactContentKind
  content: string
}

export interface SubscriptionFormatDetection {
  format: SubscriptionFormat
  reason: string
  preview: string
  topLevelKeys: string[]
}

export interface SubscriptionResponseDiagnostics {
  statusCode: number
  contentType?: string | null
  contentLength: number
}

export interface SubscriptionArtifactDiagnostics {
  formatDetection: SubscriptionFormatDetection
  response: SubscriptionResponseDiagnostics
}

export interface SubscriptionAttemptRecord {
  attempt_id: string
  trigger: SubscriptionUpdateTrigger
  started_at: number
  finished_at: number
  final_status: SubscriptionUpdateFinalStatus
  stage: SubscriptionUpdateStage
  transport?: SubscriptionUpdateTransportKind | null
  artifact_version?: string | null
  error?: string | null
  runtime_activated: boolean
  active_artifact_unchanged: boolean
  stage_history: SubscriptionStageRecord[]
}

export interface SubscriptionSourceState {
  source_id: string
  active_artifact_version?: string | null
  latest_artifact?: SubscriptionArtifactRecord | null
  latest_attempt?: SubscriptionAttemptRecord | null
  latest_success?: SubscriptionAttemptRecord | null
}

export interface SubscriptionStateDocument {
  sources: SubscriptionSourceState[]
}

export type SubscriptionUpdateEvent =
  | {
      kind: 'attempt_started'
      source_id: string
      attempt_id: string
      trigger: SubscriptionUpdateTrigger
      started_at: number
    }
  | {
      kind: 'stage_changed'
      source_id: string
      attempt_id: string
      stage: SubscriptionUpdateStage
      changed_at: number
      transport?: SubscriptionUpdateTransportKind | null
    }
  | {
      kind: 'update_finished'
      source_id: string
      attempt_id: string
      trigger: SubscriptionUpdateTrigger
      finished_at: number
      final_status: SubscriptionUpdateFinalStatus
      stage: SubscriptionUpdateStage
      transport?: SubscriptionUpdateTransportKind | null
      artifact_version?: string | null
      runtime_activated: boolean
      active_artifact_unchanged: boolean
      error?: SubscriptionUpdateErrorView | null
    }
