import { open as openDialog } from '@tauri-apps/plugin-dialog'
import { useLockFn } from 'ahooks'
import type { ReactNode, Ref } from 'react'
import { useCallback, useImperativeHandle, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, DialogRef } from '@/components/base'
import {
  Button,
  List,
  ListItem,
  ListItemText,
  Stack,
  Typography,
} from '@/components/tailwind'
import { useVerge } from '@/hooks/system'
import {
  createLocalBackup,
  createWebdavBackup,
  importLocalBackup,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import { buildWebdavSignature, setWebdavStatus } from '@/services/webdav-status'

import { AutoBackupSettings } from './auto-backup-settings'
import { BackupHistoryViewer } from './backup-history'
import { BackupWebdavDialog } from './backup-webdav-dialog'

type BackupSource = 'local' | 'webdav'

export function BackupViewer({ ref }: { ref?: Ref<DialogRef> }) {
  const { t } = useTranslation()
  const { verge } = useVerge()
  const [open, setOpen] = useState(false)
  const [busyAction, setBusyAction] = useState<BackupSource | null>(null)
  const [localImporting, setLocalImporting] = useState(false)
  const [historyOpen, setHistoryOpen] = useState(false)
  const [historySource, setHistorySource] = useState<BackupSource>('local')
  const [historyPage, setHistoryPage] = useState(0)
  const [webdavDialogOpen, setWebdavDialogOpen] = useState(false)
  const webdavSignature = buildWebdavSignature(verge)

  useImperativeHandle(ref, () => ({
    open: () => setOpen(true),
    close: () => setOpen(false),
  }))

  const openHistory = (target: BackupSource) => {
    setHistorySource(target)
    setHistoryPage(0)
    setHistoryOpen(true)
  }

  const handleBackup = useLockFn(async (target: BackupSource) => {
    try {
      setBusyAction(target)
      if (target === 'local') {
        await createLocalBackup()
        showNotice.success('settings.modals.backup.messages.localBackupCreated')
      } else {
        await createWebdavBackup()
        showNotice.success('settings.modals.backup.messages.backupCreated')
        setWebdavStatus(webdavSignature, 'ready')
      }
    } catch (error) {
      console.error(error)
      showNotice.error(
        target === 'local'
          ? 'settings.modals.backup.messages.localBackupFailed'
          : 'settings.modals.backup.messages.backupFailed',
        target === 'local' ? undefined : { error },
      )
      if (target === 'webdav') {
        setWebdavStatus(webdavSignature, 'failed')
      }
    } finally {
      setBusyAction(null)
    }
  })

  const handleImport = useLockFn(async () => {
    const selected = await openDialog({
      multiple: false,
      filters: [{ name: 'Backup File', extensions: ['zip'] }],
    })
    if (!selected || Array.isArray(selected)) return
    try {
      setLocalImporting(true)
      await importLocalBackup(selected)
      showNotice.success('settings.modals.backup.messages.localBackupImported')
      openHistory('local')
    } catch (error) {
      console.error(error)
      showNotice.error(
        'settings.modals.backup.messages.localBackupImportFailed',
        { error },
      )
    } finally {
      setLocalImporting(false)
    }
  })

  const setWebdavBusy = useCallback(
    (loading: boolean) => {
      setBusyAction(loading ? 'webdav' : null)
    },
    [setBusyAction],
  )

  const isLocalBusy = busyAction === 'local' || localImporting

  return (
    <BaseDialog
      open={open}
      title={t('settings.modals.backup.title')}
      panelStyle={{ width: 'min(650px, calc(100vw - 56px))' }}
      disableOk
      cancelBtn={t('shared.actions.close')}
      onCancel={() => setOpen(false)}
      onClose={() => setOpen(false)}
    >
      <Stack spacing={2}>
        <Stack
          className="uds-card-container border border-divider rounded-lg p-8"
          spacing={1}
        >
          <Typography variant="subtitle1" className="uds-card-title">
            {t('settings.modals.backup.auto.title')}
          </Typography>
          <List disablePadding className="[&>li]:px-0">
            <AutoBackupSettings />
          </List>
        </Stack>

        <Stack
          className="uds-card-container border border-divider rounded-lg p-8"
          spacing={1}
        >
          <Typography variant="subtitle1" className="uds-card-title">
            {t('settings.modals.backup.manual.title')}
          </Typography>
          <List disablePadding className="[&>li]:px-0">
            {(
              [
                {
                  key: 'local' as BackupSource,
                  title: t('settings.modals.backup.tabs.local'),
                  description: t('settings.modals.backup.manual.local'),
                  actions: [
                    <Button
                      key="backup"
                      variant="primary"
                      size="small"
                      loading={busyAction === 'local'}
                      disabled={localImporting}
                      onClick={() => handleBackup('local')}
                    >
                      {t('settings.modals.backup.actions.backup')}
                    </Button>,
                    <Button
                      key="history"
                      variant="outlined"
                      size="small"
                      disabled={isLocalBusy}
                      onClick={() => openHistory('local')}
                    >
                      {t('settings.modals.backup.actions.viewHistory')}
                    </Button>,
                    <Button
                      key="import"
                      variant="text"
                      size="small"
                      loading={localImporting}
                      disabled={busyAction === 'local'}
                      onClick={() => handleImport()}
                    >
                      {t('settings.modals.backup.actions.importBackup')}
                    </Button>,
                  ],
                },
                {
                  key: 'webdav' as BackupSource,
                  title: t('settings.modals.backup.tabs.webdav'),
                  description: t('settings.modals.backup.manual.webdav'),
                  actions: [
                    <Button
                      key="backup"
                      variant="primary"
                      size="small"
                      loading={busyAction === 'webdav'}
                      onClick={() => handleBackup('webdav')}
                    >
                      {t('settings.modals.backup.actions.backup')}
                    </Button>,
                    <Button
                      key="history"
                      variant="outlined"
                      size="small"
                      onClick={() => openHistory('webdav')}
                    >
                      {t('settings.modals.backup.actions.viewHistory')}
                    </Button>,
                    <Button
                      key="configure"
                      variant="text"
                      size="small"
                      onClick={() => setWebdavDialogOpen(true)}
                    >
                      {t('settings.modals.backup.manual.configureWebdav')}
                    </Button>,
                  ],
                },
              ] satisfies Array<{
                key: BackupSource
                title: string
                description: string
                actions: ReactNode[]
              }>
            ).map((item, idx) => (
              <ListItem key={item.key} disableGutters divider={idx === 0}>
                <Stack spacing={1} className="w-full">
                  <ListItemText
                    primary={<span className="uds-card-title">{item.title}</span>}
                    slotProps={{ secondary: { component: 'span' } }}
                    secondary={<span className="uds-desc">{item.description}</span>}
                  />
                  <Stack
                    direction="row"
                    spacing={1}
                    useFlexGap
                    className="flex-wrap items-center"
                  >
                    {item.actions}
                  </Stack>
                </Stack>
              </ListItem>
            ))}
          </List>
        </Stack>
      </Stack>

      <BackupHistoryViewer
        open={historyOpen}
        source={historySource}
        page={historyPage}
        onSourceChange={setHistorySource}
        onPageChange={setHistoryPage}
        onClose={() => setHistoryOpen(false)}
      />
      <BackupWebdavDialog
        open={webdavDialogOpen}
        onClose={() => setWebdavDialogOpen(false)}
        onBackupSuccess={() => openHistory('webdav')}
        setBusy={setWebdavBusy}
      />
    </BaseDialog>
  )
}
