import { BaseDialog } from '@/components/base'
import { Typography } from '@/components/tailwind'

import type { PendingConfirmation } from './types'

interface BackupConfirmationDialogProps {
  pendingConfirmation: PendingConfirmation
  title: string
  message: string
  confirmLabel: string
  cancelLabel: string
  loading: boolean
  onCancel: () => void
  onConfirm: () => void
}

export function BackupConfirmationDialog({
  pendingConfirmation,
  title,
  message,
  confirmLabel,
  cancelLabel,
  loading,
  onCancel,
  onConfirm,
}: BackupConfirmationDialogProps) {
  return (
    <BaseDialog
      open={pendingConfirmation !== null}
      title={title}
      okBtn={confirmLabel}
      cancelBtn={cancelLabel}
      panelStyle={{ width: 'min(420px, calc(100vw - 56px))' }}
      loading={loading}
      onCancel={onCancel}
      onClose={onCancel}
      onOk={onConfirm}
    >
      <Typography variant="body2" className="break-words">
        {message}
      </Typography>
      {pendingConfirmation?.filename && (
        <Typography variant="caption" className="mt-4 block break-all text-secondary">
          {pendingConfirmation.filename}
        </Typography>
      )}
    </BaseDialog>
  )
}
