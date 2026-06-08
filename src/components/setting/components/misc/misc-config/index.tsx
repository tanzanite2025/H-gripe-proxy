import { useLockFn } from 'ahooks'
import { forwardRef, useImperativeHandle, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, DialogRef } from '@/components/base'
import { useVerge } from '@/hooks/system'
import { showNotice } from '@/services/notice-service'

import { MiscConfigForm } from './form'
import { createMiscConfigValues } from './types'

export const MiscViewer = forwardRef<DialogRef>((props, ref) => {
  const { t } = useTranslation()
  const { verge, patchVerge } = useVerge()
  const [open, setOpen] = useState(false)
  const [values, setValues] = useState(() => createMiscConfigValues(verge))

  useImperativeHandle(ref, () => ({
    open: () => {
      setOpen(true)
      setValues(createMiscConfigValues(verge))
    },
    close: () => setOpen(false),
  }))

  const onSave = useLockFn(async () => {
    try {
      await patchVerge({
        app_log_level: values.appLogLevel,
        app_log_max_size: values.appLogMaxSize,
        app_log_max_count: values.appLogMaxCount,
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
    } catch (error) {
      showNotice.error(error)
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
      <MiscConfigForm values={values} setValues={setValues} />
    </BaseDialog>
  )
})
