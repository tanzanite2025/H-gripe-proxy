import { Clock3, Wifi } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Box, InputAdornment, TextField } from '@/components/tailwind'

import type { ProxyDelaySettingsCopy } from './use-proxy-delay-settings-copy'
import type { DelaySettingsFormState } from './shared'

interface ProxyDelaySettingsFieldsProps {
  copy: ProxyDelaySettingsCopy
  values: DelaySettingsFormState
  onLatencyTestChange: (event: ChangeEvent<HTMLInputElement>) => void
  onLatencyTimeoutChange: (event: ChangeEvent<HTMLInputElement>) => void
}

export function ProxyDelaySettingsFields({
  copy,
  values,
  onLatencyTestChange,
  onLatencyTimeoutChange,
}: ProxyDelaySettingsFieldsProps) {
  const {
    delayCheckLabel,
    delayCheckUrlLabel,
    delayCheckUrlPlaceholder,
    millisecondsLabel,
    timeoutLabel,
  } = copy

  return (
    <>
      <Box className="flex min-w-0 flex-col justify-center">
        <Box className="flex items-center gap-2 text-sm font-semibold text-gray-800 dark:text-gray-100">
          <Wifi className="h-4 w-4 text-sky-500" />
          <span>{delayCheckLabel}</span>
        </Box>
        <Box className="mt-1 text-xs text-gray-500 dark:text-gray-400">
          {delayCheckUrlLabel}
        </Box>
      </Box>

      <Box className="grid grid-cols-1 gap-3 lg:grid-cols-[minmax(0,1fr)_180px]">
        <Box className="min-w-0">
          <TextField
            label={delayCheckUrlLabel}
            size="small"
            fullWidth
            autoComplete="new-password"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck="false"
            value={values.defaultLatencyTest}
            placeholder={delayCheckUrlPlaceholder}
            onChange={onLatencyTestChange}
          />
        </Box>

        <Box className="min-w-0">
          <TextField
            label={timeoutLabel}
            size="small"
            type="number"
            fullWidth
            autoComplete="new-password"
            value={values.defaultLatencyTimeout}
            onChange={onLatencyTimeoutChange}
            slotProps={{
              input: {
                startAdornment: <Clock3 className="h-4 w-4 text-gray-400" />,
                endAdornment: (
                  <InputAdornment position="end">
                    {millisecondsLabel}
                  </InputAdornment>
                ),
              },
            }}
          />
        </Box>
      </Box>
    </>
  )
}
