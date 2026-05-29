import { forwardRef, type ReactNode } from 'react'

import { cn } from '@/utils/cn'

export interface DialogActionsProps {
  children?: ReactNode
  className?: string
}

export const DialogActions = forwardRef<HTMLDivElement, DialogActionsProps>(
  ({ children, className }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          'flex items-center justify-end gap-2 px-6 py-4 border-t border-divider',
          className
        )}
      >
        {children}
      </div>
    )
  }
)

DialogActions.displayName = 'DialogActions'
