import { useLockFn } from 'ahooks'
import type { ChangeEvent } from 'react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import {
  Box,
  Button,
  Card,
  InputAdornment,
  Select,
  SelectMenuItem,
  TextField,
} from '@/components/tailwind'
import { useClash } from '@/hooks/data'
import { useClashLog, useVerge } from '@/hooks/system'
import { openLogsDir } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import type { TranslationKey } from '@/types/generated/i18n-keys'
import { CORE_LOG_LEVEL_OPTIONS, normalizeCoreLogLevel } from '@/utils/log-level'

const APP_LOG_LEVEL_OPTIONS = [
  'trace',
  'debug',
  'info',
  'warn',
  'error',
  'silent',
] as const

const CORE_LOG_LEVEL_LABEL_KEYS: Record<LogLevel, TranslationKey> = {
  debug: 'settings.sections.clash.form.options.logLevel.debug',
  info: 'settings.sections.clash.form.options.logLevel.info',
  warning: 'settings.sections.clash.form.options.logLevel.warning',
  error: 'settings.sections.clash.form.options.logLevel.error',
  silent: 'settings.sections.clash.form.options.logLevel.silent',
}

const AUTO_LOG_CLEAN_DAY_OPTIONS = [1, 7, 30, 90] as const

interface LogSettingsValues {
  coreLogLevel: LogLevel
  appLogLevel: string
  appLogMaxSize: number
  appLogMaxCount: number
  autoLogClean: number
}

const createLogSettingsValues = (
  verge?: IVergeConfig | null,
  clash?: IConfigData | null,
): LogSettingsValues => ({
  coreLogLevel: normalizeCoreLogLevel(clash?.['log-level']),
  appLogLevel: verge?.app_log_level ?? 'warn',
  appLogMaxSize: verge?.app_log_max_size ?? 128,
  appLogMaxCount: verge?.app_log_max_count ?? 8,
  autoLogClean: verge?.auto_log_clean || 0,
})

const parsePositiveInt = (value: string, fallback: number) => {
  const parsed = parseInt(value, 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
}

export function LogSettingsPanel() {
  const { t } = useTranslation()
  const { verge, patchVerge } = useVerge()
  const { clash, patchClash } = useClash()
  const [, setClashLog] = useClashLog()
  const [values, setValues] = useState(() =>
    createLogSettingsValues(verge, clash),
  )

  useEffect(() => {
    setValues(createLogSettingsValues(verge, clash))
  }, [
    clash?.['log-level'],
    verge?.app_log_level,
    verge?.app_log_max_size,
    verge?.app_log_max_count,
    verge?.auto_log_clean,
  ])

  const onSave = useLockFn(async () => {
    try {
      await Promise.all([
        patchVerge({
          app_log_level: values.appLogLevel,
          app_log_max_size: values.appLogMaxSize,
          app_log_max_count: values.appLogMaxCount,
          auto_log_clean: values.autoLogClean as any,
        }),
        patchClash({
          'log-level': values.coreLogLevel,
        }),
      ])
      setClashLog((current) => ({
        ...current!,
        logLevel: values.coreLogLevel,
      }))
    } catch (error) {
      showNotice.error(error)
    }
  })

  return (
    <Card
      variant="outlined"
      className="mx-[10px] mb-3 overflow-hidden shadow-none"
    >
      <Box className="flex flex-wrap items-center justify-end gap-2 border-b border-divider px-4 py-3">
        <Button
          size="small"
          variant="outlined"
          color="inherit"
          onClick={openLogsDir}
        >
          {t('settings.components.verge.advanced.fields.openLogsDir')}
        </Button>
        <Button size="small" variant="primary" onClick={onSave}>
          {t('shared.actions.save')}
        </Button>
      </Box>

      <Box className="grid grid-cols-1 gap-3 p-4 md:grid-cols-2 xl:grid-cols-5">
        <Box className="flex min-w-0 flex-col gap-1">
          <Box as="label" className="px-[14px] text-xs text-text-secondary">
            {t('settings.sections.clash.form.fields.logLevel')}
          </Box>
          <Select
            size="small"
            value={values.coreLogLevel}
            onChange={(event) =>
              setValues((current) => ({
                ...current,
                coreLogLevel: event.target.value as LogLevel,
              }))
            }
          >
            {CORE_LOG_LEVEL_OPTIONS.map((option) => (
              <SelectMenuItem value={option} key={option}>
                {t(CORE_LOG_LEVEL_LABEL_KEYS[option])}
              </SelectMenuItem>
            ))}
          </Select>
        </Box>

        <Box className="flex min-w-0 flex-col gap-1">
          <Box as="label" className="px-[14px] text-xs text-text-secondary">
            {t('settings.modals.misc.fields.appLogLevel')}
          </Box>
          <Select
            size="small"
            value={values.appLogLevel}
            onChange={(event) =>
              setValues((current) => ({
                ...current,
                appLogLevel: event.target.value as string,
              }))
            }
          >
            {APP_LOG_LEVEL_OPTIONS.map((option) => (
              <SelectMenuItem value={option} key={option}>
                {option[0].toUpperCase() + option.slice(1).toLowerCase()}
              </SelectMenuItem>
            ))}
          </Select>
        </Box>

        <TextField
          autoComplete="new-password"
          size="small"
          type="number"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck="false"
          label={t('settings.modals.misc.fields.appLogMaxSize')}
          value={values.appLogMaxSize}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            setValues((current) => ({
              ...current,
              appLogMaxSize: parsePositiveInt(event.target.value, 128),
            }))
          }
          slotProps={{
            input: {
              endAdornment: (
                <InputAdornment position="end">
                  {t('shared.units.kilobytes')}
                </InputAdornment>
              ),
            },
          }}
        />

        <TextField
          autoComplete="new-password"
          size="small"
          type="number"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck="false"
          label={t('settings.modals.misc.fields.appLogMaxCount')}
          value={values.appLogMaxCount}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            setValues((current) => ({
              ...current,
              appLogMaxCount: parsePositiveInt(event.target.value, 1),
            }))
          }
          slotProps={{
            input: {
              endAdornment: (
                <InputAdornment position="end">
                  {t('shared.units.files')}
                </InputAdornment>
              ),
            },
          }}
        />

        <Box className="flex min-w-0 flex-col gap-1">
          <Box as="label" className="px-[14px] text-xs text-text-secondary">
            {t('settings.modals.misc.fields.autoLogClean')}
          </Box>
          <Select
            size="small"
            value={values.autoLogClean}
            onChange={(event) =>
              setValues((current) => ({
                ...current,
                autoLogClean: Number(event.target.value),
              }))
            }
          >
            <SelectMenuItem value={0}>
              {t('settings.modals.misc.options.autoLogClean.never')}
            </SelectMenuItem>
            {AUTO_LOG_CLEAN_DAY_OPTIONS.map((days, index) => (
              <SelectMenuItem key={days} value={index + 1}>
                {t('settings.modals.misc.options.autoLogClean.retainDays', {
                  n: days,
                })}
              </SelectMenuItem>
            ))}
          </Select>
        </Box>
      </Box>
    </Card>
  )
}
