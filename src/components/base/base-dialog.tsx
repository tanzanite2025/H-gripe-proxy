import { type CSSProperties, type ReactNode } from 'react'

import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind'

interface Props {
  title: ReactNode
  open: boolean
  okBtn?: ReactNode
  cancelBtn?: ReactNode
  disableEnforceFocus?: boolean
  disableOk?: boolean
  disableCancel?: boolean
  disableFooter?: boolean
  className?: string
  panelClassName?: string
  panelStyle?: CSSProperties
  contentClassName?: string
  children?: ReactNode
  loading?: boolean
  onOk?: () => void
  onCancel?: () => void
  onClose?: () => void
}

export interface DialogRef {
  open: () => void
  close: () => void
}

export const BaseDialog: React.FC<Props> = ({
  open,
  title,
  children,
  okBtn,
  cancelBtn,
  disableEnforceFocus: _disableEnforceFocus,
  className,
  panelClassName,
  panelStyle,
  contentClassName,
  disableCancel,
  disableOk,
  disableFooter,
  loading,
  onOk,
  onCancel,
  onClose,
}) => {
  const handleClose = onClose ?? (() => {})

  return (
    <Dialog
      open={open}
      onClose={handleClose}
      className={`uds-dialog ${className || ''}`}
      slotProps={{
        paper: {
          className: panelClassName,
          style: panelStyle,
        },
      }}
    >
      <DialogTitle className="uds-title-h2">{title}</DialogTitle>

      <DialogContent className={`uds-dialog__content ${className || ''} ${contentClassName || ''}`}>
        {children}
      </DialogContent>

      {!disableFooter && (
        <DialogActions className="uds-dialog__actions">
          {!disableCancel && (
            <Button variant="outlined" onClick={onCancel}>
              {cancelBtn}
            </Button>
          )}
          {!disableOk && (
            <Button loading={loading} variant="primary" onClick={onOk}>
              {okBtn}
            </Button>
          )}
        </DialogActions>
      )}
    </Dialog>
  )
}
