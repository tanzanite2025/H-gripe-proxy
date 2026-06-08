import type { ChangeEvent, Dispatch, SetStateAction } from 'react'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'
import {
  InputAdornment,
  Select,
  SelectMenuItem,
  TextField,
} from '@/components/tailwind'
import type { SelectChangeEvent } from '@/components/tailwind/Select'

import { MiscConfigFormRow } from './form-row'
import {
  APP_LOG_LEVEL_OPTIONS,
  AUTO_LOG_CLEAN_DAY_OPTIONS,
  PROXY_LAYOUT_COLUMN_OPTIONS,
} from './options'
import type { MiscConfigValues } from './types'

interface MiscConfigFormProps {
  values: MiscConfigValues
  setValues: Dispatch<SetStateAction<MiscConfigValues>>
}

const parsePositiveInt = (value: string, fallback: number) => {
  const parsed = parseInt(value, 10)
  return Number.isFinite(parsed) && parsed > 0 ? parsed : fallback
}

export function MiscConfigForm({
  values,
  setValues,
}: MiscConfigFormProps) {
  const { t } = useTranslation()

  return (
    <div className="flex flex-col gap-3">
      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.appLogLevel')}
      >
        <Select
          size="small"
          className="w-[120px]"
          value={values.appLogLevel}
          onChange={(event: SelectChangeEvent) =>
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
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.appLogMaxSize')}
      >
        <TextField
          autoComplete="new-password"
          size="small"
          type="number"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck="false"
          className="w-[140px]"
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
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.appLogMaxCount')}
      >
        <TextField
          autoComplete="new-password"
          size="small"
          type="number"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck="false"
          className="w-[140px]"
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
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.autoCloseConnections')}
        tooltip={t('settings.modals.misc.tooltips.autoCloseConnections')}
      >
        <Switch
          checked={values.autoCloseConnection}
          onCheckedChange={(checked) =>
            setValues((current) => ({
              ...current,
              autoCloseConnection: checked,
            }))
          }
        />
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.autoCheckUpdate')}
      >
        <Switch
          checked={values.autoCheckUpdate}
          onCheckedChange={(checked) =>
            setValues((current) => ({
              ...current,
              autoCheckUpdate: checked,
            }))
          }
        />
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.enableBuiltinEnhanced')}
        tooltip={t('settings.modals.misc.tooltips.enableBuiltinEnhanced')}
      >
        <Switch
          checked={values.enableBuiltinEnhanced}
          onCheckedChange={(checked) =>
            setValues((current) => ({
              ...current,
              enableBuiltinEnhanced: checked,
            }))
          }
        />
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.proxyLayoutColumns')}
      >
        <Select
          size="small"
          className="w-[160px]"
          value={values.proxyLayoutColumn}
          onChange={(event: SelectChangeEvent) =>
            setValues((current) => ({
              ...current,
              proxyLayoutColumn: Number(event.target.value),
            }))
          }
        >
          {PROXY_LAYOUT_COLUMN_OPTIONS.map((option) => (
            <SelectMenuItem value={option} key={option}>
              {option === 6
                ? t('settings.modals.misc.options.proxyLayoutColumns.auto')
                : option}
            </SelectMenuItem>
          ))}
        </Select>
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.autoLogClean')}
      >
        <Select
          size="small"
          className="w-[160px]"
          value={values.autoLogClean}
          onChange={(event: SelectChangeEvent) =>
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
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.autoDelayDetection')}
        tooltip={t('settings.modals.misc.tooltips.autoDelayDetection')}
      >
        <Switch
          checked={values.enableAutoDelayDetection}
          onCheckedChange={(checked) =>
            setValues((current) => ({
              ...current,
              enableAutoDelayDetection: checked,
            }))
          }
        />
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.autoDelayDetectionInterval')}
      >
        <TextField
          autoComplete="new-password"
          size="small"
          type="number"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck="false"
          className="w-[160px]"
          value={values.autoDelayDetectionIntervalMinutes}
          disabled={!values.enableAutoDelayDetection}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            setValues((current) => ({
              ...current,
              autoDelayDetectionIntervalMinutes: parsePositiveInt(
                event.target.value,
                1,
              ),
            }))
          }
          slotProps={{
            input: {
              endAdornment: (
                <InputAdornment position="end">
                  {t('shared.units.minutes')}
                </InputAdornment>
              ),
            },
          }}
        />
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.defaultLatencyTest')}
        tooltip={t('settings.modals.misc.tooltips.defaultLatencyTest')}
      >
        <TextField
          autoComplete="new-password"
          size="small"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck="false"
          className="w-[250px]"
          value={values.defaultLatencyTest}
          placeholder="http://cp.cloudflare.com/generate_204"
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            setValues((current) => ({
              ...current,
              defaultLatencyTest: event.target.value,
            }))
          }
        />
      </MiscConfigFormRow>

      <MiscConfigFormRow
        label={t('settings.modals.misc.fields.defaultLatencyTimeout')}
      >
        <TextField
          autoComplete="new-password"
          size="small"
          type="number"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck="false"
          className="w-[250px]"
          value={values.defaultLatencyTimeout}
          placeholder="10000"
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            setValues((current) => ({
              ...current,
              defaultLatencyTimeout: parsePositiveInt(
                event.target.value,
                10000,
              ),
            }))
          }
          slotProps={{
            input: {
              endAdornment: (
                <InputAdornment position="end">
                  {t('shared.units.milliseconds')}
                </InputAdornment>
              ),
            },
          }}
        />
      </MiscConfigFormRow>
    </div>
  )
}
