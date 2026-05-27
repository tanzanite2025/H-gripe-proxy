import type { Control, UseFormGetValues, UseFormSetValue } from 'react-hook-form'
import { Controller } from 'react-hook-form'
import { useTranslation } from 'react-i18next'

import { BaseDialog, Switch } from '@/components/base'
import { InputAdornment } from '@/components/tailwind/InputAdornment'
import { Select } from '@/components/tailwind/Select'
import { TextField } from '@/components/tailwind/TextField'
import { version } from '@root/package.json'

import { FileInput } from './file-input'

interface ProfileViewerUIProps {
  open: boolean
  openType: 'new' | 'edit'
  loading: boolean
  control: Control<IProfileItem>
  formType: string
  onClose: () => void
  onCancel: () => void
  onOk: () => void
  setValue: UseFormSetValue<IProfileItem>
  getValues: UseFormGetValues<IProfileItem>
  fileDataRef: React.MutableRefObject<string | null>
}

export function ProfileViewerUI({
  open,
  openType,
  loading,
  control,
  formType,
  onClose,
  onCancel,
  onOk,
  setValue,
  getValues,
  fileDataRef,
}: ProfileViewerUIProps) {
  const { t } = useTranslation()

  const text = {
    fullWidth: true,
    size: 'small',
    margin: 'normal',
    variant: 'outlined',
    autoComplete: 'off',
    autoCorrect: 'off',
  } as const

  const isRemote = formType === 'remote'
  const isLocal = formType === 'local'

  return (
    <BaseDialog
      open={open}
      title={
        openType === 'new'
          ? t('profiles.modals.profileForm.title.create')
          : t('profiles.modals.profileForm.title.edit')
      }
      contentSx={{ width: 375, pb: 0, maxHeight: '80%' }}
      okBtn={t('shared.actions.save')}
      cancelBtn={t('shared.actions.cancel')}
      onClose={onClose}
      onCancel={onCancel}
      onOk={onOk}
      loading={loading}
    >
      <Controller
        name="type"
        control={control}
        render={({ field }) => (
          <Select
            {...field}
            label={t('profiles.modals.profileForm.fields.type')}
            className="mt-2 mb-2"
            fullWidth
            autoFocus
          >
            <option value="remote">Remote</option>
            <option value="local">Local</option>
          </Select>
        )}
      />

      <Controller
        name="name"
        control={control}
        render={({ field }) => (
          <TextField {...text} {...field} label={t('shared.labels.name')} />
        )}
      />

      <Controller
        name="desc"
        control={control}
        render={({ field }) => (
          <TextField
            {...text}
            {...field}
            label={t('profiles.modals.profileForm.fields.description')}
          />
        )}
      />

      {isRemote && (
        <>
          <Controller
            name="url"
            control={control}
            render={({ field }) => (
              <TextField
                {...text}
                {...field}
                multiline
                label={t('profiles.modals.profileForm.fields.subscriptionUrl')}
              />
            )}
          />

          <Controller
            name="option.user_agent"
            control={control}
            render={({ field }) => (
              <TextField
                {...text}
                {...field}
                placeholder={`clash-verge/v${version}`}
                label="User Agent"
              />
            )}
          />

          <Controller
            name="option.timeout_seconds"
            control={control}
            render={({ field }) => (
              <TextField
                {...text}
                {...field}
                type="number"
                placeholder="60"
                label={t('profiles.modals.profileForm.fields.httpTimeout')}
                slotProps={{
                  input: {
                    endAdornment: (
                      <InputAdornment position="end">
                        {t('shared.units.seconds')}
                      </InputAdornment>
                    ),
                  },
                }}
              />
            )}
          />
        </>
      )}

      {(isRemote || isLocal) && (
        <Controller
          name="option.update_interval"
          control={control}
          render={({ field }) => (
            <TextField
              {...text}
              {...field}
              type="number"
              label={t('profiles.modals.profileForm.fields.updateInterval')}
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
          )}
        />
      )}

      {isLocal && openType === 'new' && (
        <FileInput
          onChange={(file, val) => {
            setValue('name', getValues('name') || file.name)
            fileDataRef.current = val
          }}
        />
      )}

      {isRemote && (
        <>
          <Controller
            name="option.with_proxy"
            control={control}
            render={({ field }) => (
              <div className="mx-0 my-2 ml-2 flex items-center justify-between">
                <label className="uds-label">
                  {t('profiles.modals.profileForm.fields.useSystemProxy')}
                </label>
                <Switch checked={field.value} {...field} color="primary" />
              </div>
            )}
          />

          <Controller
            name="option.self_proxy"
            control={control}
            render={({ field }) => (
              <div className="mx-0 my-2 ml-2 flex items-center justify-between">
                <label className="uds-label">
                  {t('profiles.modals.profileForm.fields.useClashProxy')}
                </label>
                <Switch checked={field.value} {...field} color="primary" />
              </div>
            )}
          />

          <Controller
            name="option.danger_accept_invalid_certs"
            control={control}
            render={({ field }) => (
              <div className="mx-0 my-2 ml-2 flex items-center justify-between">
                <label className="uds-label">
                  {t('profiles.modals.profileForm.fields.acceptInvalidCerts')}
                </label>
                <Switch checked={field.value} {...field} color="primary" />
              </div>
            )}
          />

          <Controller
            name="option.allow_auto_update"
            control={control}
            render={({ field }) => (
              <div className="mx-0 my-2 ml-2 flex items-center justify-between">
                <label className="uds-label">
                  {t('profiles.modals.profileForm.fields.allowAutoUpdate')}
                </label>
                <Switch checked={field.value} {...field} color="primary" />
              </div>
            )}
          />
        </>
      )}
    </BaseDialog>
  )
}
