import { useLockFn } from 'ahooks'
import type { Ref } from 'react'
import { useEffect, useImperativeHandle, useRef, useState } from 'react'
import { useForm } from 'react-hook-form'

import { useProfiles } from '@/hooks/data'
import { createProfile, patchProfile } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import { ProfileViewerUI } from './profile-viewer-ui'

interface Props {
  onChange: (isActivating?: boolean) => void
}

export interface ProfileViewerRef {
  create: () => void
  edit: (item: IProfileItem) => void
}

// create or edit the profile
// remote / local
type ProfileViewerProps = Props & { ref?: Ref<ProfileViewerRef> }

export function ProfileViewer({ onChange, ref }: ProfileViewerProps) {
  const [open, setOpen] = useState(false)
  const [openType, setOpenType] = useState<'new' | 'edit'>('new')
  const [loading, setLoading] = useState(false)
  const { profiles } = useProfiles()

  // file input
  const fileDataRef = useRef<string | null>(null)

  const { control, watch, setValue, reset, handleSubmit, getValues } =
    useForm<IProfileItem>({
      defaultValues: {
        type: 'remote',
        name: '',
        desc: '',
        url: '',
        option: {
          with_proxy: false,
          self_proxy: false,
        },
      },
    })

  useImperativeHandle(ref, () => ({
    create: () => {
      setOpenType('new')
      setOpen(true)
    },
    edit: (item: IProfileItem) => {
      if (item) {
        Object.entries(item).forEach(([key, value]) => {
          setValue(key as any, value)
        })
      }
      setOpenType('edit')
      setOpen(true)
    },
  }))

  const selfProxy = watch('option.self_proxy')
  const withProxy = watch('option.with_proxy')

  useEffect(() => {
    if (selfProxy) setValue('option.with_proxy', false)
  }, [selfProxy, setValue])

  useEffect(() => {
    if (withProxy) setValue('option.self_proxy', false)
  }, [setValue, withProxy])

  const handleOk = useLockFn(
    handleSubmit(async (form) => {
      setLoading(true)
      try {
        // 基本验证
        if (!form.type) throw new Error('`Type` should not be null')
        if (form.type === 'remote' && !form.url) {
          throw new Error('The URL should not be null')
        }

        // 处理表单数据
        const option = form.option ? { ...form.option } : undefined
        if (option?.timeout_seconds) {
          option.timeout_seconds = +option.timeout_seconds
        }
        if (option?.update_interval) {
          option.update_interval = +option.update_interval
        } else if (option) {
          option.update_interval = undefined
        }
        if (option?.user_agent === '') {
          option.user_agent = undefined
        }

        const name = form.name || `${form.type} file`
        const item = { ...form, name, option }
        const isRemote = form.type === 'remote'
        const isUpdate = openType === 'edit'

        // 判断是否是当前激活的配置
        const isActivating = isUpdate && form.uid === (profiles?.current ?? '')

        // 保存原始代理设置以便回退成功后恢复
        const originalOptions = {
          with_proxy: form.option?.with_proxy,
          self_proxy: form.option?.self_proxy,
        }

        // 执行创建或更新操作，本地配置不需要回退机制
        if (!isRemote) {
          if (openType === 'new') {
            await createProfile(item, fileDataRef.current)
          } else {
            if (!form.uid) throw new Error('UID not found')
            await patchProfile(form.uid, item)
          }
        } else {
          // 远程配置使用回退机制
          try {
            // 尝试正常操作
            if (openType === 'new') {
              await createProfile(item, fileDataRef.current)
            } else {
              if (!form.uid) throw new Error('UID not found')
              await patchProfile(form.uid, item)
            }
          } catch {
            // 首次创建/更新失败，尝试使用自身代理
            showNotice.info(
              'profiles.modals.profileForm.feedback.notifications.creationRetry',
            )

            // 使用自身代理的配置
            const retryItem = {
              ...item,
              option: {
                ...item.option,
                with_proxy: false,
                self_proxy: true,
              },
            }

            // 使用自身代理再次尝试
            if (openType === 'new') {
              await createProfile(retryItem, fileDataRef.current)
            } else {
              if (!form.uid) throw new Error('UID not found')
              await patchProfile(form.uid, retryItem)

              // 编辑模式下恢复原始代理设置
              await patchProfile(form.uid, { option: originalOptions })
            }

            showNotice.success(
              'profiles.modals.profileForm.feedback.notifications.creationSuccess',
            )
          }
        }

        // 成功后的操作
        setOpen(false)
        setTimeout(() => reset(), 500)
        fileDataRef.current = null

        // 优化：UI先关闭，异步通知父组件
        setTimeout(() => {
          onChange(isActivating)
        }, 0)
      } catch (err) {
        showNotice.error(err)
      } finally {
        setLoading(false)
      }
    }),
  )

  const handleClose = () => {
    try {
      setOpen(false)
      fileDataRef.current = null
      setTimeout(() => reset(), 500)
    } catch (e) {
      console.warn('[ProfileViewer] handleClose error:', e)
    }
  }

  const formType = watch('type')

  return (
    <ProfileViewerUI
      open={open}
      openType={openType}
      loading={loading}
      control={control}
      formType={formType}
      onClose={handleClose}
      onCancel={handleClose}
      onOk={handleOk}
      setValue={setValue}
      getValues={getValues}
      onFileDataChange={(value) => {
        fileDataRef.current = value
      }}
    />
  )
}
