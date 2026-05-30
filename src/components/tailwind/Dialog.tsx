import { Dialog as HeadlessDialog, DialogPanel, DialogBackdrop, DialogTitle as HeadlessDialogTitle } from '@headlessui/react'
import { X } from 'lucide-react'
import { forwardRef, type CSSProperties, type ReactNode } from 'react'

import { cn } from '@/utils/cn'

import { IconButton } from './IconButton'

export interface DialogProps {
  open: boolean
  onClose: () => void
  title?: string
  description?: string
  children: ReactNode
  actions?: ReactNode
  maxWidth?: 'xs' | 'sm' | 'md' | 'lg' | 'xl'
  fullWidth?: boolean
  showCloseButton?: boolean
  className?: string
  disableEnforceFocus?: boolean
  slotProps?: {
    paper?: {
      className?: string
      style?: CSSProperties
    }
  }
}

export interface DialogTitleProps {
  children: ReactNode
  className?: string
}

export interface DialogContentProps {
  children: ReactNode
  className?: string
}

export interface DialogActionsProps {
  children: ReactNode
  className?: string
}

export const Dialog = ({
  open,
  onClose,
  title,
  description,
  children,
  actions,
  maxWidth = 'md',
  fullWidth = false,
  showCloseButton = false,
  className,
  disableEnforceFocus: _disableEnforceFocus,
  slotProps,
}: DialogProps) => {
  const maxWidthClasses = {
    xs: 'max-w-[444px]',
    sm: 'max-w-[600px]',
    md: 'max-w-[900px]',
    lg: 'max-w-[1200px]',
    xl: 'max-w-[1536px]',
  }

  const paperClassName = slotProps?.paper?.className
  const paperStyle = slotProps?.paper?.style

  return (
    <HeadlessDialog open={open} onClose={onClose} className="relative z-50">
      {/* 背景遮罩 */}
      <DialogBackdrop
        transition
        className="fixed inset-0 bg-black/30 backdrop-blur-sm transition-opacity duration-300 data-[closed]:opacity-0 data-[enter]:opacity-100 data-[leave]:opacity-0"
      />

      {/* 对话框容器 */}
      <div className="fixed inset-0 flex items-center justify-center p-4">
        <DialogPanel
          transition
          style={paperStyle}
          className={cn(
            fullWidth ? 'w-full' : 'w-auto',
            maxWidthClasses[maxWidth],
            'flex flex-col max-h-[85vh] rounded-dialog bg-card shadow-dialog p-6 transition duration-300 data-[closed]:opacity-0 data-[closed]:scale-95 data-[enter]:opacity-100 data-[enter]:scale-100 data-[leave]:opacity-0 data-[leave]:scale-95',
            className,
            paperClassName,
          )}
        >
          {/* 关闭按钮 */}
          {showCloseButton && (
            <div className="absolute right-4 top-4">
              <IconButton onClick={onClose} aria-label="Close dialog">
                <X className="h-5 w-5" />
              </IconButton>
            </div>
          )}

          {/* 标题 */}
          {title && (
            <HeadlessDialogTitle className="mb-3 pr-8 text-sm font-semibold uppercase tracking-tight text-text-primary">
              {title}
            </HeadlessDialogTitle>
          )}

          {/* 内容 */}
          {children}

          {/* 操作按钮 */}
          {actions && <div className="flex justify-end gap-2">{actions}</div>}
        </DialogPanel>
      </div>
    </HeadlessDialog>
  )
}

Dialog.displayName = 'Dialog'

// DialogTitle component
export const DialogTitle = forwardRef<HTMLDivElement, DialogTitleProps>(
  ({ children, className }, ref) => {
    return (
      <div
        ref={ref}
        className={cn('pb-4', className)}
      >
        {children}
      </div>
    )
  }
)

DialogTitle.displayName = 'DialogTitle'

// DialogContent component
export const DialogContent = forwardRef<HTMLDivElement, DialogContentProps>(
  ({ children, className }, ref) => {
    return (
      <div
        ref={ref}
        className={cn('py-2 overflow-y-auto', className)}
      >
        {children}
      </div>
    )
  }
)

DialogContent.displayName = 'DialogContent'

// DialogActions component
export const DialogActions = forwardRef<HTMLDivElement, DialogActionsProps>(
  ({ children, className }, ref) => {
    return (
      <div
        ref={ref}
        className={cn('pt-4 flex justify-end gap-2', className)}
      >
        {children}
      </div>
    )
  }
)

DialogActions.displayName = 'DialogActions'
