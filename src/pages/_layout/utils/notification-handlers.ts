import { restartCore } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

type NavigateFunction = (path: string, options?: any) => void
type TranslateFunction = (key: string) => string

const CORE_PANIC_NOTICE_WINDOW_MS = 8000

let lastCorePanicNoticeAt = 0
let pendingCoreRestartTimer: ReturnType<typeof setTimeout> | null = null

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
    update_with_clash_proxy: () =>
      showNotice.success(
        'settings.feedback.notifications.updater.withClashProxySuccess',
        msg,
      ),
    update_failed_even_with_clash: () =>
      showNotice.error(
        'settings.feedback.notifications.updater.withClashProxyFailed',
        msg,
      ),
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
