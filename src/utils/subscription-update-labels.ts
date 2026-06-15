import type {
  SubscriptionUpdateStage,
  SubscriptionUpdateTransportKind,
} from '@/types/subscription-update'

type TranslateFunction = (
  key: string,
  options?: Record<string, unknown>,
) => string

export const getSubscriptionStageLabel = (
  stage: SubscriptionUpdateStage,
  t: TranslateFunction,
) => {
  switch (stage) {
    case 'resolve_source':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.resolveSource',
        {
          defaultValue: 'Resolve source',
        },
      )
    case 'resolve_transport_plan':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.resolveTransportPlan',
        { defaultValue: 'Resolve transport plan' },
      )
    case 'fetch_payload':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.fetchPayload',
        {
          defaultValue: 'Fetch payload',
        },
      )
    case 'decode_payload':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.decodePayload',
        {
          defaultValue: 'Decode payload',
        },
      )
    case 'materialize_artifact':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.materializeArtifact',
        { defaultValue: 'Materialize artifact' },
      )
    case 'generate_runtime_config_candidate':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.generateRuntimeConfigCandidate',
        { defaultValue: 'Generate runtime candidate' },
      )
    case 'validate_runtime_candidate':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.validateRuntimeCandidate',
        { defaultValue: 'Validate runtime candidate' },
      )
    case 'publish_artifact':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.publishArtifact',
        {
          defaultValue: 'Publish artifact',
        },
      )
    case 'activate_runtime':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.activateRuntime',
        {
          defaultValue: 'Activate runtime',
        },
      )
    case 'emit_final_result':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.emitFinalResult',
        {
          defaultValue: 'Emit final result',
        },
      )
  }
}

export const getSubscriptionTransportLabel = (
  transport: SubscriptionUpdateTransportKind,
  t: TranslateFunction,
) => {
  switch (transport) {
    case 'direct':
      return t('profiles.page.feedback.subscriptionUpdate.transports.direct', {
        defaultValue: 'Direct connection',
      })
    case 'local_proxy':
      return t(
        'profiles.page.feedback.subscriptionUpdate.transports.localProxy',
        {
          defaultValue: 'Clash proxy',
        },
      )
    case 'system_proxy':
      return t(
        'profiles.page.feedback.subscriptionUpdate.transports.systemProxy',
        {
          defaultValue: 'System proxy',
        },
      )
  }
}
