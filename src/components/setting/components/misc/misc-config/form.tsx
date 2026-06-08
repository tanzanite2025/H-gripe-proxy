import type { ChangeEvent, Dispatch, SetStateAction } from 'react'
import { useTranslation } from 'react-i18next'

import { Switch } from '@/components/base'
import {
  InputAdornment,
  TextField,
} from '@/components/tailwind'

import { MiscConfigFormRow } from './form-row'
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
