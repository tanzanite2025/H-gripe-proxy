import { useLockFn } from 'ahooks'
import type { Ref } from 'react'
import { useImperativeHandle, useState, type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, DialogRef, Switch, TooltipIcon } from '@/components/base'
import {
  InputAdornment,
  List,
  ListItem,
  ListItemText,
  TextField,
} from '@/components/tailwind'
import { useVerge } from '@/hooks/system'
import { entry_lightweight_mode } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

export function LiteModeViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()
  const { verge, patchVerge } = useVerge()

  const [open, setOpen] = useState(false)
  const [values, setValues] = useState({
    autoEnterLiteMode: false,
    autoEnterLiteModeDelay: 10, // 默认10分钟
  })

  useImperativeHandle(ref, () => ({
    open: () => {
      setOpen(true)
      setValues({
        autoEnterLiteMode: verge?.enable_auto_light_weight_mode ?? false,
        autoEnterLiteModeDelay: verge?.auto_light_weight_minutes ?? 10,
      })
    },
    close: () => setOpen(false),
  }))

  const onSave = useLockFn(async () => {
    try {
      await patchVerge({
        enable_auto_light_weight_mode: values.autoEnterLiteMode,
        auto_light_weight_minutes: values.autoEnterLiteModeDelay,
      })
      setOpen(false)
    } catch (err) {
      showNotice.error(err)
    }
  })

  return (
    <BaseDialog
      open={open}
      title={t('settings.modals.liteMode.title')}
      panelStyle={{ width: 450 }}
      okBtn={t('shared.actions.save')}
      cancelBtn={t('shared.actions.cancel')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
      onOk={onSave}
    >
      <List>
        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t('settings.modals.liteMode.actions.enterNow')}
          />
          <button
            type="button"
            className="text-sm font-medium text-primary hover:underline"
            onClick={async () => await entry_lightweight_mode()}
          >
            {t('shared.actions.enable')}
          </button>
        </ListItem>

        <ListItem className="py-[5px] px-[2px]">
          <ListItemText
            primary={t('settings.modals.liteMode.toggles.autoEnter')}
            className="max-w-fit"
          />
          <TooltipIcon
            title={t('settings.modals.liteMode.tooltips.autoEnter')}
            className="opacity-70"
          />
          <Switch
            checked={values.autoEnterLiteMode}
            onCheckedChange={(checked) =>
              setValues((v) => ({ ...v, autoEnterLiteMode: checked }))
            }
            className="ml-auto"
          />
        </ListItem>

        {values.autoEnterLiteMode && (
          <>
            <ListItem className="py-[5px] px-[2px]">
              <ListItemText
                primary={t('settings.modals.liteMode.fields.delay')}
              />
              <TextField
                autoComplete="off"
                size="small"
                type="number"
                autoCorrect="off"
                autoCapitalize="off"
                spellCheck="false"
                className="w-[150px]"
                value={values.autoEnterLiteModeDelay}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  setValues((v) => ({
                    ...v,
                    autoEnterLiteModeDelay: parseInt(e.target.value) || 1,
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
            </ListItem>

            <ListItem className="py-[5px] px-[2px]">
              <div className="text-sm italic text-gray-600 dark:text-gray-400">
                {t('settings.modals.liteMode.messages.autoEnterHint', {
                  n: values.autoEnterLiteModeDelay,
                })}
              </div>
            </ListItem>
          </>
        )}
      </List>
    </BaseDialog>
  )
}
