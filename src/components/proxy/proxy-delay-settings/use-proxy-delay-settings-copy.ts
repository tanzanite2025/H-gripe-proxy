import { useTranslation } from 'react-i18next'

export interface ProxyDelaySettingsCopy {
  cancelLabel: string
  delayCheckLabel: string
  delayCheckUrlLabel: string
  delayCheckUrlPlaceholder: string
  millisecondsLabel: string
  saveLabel: string
  timeoutLabel: string
}

export function useProxyDelaySettingsCopy(): ProxyDelaySettingsCopy {
  const { t } = useTranslation()

  return {
    cancelLabel: t('shared.actions.cancel'),
    delayCheckLabel: t('proxies.page.tooltips.delayCheck'),
    delayCheckUrlLabel: t('proxies.page.tooltips.delayCheckUrl'),
    delayCheckUrlPlaceholder: t('proxies.page.placeholders.delayCheckUrl'),
    millisecondsLabel: t('shared.units.milliseconds'),
    saveLabel: t('shared.actions.save'),
    timeoutLabel: t('shared.labels.timeout'),
  }
}
