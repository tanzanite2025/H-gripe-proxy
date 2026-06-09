import { useLockFn } from 'ahooks'
import { Clock3, Wifi } from 'lucide-react'
import { useEffect, useMemo, useState, type ChangeEvent, type FormEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { Box, Button, InputAdornment, TextField } from '@/components/tailwind'
import { useVerge } from '@/hooks/system'
import { showNotice } from '@/services/notice-service'

const DEFAULT_TIMEOUT = 10000

const parsePositiveInt = (value: string, fallback: number) => {
  const parsed = parseInt(value, 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
}

interface DelaySettingsFormState {
  defaultLatencyTest: string
  defaultLatencyTimeout: number
}

const createDelaySettingsState = (
  verge?: IVergeConfig | null,
): DelaySettingsFormState => ({
  defaultLatencyTest: verge?.default_latency_test || '',
  defaultLatencyTimeout: verge?.default_latency_timeout || DEFAULT_TIMEOUT,
})

export function ProxyDelaySettings() {
  const { t } = useTranslation()
  const { verge, patchVerge } = useVerge()
  const [values, setValues] = useState<DelaySettingsFormState>(() =>
    createDelaySettingsState(verge),
  )

  useEffect(() => {
    setValues(createDelaySettingsState(verge))
  }, [verge])

  const isDirty = useMemo(
    () =>
      values.defaultLatencyTest !== (verge?.default_latency_test || '') ||
      values.defaultLatencyTimeout !==
        (verge?.default_latency_timeout || DEFAULT_TIMEOUT),
    [values, verge?.default_latency_test, verge?.default_latency_timeout],
  )

  const onReset = () => {
    setValues(createDelaySettingsState(verge))
  }

  const onSave = useLockFn(async () => {
    try {
      await patchVerge({
        default_latency_test: values.defaultLatencyTest.trim(),
        default_latency_timeout: values.defaultLatencyTimeout,
      })
      showNotice.success('shared.feedback.notifications.saved', 1000)
    } catch (error) {
      showNotice.error(error)
    }
  })

  const onSubmit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault()
    if (!isDirty) return
    void onSave()
  }

  return (
    <form
      onSubmit={onSubmit}
      className="mx-3 mb-3 rounded-2xl border border-gray-200/70 bg-white/70 px-4 py-3 dark:border-gray-700/70 dark:bg-gray-900/40"
    >
      <Box className="flex flex-col gap-4">
        <Box className="flex min-w-0 flex-col justify-center">
          <Box className="flex items-center gap-2 text-sm font-semibold text-gray-800 dark:text-gray-100">
            <Wifi className="h-4 w-4 text-sky-500" />
            <span>{t('proxies.page.tooltips.delayCheck')}</span>
          </Box>
          <Box className="mt-1 text-xs text-gray-500 dark:text-gray-400">
            {t('proxies.page.tooltips.delayCheckUrl')}
          </Box>
        </Box>

        <Box className="grid grid-cols-1 gap-3 lg:grid-cols-[minmax(0,1fr)_180px]">
          <Box className="min-w-0">
            <TextField
              label={t('proxies.page.tooltips.delayCheckUrl')}
              size="small"
              fullWidth
              autoComplete="new-password"
              autoCorrect="off"
              autoCapitalize="off"
              spellCheck="false"
              value={values.defaultLatencyTest}
              placeholder={t('proxies.page.placeholders.delayCheckUrl')}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                setValues((current) => ({
                  ...current,
                  defaultLatencyTest: event.target.value,
                }))
              }
            />
          </Box>

          <Box className="min-w-0">
            <TextField
              label={t('shared.labels.timeout')}
              size="small"
              type="number"
              fullWidth
              autoComplete="new-password"
              value={values.defaultLatencyTimeout}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                setValues((current) => ({
                  ...current,
                  defaultLatencyTimeout: parsePositiveInt(
                    event.target.value,
                    DEFAULT_TIMEOUT,
                  ),
                }))
              }
              slotProps={{
                input: {
                  startAdornment: <Clock3 className="h-4 w-4 text-gray-400" />,
                  endAdornment: (
                    <InputAdornment position="end">
                      {t('shared.units.milliseconds')}
                    </InputAdornment>
                  ),
                },
              }}
            />
          </Box>
        </Box>

        <Box className="flex flex-wrap items-center justify-end gap-2 border-t border-gray-200/70 pt-3 dark:border-gray-700/70">
          <Button
            type="button"
            size="small"
            variant="text"
            disabled={!isDirty}
            onClick={onReset}
          >
            {t('shared.actions.cancel')}
          </Button>
          <Button
            type="submit"
            size="small"
            variant="outlined"
            disabled={!isDirty}
            loading={false}
          >
            {t('shared.actions.save')}
          </Button>
        </Box>
      </Box>
    </form>
  )
}
