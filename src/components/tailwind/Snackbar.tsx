import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface SnackbarProps {
  children?: ReactNode
  className?: string
  open?: boolean
  autoHideDuration?: number
  onClose?: () => void
  anchorOrigin?: {
    vertical: 'top' | 'bottom'
    horizontal: 'left' | 'center' | 'right'
  }
  message?: ReactNode
  style?: React.CSSProperties
}

export const Snackbar = forwardRef<HTMLDivElement, SnackbarProps>(
  ({ children, className, open, anchorOrigin = { vertical: 'bottom', horizontal: 'left' }, message, style }, ref) => {
    if (!open) return null

    const positionClasses = {
      'top-left': 'top-4 left-4',
      'top-center': 'top-4 left-1/2 -translate-x-1/2',
      'top-right': 'top-4 right-4',
      'bottom-left': 'bottom-4 left-4',
      'bottom-center': 'bottom-4 left-1/2 -translate-x-1/2',
      'bottom-right': 'bottom-4 right-4',
    }

    const positionKey = `${anchorOrigin.vertical}-${anchorOrigin.horizontal}` as keyof typeof positionClasses

    return (
      <div
        ref={ref}
        style={style}
        className={cn(
          'fixed z-50',
          positionClasses[positionKey],
          className
        )}
      >
        {message || children}
      </div>
    )
  }
)

Snackbar.displayName = 'Snackbar'
