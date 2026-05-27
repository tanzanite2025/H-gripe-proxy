import { useLockFn } from 'ahooks'
import { RefreshCw, Repeat } from 'lucide-react'
import type { Ref } from 'react'
import { useImperativeHandle, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { closeAllConnections, upgradeCore } from 'tauri-plugin-mihomo-api'

import { BaseDialog, DialogRef } from '@/components/base'
import {
  Box,
  Button,
  Chip,
  List,
  ListItemButton,
  ListItemText,
} from '@/components/tailwind'
import { Spinner } from '@/components/tailwind/icons'
import { useClash, useClashInfo } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import { changeClashCore, restartCore } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

const VALID_CORE = [
  {
    name: 'Mihomo',
    core: 'verge-mihomo',
    chipKey: 'settings.modals.clashCore.variants.release',
  },
]

export function ClashCoreViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()

  const { verge, mutateVerge } = useVerge()
  const { mutateVersion } = useClash()
  const { invalidateClashConfig } = useClashInfo()

  const [open, setOpen] = useState(false)
  const [upgrading, setUpgrading] = useState(false)
  const [restarting, setRestarting] = useState(false)
  const [changingCore, setChangingCore] = useState<string | null>(null)

  useImperativeHandle(ref, () => ({
    open: () => setOpen(true),
    close: () => setOpen(false),
  }))

  const { clash_core = 'verge-mihomo' } = verge ?? {}

  const onCoreChange = useLockFn(async (core: string) => {
    if (core === clash_core) return

    try {
      setChangingCore(core)
      closeAllConnections()
      const errorMsg = await changeClashCore(core)

      if (errorMsg) {
        showNotice.error(errorMsg)
        setChangingCore(null)
        return
      }

      mutateVerge()
      await new Promise((resolve) => setTimeout(resolve, 500))
      invalidateClashConfig()
      mutateVersion()
    } catch (err) {
      showNotice.error(err)
    } finally {
      setChangingCore(null)
    }
  })

  const onRestart = useLockFn(async () => {
    try {
      setRestarting(true)
      await restartCore()
      showNotice.success(
        t('settings.feedback.notifications.clash.restartSuccess'),
      )
      setRestarting(false)
    } catch (err) {
      setRestarting(false)
      showNotice.error(err)
    }
  })

  const onUpgrade = useLockFn(async () => {
    try {
      setUpgrading(true)
      await upgradeCore()
      setUpgrading(false)
      mutateVersion()
      showNotice.success(
        t('settings.feedback.notifications.clash.versionUpdated'),
      )
    } catch (err: any) {
      setUpgrading(false)
      const errMsg = err?.response?.data?.message ?? String(err)
      const showMsg = errMsg.includes('already using latest version')
        ? t('settings.feedback.notifications.clash.alreadyLatestVersion')
        : errMsg
      showNotice.info(showMsg)
    }
  })

  return (
    <BaseDialog
      open={open}
      title={
        <Box className="flex justify-between">
          {t('settings.sections.clash.form.fields.clashCore')}
          <Box>
            <Button
              variant="primary"
              size="small"
              startIcon={<Repeat className="h-4 w-4" />}
              loadingPosition="start"
              loading={upgrading}
              disabled={restarting || changingCore !== null}
              className="mr-8"
              onClick={onUpgrade}
            >
              {t('shared.actions.upgrade')}
            </Button>
            <Button
              variant="primary"
              size="small"
              startIcon={<RefreshCw className="h-4 w-4" />}
              loadingPosition="start"
              loading={restarting}
              disabled={upgrading}
              onClick={onRestart}
            >
              {t('shared.actions.restart')}
            </Button>
          </Box>
        </Box>
      }
      contentSx={{
        pb: 0,
        width: 400,
        height: 180,
        overflowY: 'auto',
        userSelect: 'text',
        marginTop: '-8px',
      }}
      disableOk
      cancelBtn={t('shared.actions.close')}
      onClose={() => setOpen(false)}
      onCancel={() => setOpen(false)}
    >
      <List component="nav">
        {VALID_CORE.map((each) => (
          <ListItemButton
            key={each.core}
            selected={each.core === clash_core}
            onClick={() => onCoreChange(each.core)}
            disabled={changingCore !== null || restarting || upgrading}
          >
            <ListItemText primary={each.name} secondary={`/${each.core}`} />
            {changingCore === each.core ? (
              <Spinner className="h-5 w-5 mr-4" />
            ) : (
              <Chip label={t(each.chipKey)} size="small" />
            )}
          </ListItemButton>
        ))}
      </List>
    </BaseDialog>
  )
}
