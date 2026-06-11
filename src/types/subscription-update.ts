export type SubscriptionUpdateTrigger = 'manual' | 'automatic'

export type SubscriptionUpdateStage =
  | 'resolve_source'
  | 'resolve_transport_plan'
  | 'fetch_payload'
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
