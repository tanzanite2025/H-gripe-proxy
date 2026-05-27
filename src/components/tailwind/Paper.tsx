import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface PaperProps {
  children?: ReactNode
  className?: string
  variant?: 'elevation' | 'outlined'
  elevation?: number
}

export const Paper = forwardRef<HTMLDivElement, PaperProps>(
  ({ children, className, variant = 'elevation', elevation = 1 }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          'rounded-lg bg-white dark:bg-[#24252f]',
          variant === 'outlined' && 'border border-divider',
          variant === 'elevation' && elevation > 0 && 'shadow-md',
          className
        )}
      >
        {children}
      </div>
    )
  }
)

Paper.displayName = 'Paper'
