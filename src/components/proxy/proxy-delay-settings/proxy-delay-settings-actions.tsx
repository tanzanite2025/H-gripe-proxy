import { Box, Button } from '@/components/tailwind'
import type { ProxyDelaySettingsCopy } from './use-proxy-delay-settings-copy'

interface ProxyDelaySettingsActionsProps {
  copy: Pick<ProxyDelaySettingsCopy, 'cancelLabel' | 'saveLabel'>
  disabled: boolean
  saving: boolean
  onReset: () => void
}

export function ProxyDelaySettingsActions({
  copy,
  disabled,
  saving,
  onReset,
}: ProxyDelaySettingsActionsProps) {
  const { cancelLabel, saveLabel } = copy

  return (
    <Box className="flex flex-wrap items-center justify-end gap-2 border-t border-gray-200/70 pt-3 dark:border-gray-700/70">
      <Button
        type="button"
        size="small"
        variant="text"
        disabled={disabled}
        onClick={onReset}
      >
        {cancelLabel}
      </Button>
      <Button
        type="submit"
        size="small"
        variant="outlined"
        disabled={disabled}
        loading={saving}
      >
        {saveLabel}
      </Button>
    </Box>
  )
}
