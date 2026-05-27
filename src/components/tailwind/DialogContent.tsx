import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface DialogContentProps {
  children?: ReactNode
  className?: string
  dividers?: boolean
}

export const DialogContent = forwardRef<HTMLDivElement, DialogContentProps>(
  ({ children, className, dividers }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          'flex-1 px-6 py-4 overflow-auto',
          dividers && 'border-t border-b border-divider',
          className
        )}
      >
        {children}
      </div>
    )
  }
)

DialogContent.displayName = 'DialogContent'
