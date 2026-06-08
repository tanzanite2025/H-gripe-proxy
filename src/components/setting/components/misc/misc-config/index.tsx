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
        auto_check_update: values.autoCheckUpdate,
        default_latency_test: values.defaultLatencyTest,
        default_latency_timeout: values.defaultLatencyTimeout,
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
