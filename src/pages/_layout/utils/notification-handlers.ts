import { restartCore } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { queryClient } from '@/services/query-client'
import type {
  SubscriptionUpdateEvent,
  SubscriptionUpdateStage,
  SubscriptionUpdateTransportKind,
} from '@/types/subscription-update'

type NavigateFunction = (path: string, options?: any) => void
type TranslateFunction = (
  key: string,
  options?: Record<string, unknown>,
) => string

const CORE_PANIC_NOTICE_WINDOW_MS = 8000

let lastCorePanicNoticeAt = 0
let pendingCoreRestartTimer: ReturnType<typeof setTimeout> | null = null

const resolveSubscriptionProfile = (sourceId: string) => {
  const profiles = queryClient.getQueryData<IProfilesView>(['getProfiles'])
  const item = profiles?.items?.find((entry) => entry?.uid === sourceId)
  const isCurrent =
    profiles?.current === sourceId || profiles?.currentPrimaryUid === sourceId

  return {
    name: item?.name || sourceId,
    isCurrent,
  }
}

const getSubscriptionStageLabel = (
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
    case 'activate_runtime':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.activateRuntime',
        { defaultValue: 'Activate runtime' },
      )
    case 'emit_final_result':
      return t(
        'profiles.page.feedback.subscriptionUpdate.stages.emitFinalResult',
        { defaultValue: 'Emit final result' },
      )
  }
}

const getSubscriptionTransportLabel = (
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
        { defaultValue: 'System proxy' },
      )
  }
}

export const handleSubscriptionUpdateEvent = (
  event: SubscriptionUpdateEvent,
  t: TranslateFunction,
) => {
  if (event.kind !== 'update_finished') {
    return
  }

  const profile = resolveSubscriptionProfile(event.source_id)
  const stage = getSubscriptionStageLabel(event.stage, t)
  const transport = event.transport
    ? getSubscriptionTransportLabel(event.transport, t)
    : undefined

  if (event.final_status === 'failed') {
    if (event.trigger === 'automatic' && !profile.isCurrent) {
      return
    }

    const messageKey = transport
      ? 'profiles.page.feedback.subscriptionUpdate.failedWithTransport'
      : 'profiles.page.feedback.subscriptionUpdate.failed'

    const rawMessage = event.error?.message?.trim()

    if (rawMessage) {
      showNotice.error(
        messageKey,
        {
          name: profile.name,
          stage,
          transport,
        },
        rawMessage,
      )
    } else {
      showNotice.error(messageKey, {
        name: profile.name,
        stage,
        transport,
      })
    }
    return
  }

  if (
    event.trigger === 'manual' &&
    event.transport &&
    event.transport !== 'direct'
  ) {
    showNotice.success(
      'profiles.page.feedback.subscriptionUpdate.succeededViaTransport',
      {
        name: profile.name,
        transport,
      },
    )
  }
}

export const handleNoticeMessage = (
  status: string,
  msg: string,
  _t: TranslateFunction,
  navigate: NavigateFunction,
) => {
  const handlers: Record<string, () => void> = {
    'import_sub_url::ok': () => {
      navigate('/profile')
      showNotice.success(
        'shared.feedback.notifications.importSubscriptionSuccess',
      )
    },
    'import_sub_url::error': () => {
      navigate('/profile')
      showNotice.error(msg)
    },
    'set_config::error': () => showNotice.error(msg),
    'reactivate_profiles::error': () => showNotice.error(msg),
    update_failed: () => showNotice.error(msg),
    'config_validate::boot_error': () =>
      showNotice.error('shared.feedback.validation.config.bootFailed', msg),
    'config_validate::core_change': () =>
      showNotice.error(
        'shared.feedback.validation.config.coreChangeFailed',
        msg,
      ),
    'config_validate::error': () =>
      showNotice.error('shared.feedback.validation.config.failed', msg),
    'config_validate::process_terminated': () =>
      showNotice.error('shared.feedback.validation.config.processTerminated'),
    'config_validate::stdout_error': () =>
      showNotice.error('shared.feedback.validation.config.failed', msg),
    'config_validate::script_error': () =>
      showNotice.error('shared.feedback.validation.script.fileError', msg),
    'config_validate::script_syntax_error': () =>
      showNotice.error('shared.feedback.validation.script.syntaxError', msg),
    'config_validate::script_missing_main': () =>
      showNotice.error('shared.feedback.validation.script.missingMain', msg),
    'config_validate::file_not_found': () =>
      showNotice.error('shared.feedback.validation.script.fileNotFound', msg),
    'config_validate::yaml_syntax_error': () =>
      showNotice.error('shared.feedback.validation.yaml.syntaxError', msg),
    'config_validate::yaml_read_error': () =>
      showNotice.error('shared.feedback.validation.yaml.readError', msg),
    'config_validate::yaml_mapping_error': () =>
      showNotice.error('shared.feedback.validation.yaml.mappingError', msg),
    'config_validate::yaml_key_error': () =>
      showNotice.error('shared.feedback.validation.yaml.keyError', msg),
    'config_validate::yaml_error': () =>
      showNotice.error('shared.feedback.validation.yaml.generalError', msg),
    'config_validate::merge_syntax_error': () =>
      showNotice.error('shared.feedback.validation.merge.syntaxError', msg),
    'config_validate::merge_mapping_error': () =>
      showNotice.error('shared.feedback.validation.merge.mappingError', msg),
    'config_validate::merge_key_error': () =>
      showNotice.error('shared.feedback.validation.merge.keyError', msg),
    'config_validate::merge_error': () =>
      showNotice.error('shared.feedback.validation.merge.generalError', msg),
    core_panic_recovered: () => {
      const now = Date.now()

      if (now - lastCorePanicNoticeAt >= CORE_PANIC_NOTICE_WINDOW_MS) {
        lastCorePanicNoticeAt = now
        showNotice.error(msg)
      }

      if (pendingCoreRestartTimer) {
        return
      }

      pendingCoreRestartTimer = setTimeout(() => {
        pendingCoreRestartTimer = null
        restartCore().catch((err) => {
          console.error('自动重启内核失败:', err)
        })
      }, 2000)
    },
  }

  const handler = handlers[status]
  if (handler) {
    handler()
  } else {
    console.warn(`未处理的通知状态: ${status}`)
  }
}
