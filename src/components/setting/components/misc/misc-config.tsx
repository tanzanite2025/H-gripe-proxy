import { useLockFn } from 'ahooks'
import { forwardRef, useImperativeHandle, useState, type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, DialogRef, Switch, TooltipIcon } from '@/components/base'
import {
  InputAdornment,
  SelectMenuItem,
  Select,
  TextField,
} from '@/components/tailwind'
import type { SelectChangeEvent } from '@/components/tailwind/Select'
import { useVerge } from '@/hooks/system'
import { showNotice } from '@/services/notice-service'

export const MiscViewer = forwardRef<DialogRef>((props, ref) => {
  const { t } = useTranslation()
  const { verge, patchVerge } = useVerge()

  const [open, setOpen] = useState(false)
  const [values, setValues] = useState({
    appLogLevel: 'warn',
    appLogMaxSize: 8,
    appLogMaxCount: 12,
    autoCloseConnection: true,
    autoCheckUpdate: true,
    enableBuiltinEnhanced: true,
    proxyLayoutColumn: 6,
    enableAutoDelayDetection: false,
    autoDelayDetectionIntervalMinutes: 5,
    defaultLatencyTest: '',
    autoLogClean: 2,
    defaultLatencyTimeout: 10000,
  })

  useImperativeHandle(ref, () => ({
    open: () => {
      setOpen(true)
      setValues({
        appLogLevel: verge?.app_log_level ?? 'warn',
        appLogMaxSize: verge?.app_log_max_size ?? 128,
        appLogMaxCount: verge?.app_log_max_count ?? 8,
        autoCloseConnection: verge?.auto_close_connection ?? true,
        autoCheckUpdate: verge?.auto_check_update ?? true,
        enableBuiltinEnhanced: verge?.enable_builtin_enhanced ?? true,
        proxyLayoutColumn: verge?.proxy_layout_column || 6,
        enableAutoDelayDetection: verge?.enable_auto_delay_detection ?? false,
        autoDelayDetectionIntervalMinutes:
          verge?.auto_delay_detection_interval_minutes ?? 5,
        defaultLatencyTest: verge?.default_latency_test || '',
        autoLogClean: verge?.auto_log_clean || 0,
        defaultLatencyTimeout: verge?.default_latency_timeout || 10000,
      })
    },
    close: () => setOpen(false),
  }))

  const onSave = useLockFn(async () => {
    try {
      await patchVerge({
        app_log_level: values.appLogLevel,
        app_log_max_size: values.appLogMaxSize,
        app_log_max_count: values.appLogMaxCount,
        auto_close_connection: values.autoCloseConnection,
        auto_check_update: values.autoCheckUpdate,
        enable_builtin_enhanced: values.enableBuiltinEnhanced,
        proxy_layout_column: values.proxyLayoutColumn,
        enable_auto_delay_detection: values.enableAutoDelayDetection,
        auto_delay_detection_interval_minutes:
          values.autoDelayDetectionIntervalMinutes,
        default_latency_test: values.defaultLatencyTest,
        default_latency_timeout: values.defaultLatencyTimeout,
        auto_log_clean: values.autoLogClean as any,
      })
      setOpen(false)
    } catch (err) {
      showNotice.error(err)
    }
  })

  return (
    <BaseDialog
      open={open}
      title={t('settings.modals.misc.title')}
      panelStyle={{ width: 600, maxWidth: 600 }}
      okBtn={t('shared.actions.save')}
      cancelBtn={t('shared.actions.cancel')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
      onOk={onSave}
    >
      <div className="flex flex-col gap-3">
        {/* App Log Level */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.appLogLevel')}</span>
          <Select
            size="small"
            className="ml-auto w-[120px] shrink-0"
            value={values.appLogLevel}
            onChange={(e: SelectChangeEvent) =>
              setValues((v) => ({
                ...v,
                appLogLevel: e.target.value as string,
              }))
            }
          >
            {['trace', 'debug', 'info', 'warn', 'error', 'silent'].map((i) => (
              <SelectMenuItem value={i} key={i}>
                {i[0].toUpperCase() + i.slice(1).toLowerCase()}
              </SelectMenuItem>
            ))}
          </Select>
        </div>

        {/* App Log Max Size */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.appLogMaxSize')}</span>
          <TextField
            autoComplete="new-password"
            size="small"
            type="number"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck="false"
            className="ml-auto w-[140px] shrink-0"
            value={values.appLogMaxSize}
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setValues((v) => ({
                ...v,
                appLogMaxSize: Math.max(1, parseInt(e.target.value) || 128),
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
        </div>

        {/* App Log Max Count */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.appLogMaxCount')}</span>
          <TextField
            autoComplete="new-password"
            size="small"
            type="number"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck="false"
            className="ml-auto w-[140px] shrink-0"
            value={values.appLogMaxCount}
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setValues((v) => ({
                ...v,
                appLogMaxCount: Math.max(1, parseInt(e.target.value) || 1),
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
        </div>

        {/* Auto Close Connections */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.autoCloseConnections')}</span>
          <TooltipIcon
            title={t('settings.modals.misc.tooltips.autoCloseConnections')}
            className="opacity-70"
          />
          <Switch
            checked={values.autoCloseConnection}
            onCheckedChange={(checked) =>
              setValues((v) => ({ ...v, autoCloseConnection: checked }))
            }
            className="ml-auto"
          />
        </div>

        {/* Auto Check Update */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.autoCheckUpdate')}</span>
          <Switch
            checked={values.autoCheckUpdate}
            onCheckedChange={(checked) =>
              setValues((v) => ({ ...v, autoCheckUpdate: checked }))
            }
            className="ml-auto"
          />
        </div>

        {/* Enable Builtin Enhanced */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.enableBuiltinEnhanced')}</span>
          <TooltipIcon
            title={t('settings.modals.misc.tooltips.enableBuiltinEnhanced')}
            className="opacity-70"
          />
          <Switch
            checked={values.enableBuiltinEnhanced}
            onCheckedChange={(checked) =>
              setValues((v) => ({ ...v, enableBuiltinEnhanced: checked }))
            }
            className="ml-auto"
          />
        </div>

        {/* Proxy Layout Columns */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.proxyLayoutColumns')}</span>
          <Select
            size="small"
            className="ml-auto w-[160px] shrink-0"
            value={values.proxyLayoutColumn}
            onChange={(e: SelectChangeEvent) =>
              setValues((v) => ({
                ...v,
                proxyLayoutColumn: Number(e.target.value),
              }))
            }
          >
            <SelectMenuItem value={6} key={6}>
              {t('settings.modals.misc.options.proxyLayoutColumns.auto')}
            </SelectMenuItem>
            {[1, 2, 3, 4, 5].map((i) => (
              <SelectMenuItem value={i} key={i}>
                {i}
              </SelectMenuItem>
            ))}
          </Select>
        </div>

        {/* Auto Log Clean */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.autoLogClean')}</span>
          <Select
            size="small"
            className="ml-auto w-[160px] shrink-0"
            value={values.autoLogClean}
            onChange={(e: SelectChangeEvent) =>
              setValues((v) => ({
                ...v,
                autoLogClean: Number(e.target.value),
              }))
            }
          >
            {/* 1: 1天, 2: 7天, 3: 30天, 4: 90天*/}
            {[
              {
                key: t('settings.modals.misc.options.autoLogClean.never'),
                value: 0,
              },
              {
                key: t('settings.modals.misc.options.autoLogClean.retainDays', {
                  n: 1,
                }),
                value: 1,
              },
              {
                key: t('settings.modals.misc.options.autoLogClean.retainDays', {
                  n: 7,
                }),
                value: 2,
              },
              {
                key: t('settings.modals.misc.options.autoLogClean.retainDays', {
                  n: 30,
                }),
                value: 3,
              },
              {
                key: t('settings.modals.misc.options.autoLogClean.retainDays', {
                  n: 90,
                }),
                value: 4,
              },
            ].map((i) => (
              <SelectMenuItem key={i.value} value={i.value}>
                {i.key}
              </SelectMenuItem>
            ))}
          </Select>
        </div>

        {/* Auto Delay Detection */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.autoDelayDetection')}</span>
          <TooltipIcon
            title={t('settings.modals.misc.tooltips.autoDelayDetection')}
            className="opacity-70"
          />
          <Switch
            checked={values.enableAutoDelayDetection}
            onCheckedChange={(checked) =>
              setValues((v) => ({ ...v, enableAutoDelayDetection: checked }))
            }
            className="ml-auto"
          />
        </div>

        {/* Auto Delay Detection Interval */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.autoDelayDetectionInterval')}</span>
          <TextField
            autoComplete="new-password"
            size="small"
            type="number"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck="false"
            className="ml-auto w-[160px] shrink-0"
            value={values.autoDelayDetectionIntervalMinutes}
            disabled={!values.enableAutoDelayDetection}
            onChange={(e: ChangeEvent<HTMLInputElement>) => {
              const parsed = parseInt(e.target.value, 10)
              const intervalMinutes =
                Number.isFinite(parsed) && parsed > 0 ? parsed : 1
              setValues((v) => ({
                ...v,
                autoDelayDetectionIntervalMinutes: intervalMinutes,
              }))
            }}
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
        </div>

        {/* Default Latency Test */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.defaultLatencyTest')}</span>
          <TooltipIcon
            title={t('settings.modals.misc.tooltips.defaultLatencyTest')}
            className="opacity-70"
          />
          <TextField
            autoComplete="new-password"
            size="small"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck="false"
            className="ml-auto w-[250px] shrink-0"
            value={values.defaultLatencyTest}
            placeholder="http://cp.cloudflare.com/generate_204"
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setValues((v) => ({ ...v, defaultLatencyTest: e.target.value }))
            }
          />
        </div>

        {/* Default Latency Timeout */}
        <div className="flex items-center gap-3">
          <span className="text-sm font-medium text-text-primary shrink-0">{t('settings.modals.misc.fields.defaultLatencyTimeout')}</span>
          <TextField
            autoComplete="new-password"
            size="small"
            type="number"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck="false"
            className="ml-auto w-[250px] shrink-0"
            value={values.defaultLatencyTimeout}
            placeholder="10000"
            onChange={(e: ChangeEvent<HTMLInputElement>) =>
              setValues((v) => ({
                ...v,
                defaultLatencyTimeout: parseInt(e.target.value),
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
        </div>
      </div>
    </BaseDialog>
  )
})
