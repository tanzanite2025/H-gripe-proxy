import { Box } from '@/components/tailwind'

import { ProxyDelaySettingsActions } from './proxy-delay-settings/proxy-delay-settings-actions'
import { ProxyDelaySettingsFields } from './proxy-delay-settings/proxy-delay-settings-fields'
import { useProxyDelaySettingsCopy } from './proxy-delay-settings/use-proxy-delay-settings-copy'
import { useProxyDelaySettingsController } from './proxy-delay-settings/use-proxy-delay-settings-controller'

export function ProxyDelaySettings() {
  const copy = useProxyDelaySettingsCopy()
  const controller = useProxyDelaySettingsController()

  return (
    <form
      onSubmit={controller.handleSubmit}
      className="mx-3 mb-3 rounded-2xl border border-gray-200/70 bg-white/70 px-4 py-3 dark:border-gray-700/70 dark:bg-gray-900/40"
    >
      <Box className="flex flex-col gap-4">
        <ProxyDelaySettingsFields
          copy={copy}
          values={controller.values}
          onLatencyTestChange={controller.handleLatencyTestChange}
          onLatencyTimeoutChange={controller.handleLatencyTimeoutChange}
        />

        <ProxyDelaySettingsActions
          copy={copy}
          disabled={!controller.isDirty || controller.saving}
          saving={controller.saving}
          onReset={controller.handleReset}
        />
      </Box>
    </form>
  )
}
