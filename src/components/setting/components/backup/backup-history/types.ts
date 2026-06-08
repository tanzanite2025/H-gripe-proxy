import type { Dayjs } from 'dayjs'

export type BackupSource = 'local' | 'webdav'

export type PendingConfirmation = {
  action: 'delete' | 'restore'
  filename: string
  source: BackupSource
} | null

export interface BackupHistoryViewerProps {
  open: boolean
  source: BackupSource
  page: number
  onSourceChange: (source: BackupSource) => void
  onPageChange: (page: number) => void
  onClose: () => void
}

export interface BackupRow {
  filename: string
  platform: string
  backupTime: Dayjs | null
  displayTime: string
  sortValue: number
}
