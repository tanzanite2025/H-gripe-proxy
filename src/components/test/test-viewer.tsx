import { useLockFn } from 'ahooks'
import { nanoid } from 'nanoid'
import { forwardRef, useImperativeHandle, useState } from 'react'
import { useForm, Controller } from 'react-hook-form'
import { useTranslation } from 'react-i18next'

import { BaseDialog } from '@/components/base'
import { TextField } from '@/components/tailwind'
import { useVerge } from '@/hooks/system'
import { showNotice } from '@/services/notice-service'

interface Props {
  onChange: (uid: string, patch?: Partial<IVergeTestItem>) => void
}

export interface TestViewerRef {
  create: () => void
  edit: (item: IVergeTestItem) => void
}

// create or edit the test item
export const TestViewer = forwardRef<TestViewerRef, Props>(
  ({ onChange }, ref) => {
    const { t } = useTranslation()
    const [open, setOpen] = useState(false)
    const [openType, setOpenType] = useState<'new' | 'edit'>('new')
    const [loading, setLoading] = useState(false)
    const { verge, patchVerge } = useVerge()
    const testList = verge?.test_list ?? []
    const { control, ...formIns } = useForm<IVergeTestItem>({
      defaultValues: {
        name: '',
        icon: '',
        url: '',
      },
    })

    const patchTestList = async (
      uid: string,
      patch: Partial<IVergeTestItem>,
    ) => {
      const newList = testList.map((x) => {
        if (x.uid === uid) {
          return { ...x, ...patch }
        }
        return x
      })
      await patchVerge({ test_list: newList })
    }

    useImperativeHandle(ref, () => ({
      create: () => {
        setOpenType('new')
        setOpen(true)
      },
      edit: (item) => {
        if (item) {
          Object.entries(item).forEach(([key, value]) => {
            formIns.setValue(key as any, value)
          })
        }
        setOpenType('edit')
        setOpen(true)
      },
    }))

    const handleOk = useLockFn(
      formIns.handleSubmit(async (form) => {
        setLoading(true)
        try {
          if (!form.name) throw new Error('`Name` should not be null')
          if (!form.url) throw new Error('`Url` should not be null')

          let newList
          let uid

          if (form.icon && form.icon.startsWith('<svg')) {
            // 移除 icon 中的注释
            if (form.icon) {
              form.icon = form.icon.replace(/<!--[\s\S]*?-->/g, '')
            }
            const doc = new DOMParser().parseFromString(
              form.icon,
              'image/svg+xml',
            )
            if (doc.querySelector('parsererror')) {
              throw new Error('`Icon`svg format error')
            }
          }

          if (openType === 'new') {
            uid = nanoid()
            const item = { ...form, uid }
            newList = [...testList, item]
            await patchVerge({ test_list: newList })
            onChange(uid)
          } else {
            if (!form.uid) throw new Error('UID not found')
            uid = form.uid

            await patchTestList(uid, form)
            onChange(uid, form)
          }
          setOpen(false)
          setLoading(false)
          setTimeout(() => formIns.reset(), 500)
        } catch (err: any) {
          showNotice.error(err)
          setLoading(false)
        }
      }),
    )

    const handleClose = () => {
      setOpen(false)
      setTimeout(() => formIns.reset(), 500)
    }

    const text = {
      fullWidth: true,
      autoComplete: 'off',
      autoCorrect: 'off',
    } as const

    return (
      <BaseDialog
        open={open}
        title={
          openType === 'new'
            ? t('tests.modals.test.title.create')
            : t('tests.modals.test.title.edit')
        }
        panelStyle={{ minWidth: 420, minHeight: 380 }}
        okBtn={t('shared.actions.save')}
        cancelBtn={t('shared.actions.cancel')}
        onClose={handleClose}
        onCancel={handleClose}
        onOk={handleOk}
        loading={loading}
      >
        <div className="space-y-4">
          <Controller
            name="name"
            control={control}
            render={({ field }) => (
              <TextField {...text} {...field} label={t('shared.labels.name')} />
            )}
          />
          <Controller
            name="icon"
            control={control}
            render={({ field }) => (
              <TextField
                {...text}
                {...field}
                multiline
                rows={5}
                label={t('shared.labels.icon')}
              />
            )}
          />
          <Controller
            name="url"
            control={control}
            render={({ field }) => (
              <TextField
                {...text}
                {...field}
                multiline
                rows={3}
                label={t('tests.modals.test.fields.url')}
              />
            )}
          />
        </div>
      </BaseDialog>
    )
  },
)
