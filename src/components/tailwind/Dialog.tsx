import { Dialog as HeadlessDialog, Transition } from '@headlessui/react'
import { X } from 'lucide-react'
import { Fragment, forwardRef, type ReactNode } from 'react'
import { IconButton } from './IconButton'
import { cn } from '@/utils/cn'

export interface DialogProps {
  open: boolean
  onClose: () => void
  title?: string
  description?: string
  children: ReactNode
  actions?: ReactNode
  maxWidth?: 'sm' | 'md' | 'lg' | 'xl'
  fullWidth?: boolean
  showCloseButton?: boolean
  className?: string
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
}: DialogProps) => {
  const maxWidthClasses = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-xl',
  }

  return (
    <Transition show={open} as={Fragment}>
      <HeadlessDialog onClose={onClose} className="relative z-50">
        {/* 背景遮罩 */}
        <Transition.Child
          as={Fragment}
          enter="ease-out duration-300"
          enterFrom="opacity-0"
          enterTo="opacity-100"
          leave="ease-in duration-200"
          leaveFrom="opacity-100"
          leaveTo="opacity-0"
        >
          <div className="fixed inset-0 bg-black/30 backdrop-blur-sm" aria-hidden="true" />
        </Transition.Child>

        {/* 对话框容器 */}
        <div className="fixed inset-0 flex items-center justify-center p-4">
          <Transition.Child
            as={Fragment}
            enter="ease-out duration-300"
            enterFrom="opacity-0 scale-95"
            enterTo="opacity-100 scale-100"
            leave="ease-in duration-200"
            leaveFrom="opacity-100 scale-100"
            leaveTo="opacity-0 scale-95"
          >
            <HeadlessDialog.Panel
              className={cn(
                fullWidth ? 'w-full' : 'w-auto',
                maxWidthClasses[maxWidth],
                'rounded-dialog bg-card-light dark:bg-card-dark shadow-dialog dark:shadow-dialog-dark',
                className
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
                <HeadlessDialog.Title className="mb-3 pr-8 text-sm font-black uppercase tracking-tight text-gray-900 dark:text-gray-100">
                  {title}
                </HeadlessDialog.Title>
              )}

              {/* 描述 */}
              {description && (
                <HeadlessDialog.Description className="mb-4 text-xs text-gray-600 dark:text-gray-400">
                  {description}
                </HeadlessDialog.Description>
              )}

              {/* 内容 */}
              {children}

              {/* 操作按钮 */}
              {actions && <div className="flex justify-end gap-2">{actions}</div>}
            </HeadlessDialog.Panel>
          </Transition.Child>
        </div>
      </HeadlessDialog>
    </Transition>
  )
}

Dialog.displayName = 'Dialog'

// DialogTitle component
export const DialogTitle = forwardRef<HTMLDivElement, DialogTitleProps>(
  ({ children, className }, ref) => {
    return (
      <div
        ref={ref}
        className={cn('px-6 pt-6 pb-4', className)}
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
        className={cn('px-6 py-4 overflow-y-auto', className)}
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
        className={cn('px-6 pb-6 pt-4 flex justify-end gap-2', className)}
      >
        {children}
      </div>
    )
  }
)

DialogActions.displayName = 'DialogActions'
