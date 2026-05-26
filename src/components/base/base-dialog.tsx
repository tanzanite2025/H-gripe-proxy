import {
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  type SxProps,
  type Theme,
} from '@mui/material'
import { ReactNode } from 'react'

interface Props {
  title: ReactNode
  open: boolean
  okBtn?: ReactNode
  cancelBtn?: ReactNode
  disableEnforceFocus?: boolean
  disableOk?: boolean
  disableCancel?: boolean
  disableFooter?: boolean
  contentSx?: SxProps<Theme>
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
  disableEnforceFocus,
  contentSx,
  disableCancel,
  disableOk,
  disableFooter,
  loading,
  onOk,
  onCancel,
  onClose,
}) => {
  return (
    <Dialog
      open={open}
      onClose={onClose}
      disableEnforceFocus={disableEnforceFocus}
      slotProps={{
        paper: {
          className: 'uds-dialog',
        },
      }}
    >
      <DialogTitle className="uds-title-h2">{title}</DialogTitle>

      <DialogContent className="uds-dialog__content" sx={contentSx}>
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
            <Button loading={loading} variant="contained" onClick={onOk}>
              {okBtn}
            </Button>
          )}
        </DialogActions>
      )}
    </Dialog>
  )
}
