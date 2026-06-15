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
  detected_format?: string | null
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
      transport?: SubscriptionUpdateTransportKind | null
    }
  | {
      kind: 'update_finished'
      source_id: string
      attempt_id: string
      trigger: SubscriptionUpdateTrigger
      final_status: SubscriptionUpdateFinalStatus
      stage: SubscriptionUpdateStage
      transport?: SubscriptionUpdateTransportKind | null
      artifact_version?: string | null
      runtime_activated: boolean
      active_artifact_unchanged: boolean
      error?: SubscriptionUpdateErrorView | null
    }
