import { useLockFn } from 'ahooks'
import { forwardRef, useImperativeHandle, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog } from '@/components/base'
import { List, Stack } from '@/components/tailwind'
import { Spinner } from '@/components/tailwind/icons'
import { useClashInfo } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import { isPortInUse } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import getSystem from '@/utils/misc'

import {
  buildClashPortConfigs,
  collectChangedPorts,
  createClashPortValues,
  generateRandomPort,
  hasDuplicatePorts,
  hasOnlyValidPorts,
} from './helpers'
import { ClashPortRow } from './port-row'
import type {
  ClashPortNumberKey,
  ClashPortValues,
  ClashPortViewerRef,
} from './types'

const os = getSystem()

export const ClashPortViewer = forwardRef<ClashPortViewerRef>((_, ref) => {
  const { t } = useTranslation()
  const { clashInfo, patchInfo } = useClashInfo()
  const { verge, patchVerge } = useVerge()
  const [open, setOpen] = useState(false)
  const [saving, setSaving] = useState(false)
  const [values, setValues] = useState<ClashPortValues>(() =>
    createClashPortValues(verge, clashInfo),
  )
  const originalValuesRef = useRef<ClashPortValues | null>(null)

  useImperativeHandle(ref, () => ({
    open: () => {
      const nextValues = createClashPortValues(verge, clashInfo)
      originalValuesRef.current = nextValues
      setValues(nextValues)
      setOpen(true)
    },
    close: () => setOpen(false),
  }))

  const updateValue = <K extends keyof ClashPortValues>(
    key: K,
    nextValue: ClashPortValues[K],
  ) => {
    setValues((current) => ({ ...current, [key]: nextValue }))
  }

  const resetToOriginalValues = () => {
    const originalValues = originalValuesRef.current
    if (originalValues) {
      setValues(originalValues)
      return
    }

    setValues(createClashPortValues(verge, clashInfo))
  }

  const onSave = useLockFn(async () => {
    if (hasDuplicatePorts(values)) {
      return
    }
    if (!hasOnlyValidPorts(values)) {
      return
    }

    const changedPorts = collectChangedPorts(values, originalValuesRef.current)

    for (const port of changedPorts) {
      try {
        const inUse = await isPortInUse(port)
        if (inUse) {
          showNotice.error('settings.modals.clashPort.messages.portInUse', {
            port,
          })
          resetToOriginalValues()
          return
        }
      } catch (error) {
        showNotice.error(error)
        return
      }
    }

    const { clashConfig, vergeConfig } = buildClashPortConfigs(values)

    setSaving(true)
    try {
      await Promise.all([patchInfo(clashConfig), patchVerge(vergeConfig)])
      setOpen(false)
      showNotice.success('settings.modals.clashPort.messages.saved')
    } catch (error) {
      showNotice.error('settings.modals.clashPort.messages.saveFailed', error)
    } finally {
      setSaving(false)
    }
  })

  const rows: Array<{
    key: ClashPortNumberKey
    label: string
    enabled: boolean
    enableToggle: boolean
    onToggleEnabled?: (enabled: boolean) => void
  }> = [
    {
      key: 'mixedPort',
      label: t('settings.modals.clashPort.fields.mixed'),
      enabled: true,
      enableToggle: false,
      onToggleEnabled: undefined,
    },
    {
      key: 'socksPort',
      label: t('settings.modals.clashPort.fields.socks'),
      enabled: values.socksEnabled,
      enableToggle: true,
      onToggleEnabled: (enabled: boolean) => updateValue('socksEnabled', enabled),
    },
    {
      key: 'httpPort',
      label: t('settings.modals.clashPort.fields.http'),
      enabled: values.httpEnabled,
      enableToggle: true,
      onToggleEnabled: (enabled: boolean) => updateValue('httpEnabled', enabled),
    },
    ...(os !== 'windows'
      ? [
          {
            key: 'redirPort' as const,
            label: t('settings.modals.clashPort.fields.redir'),
            enabled: values.redirEnabled,
            enableToggle: true,
            onToggleEnabled: (enabled: boolean) =>
              updateValue('redirEnabled', enabled),
          },
        ]
      : []),
    ...(os === 'linux'
      ? [
          {
            key: 'tproxyPort' as const,
            label: t('settings.modals.clashPort.fields.tproxy'),
            enabled: values.tproxyEnabled,
            enableToggle: true,
            onToggleEnabled: (enabled: boolean) =>
              updateValue('tproxyEnabled', enabled),
          },
        ]
      : []),
  ]

  return (
    <BaseDialog
      open={open}
      title={t('settings.modals.clashPort.title')}
      panelStyle={{ width: 400 }}
      okBtn={
        saving ? (
          <Stack direction="row" spacing={1} className="items-center">
            <Spinner className="h-5 w-5" />
            {t('shared.statuses.saving')}
          </Stack>
        ) : (
          t('shared.actions.save')
        )
      }
      cancelBtn={t('shared.actions.cancel')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
      onOk={() => void onSave()}
    >
      <List className="w-full">
        {rows.map((row) => (
          <ClashPortRow
            key={row.key}
            label={row.label}
            port={values[row.key]}
            enabled={row.enabled}
            enableToggle={row.enableToggle}
            randomTitle={t('settings.modals.clashPort.actions.random')}
            onPortChange={(port) => updateValue(row.key, port)}
            onRandomPort={() => updateValue(row.key, generateRandomPort())}
            onToggleEnabled={row.onToggleEnabled}
          />
        ))}
      </List>
    </BaseDialog>
  )
})
