import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface InputAdornmentProps {
  children?: ReactNode
  className?: string
  position?: 'start' | 'end'
}

export const InputAdornment = forwardRef<HTMLDivElement, InputAdornmentProps>(
  ({ children, className, position = 'end' }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          'flex items-center text-secondary',
          position === 'start' ? 'mr-2' : 'ml-2',
          className
        )}
      >
        {children}
      </div>
    )
  }
)

InputAdornment.displayName = 'InputAdornment'
