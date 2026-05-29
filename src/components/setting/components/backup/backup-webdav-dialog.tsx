import { useCallback, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseLoadingOverlay } from '@/components/base'
import { Box, Dialog, Button } from '@/components/tailwind'
import { useVerge } from '@/hooks/system'
import { listWebDavBackup } from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { buildWebdavSignature, setWebdavStatus } from '@/services/webdav-status'

import { BackupConfigViewer } from './backup-config'

interface BackupWebdavDialogProps {
  open: boolean
  onClose: () => void
  onBackupSuccess?: () => void
  setBusy?: (loading: boolean) => void
}

export const BackupWebdavDialog = ({
  open,
  onClose,
  onBackupSuccess,
  setBusy,
}: BackupWebdavDialogProps) => {
  const { t } = useTranslation()
  const { verge } = useVerge()
  const [loading, setLoading] = useState(false)
  const webdavSignature = buildWebdavSignature(verge)

  const handleLoading = useCallback(
    (value: boolean) => {
      setLoading(value)
      setBusy?.(value)
    },
    [setBusy],
  )

  const refreshWebdav = useCallback(
    async (options?: { silent?: boolean; signature?: string }) => {
      const signature = options?.signature ?? webdavSignature
      handleLoading(true)
      try {
        await listWebDavBackup()
        setWebdavStatus(signature, 'ready')
        if (!options?.silent) {
          showNotice.success(
            'settings.modals.backup.messages.webdavRefreshSuccess',
          )
        }
      } catch (error) {
        setWebdavStatus(signature, 'failed')
        showNotice.error(
          'settings.modals.backup.messages.webdavRefreshFailed',
          { error },
        )
      } finally {
        handleLoading(false)
      }
    },
    [handleLoading, webdavSignature],
  )

  const refreshSilently = useCallback(
    (signature?: string) => refreshWebdav({ silent: true, signature }),
    [refreshWebdav],
  )

  return (
    <Dialog
      open={open}
      onClose={onClose}
      title={t('settings.modals.backup.webdav.title')}
      maxWidth="md"
      actions={
        <Button onClick={onClose}>
          {t('shared.actions.close')}
        </Button>
      }
    >
      <Box className="w-[360px] sm:w-[520px] relative">
        <BaseLoadingOverlay isLoading={loading} />
        <BackupConfigViewer
          setLoading={handleLoading}
          onBackupSuccess={async () => {
            await refreshSilently()
            onBackupSuccess?.()
          }}
          onSaveSuccess={refreshSilently}
          onRefresh={refreshWebdav}
          onInit={refreshSilently}
        />
      </Box>
    </Dialog>
  )
}
