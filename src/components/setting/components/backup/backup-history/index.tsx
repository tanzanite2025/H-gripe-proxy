import { save } from '@tauri-apps/plugin-dialog'
import { useLockFn } from 'ahooks'
import { RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { BaseDialog, BaseLoadingOverlay } from '@/components/base'
import { Box, IconButton, Stack, Tab, Tabs, Typography } from '@/components/tailwind'
import { useVerge } from '@/hooks/system'
import {
  deleteLocalBackup,
  deleteWebdavBackup,
  exportLocalBackup,
  listLocalBackup,
  listWebDavBackup,
  restartApp,
  restoreLocalBackup,
  restoreWebDavBackup,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'
import {
  buildWebdavSignature,
  getWebdavStatus,
  setWebdavStatus,
} from '@/services/webdav-status'

import { BackupConfirmationDialog } from './confirmation-dialog'
import { BACKUP_HISTORY_PAGE_SIZE } from './constants'
import { buildBackupRow, sortBackupRows } from './helpers'
import { BackupHistoryList } from './history-list'
import type { BackupHistoryViewerProps, BackupRow, PendingConfirmation } from './types'

export const BackupHistoryViewer = ({
  open,
  source,
  page,
  onSourceChange,
  onPageChange,
  onClose,
}: BackupHistoryViewerProps) => {
  const { t } = useTranslation()
  const { verge } = useVerge()
  const [rows, setRows] = useState<BackupRow[]>([])
  const [loading, setLoading] = useState(false)
  const [isRestoring, setIsRestoring] = useState(false)
  const [isRestarting, setIsRestarting] = useState(false)
  const [isConfirming, setIsConfirming] = useState(false)
  const [pendingConfirmation, setPendingConfirmation] =
    useState<PendingConfirmation>(null)

  const isLocal = source === 'local'
  const isWebDavConfigured = Boolean(
    verge?.webdav_url && verge?.webdav_username && verge?.webdav_password,
  )
  const webdavSignature = buildWebdavSignature(verge)
  const webdavStatus = getWebdavStatus(webdavSignature)
  const shouldSkipWebDav = !isLocal && !isWebDavConfigured
  const isBusy = loading || isRestoring || isRestarting || isConfirming

  const fetchRows = useCallback(
    async (options?: { force?: boolean }) => {
      if (!open) return
      if (shouldSkipWebDav) {
        setRows([])
        return
      }
      if (!isLocal && webdavStatus === 'failed' && !options?.force) {
        setRows([])
        return
      }

      setLoading(true)
      try {
        const list = isLocal ? await listLocalBackup() : await listWebDavBackup()
        if (!isLocal) {
          setWebdavStatus(webdavSignature, 'ready')
        }

        const nextRows = list
          .map((item) =>
            buildBackupRow(item, {
              unknownPlatform: t(
                'settings.modals.backup.history.unknownPlatform',
                { defaultValue: 'unknown' },
              ),
              unknownTime: t('settings.modals.backup.history.unknownTime', {
                defaultValue: 'Unknown time',
              }),
            }),
          )
          .filter((item): item is BackupRow => item !== null)

        setRows(sortBackupRows(nextRows))
      } catch (error) {
        if (!isLocal) {
          setWebdavStatus(webdavSignature, 'failed')
        }
        console.error(error)
        setRows([])
        showNotice.error(error)
      } finally {
        setLoading(false)
      }
    },
    [isLocal, open, shouldSkipWebDav, t, webdavSignature, webdavStatus],
  )

  useEffect(() => {
    void fetchRows()
  }, [fetchRows])

  const total = rows.length
  const pageCount = Math.max(1, Math.ceil(total / BACKUP_HISTORY_PAGE_SIZE))
  const currentPage = Math.min(page, pageCount - 1)
  const pagedRows = rows.slice(
    currentPage * BACKUP_HISTORY_PAGE_SIZE,
    currentPage * BACKUP_HISTORY_PAGE_SIZE + BACKUP_HISTORY_PAGE_SIZE,
  )

  const summary = useMemo(() => {
    if (shouldSkipWebDav || (!isLocal && webdavStatus === 'failed')) {
      return t('settings.modals.backup.manual.webdav')
    }
    if (!total) {
      return t('settings.modals.backup.history.empty')
    }

    const recent =
      rows[0]?.backupTime?.fromNow() ?? rows[0]?.displayTime ?? ''
    return t('settings.modals.backup.history.summary', {
      count: total,
      recent,
    })
  }, [isLocal, rows, shouldSkipWebDav, t, total, webdavStatus])

  const handleDelete = (filename: string) => {
    if (isRestarting) return
    setPendingConfirmation({ action: 'delete', filename, source })
  }

  const handleRestore = (filename: string) => {
    if (isRestoring || isRestarting) return
    setPendingConfirmation({ action: 'restore', filename, source })
  }

  const handleConfirmAction = useLockFn(async () => {
    if (!pendingConfirmation) return

    const { action, filename, source: actionSource } = pendingConfirmation
    const actionIsLocal = actionSource === 'local'

    setIsConfirming(true)
    if (action === 'restore') {
      setIsRestoring(true)
    }

    try {
      if (action === 'delete') {
        if (actionIsLocal) {
          await deleteLocalBackup(filename)
        } else {
          await deleteWebdavBackup(filename)
        }
        setPendingConfirmation(null)
        await fetchRows()
      } else {
        if (actionIsLocal) {
          await restoreLocalBackup(filename)
        } else {
          await restoreWebDavBackup(filename)
        }
        setPendingConfirmation(null)
        showNotice.success('settings.modals.backup.messages.restoreSuccess')
        setIsRestarting(true)
        window.setTimeout(() => {
          void restartApp().catch((error: unknown) => {
            setIsRestarting(false)
            showNotice.error(error)
          })
        }, 1000)
      }
    } catch (error) {
      console.error(error)
      showNotice.error(error)
    } finally {
      setIsConfirming(false)
      setIsRestoring(false)
    }
  })

  const handleExport = useLockFn(async (filename: string) => {
    if (isRestarting || !isLocal) return

    const savePath = await save({ defaultPath: filename })
    if (!savePath || Array.isArray(savePath)) return

    try {
      await exportLocalBackup(filename, savePath)
      showNotice.success('settings.modals.backup.messages.localBackupExported')
    } catch {
      showNotice.error(
        'settings.modals.backup.messages.localBackupExportFailed',
      )
    }
  })

  const closeConfirmDialog = () => {
    if (isConfirming) return
    setPendingConfirmation(null)
  }

  const confirmTitle =
    pendingConfirmation?.action === 'delete'
      ? t('settings.modals.backup.actions.deleteBackup')
      : t('settings.modals.backup.actions.restoreBackup')
  const confirmMessage =
    pendingConfirmation?.action === 'delete'
      ? t('settings.modals.backup.messages.confirmDelete')
      : t('settings.modals.backup.messages.confirmRestore')

  return (
    <BaseDialog
      open={open}
      title={t('settings.modals.backup.history.title')}
      panelStyle={{ width: 'min(650px, calc(100vw - 56px))' }}
      disableOk
      cancelBtn={t('shared.actions.close')}
      onCancel={onClose}
      onClose={onClose}
    >
      <Box className="relative min-h-[320px]">
        <BaseLoadingOverlay isLoading={isBusy} />
        <Stack spacing={2}>
          <Stack direction="row" className="items-center justify-between">
            <Tabs
              value={source}
              onChange={(_event, value) => {
                if (isBusy) return
                onSourceChange(value as 'local' | 'webdav')
                onPageChange(0)
              }}
              textColor="primary"
              indicatorColor="primary"
            >
              <Tab
                value="local"
                label={t('settings.modals.backup.tabs.local')}
                disabled={isBusy}
                className="px-8"
              />
              <Tab
                value="webdav"
                label={t('settings.modals.backup.tabs.webdav')}
                disabled={isBusy}
                className="px-8"
              />
            </Tabs>
            <IconButton
              size="small"
              onClick={() => {
                if (isRestarting) return
                void fetchRows({ force: true })
              }}
              disabled={isBusy}
            >
              <RefreshCw className="h-4 w-4" />
            </IconButton>
          </Stack>

          <Typography variant="body2" className="text-secondary">
            {summary}
          </Typography>

          <BackupHistoryList
            rows={pagedRows}
            isBusy={isBusy}
            isLocal={isLocal}
            title={t('settings.modals.backup.history.title')}
            emptyLabel={t('settings.modals.backup.history.empty')}
            previousLabel={t('shared.actions.previous')}
            nextLabel={t('shared.actions.next')}
            currentPage={currentPage}
            pageCount={pageCount}
            onExport={(filename) => void handleExport(filename)}
            onDelete={handleDelete}
            onRestore={handleRestore}
            onPrevPage={() => onPageChange(Math.max(0, currentPage - 1))}
            onNextPage={() =>
              onPageChange(Math.min(pageCount - 1, currentPage + 1))
            }
          />
        </Stack>
      </Box>

      <BackupConfirmationDialog
        pendingConfirmation={pendingConfirmation}
        title={confirmTitle}
        message={confirmMessage}
        confirmLabel={t('shared.actions.confirm')}
        cancelLabel={t('shared.actions.cancel')}
        loading={isConfirming}
        onCancel={closeConfirmDialog}
        onConfirm={() => void handleConfirmAction()}
      />
    </BaseDialog>
  )
}
