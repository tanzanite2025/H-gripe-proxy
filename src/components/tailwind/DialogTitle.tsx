import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface DialogTitleProps {
  children?: ReactNode
  className?: string
}

export const DialogTitle = forwardRef<HTMLHeadingElement, DialogTitleProps>(
  ({ children, className }, ref) => {
    return (
      <h2
        ref={ref}
        className={cn(
          'text-lg font-semibold px-6 py-4',
          className
        )}
      >
        {children}
      </h2>
    )
  }
)

DialogTitle.displayName = 'DialogTitle'
