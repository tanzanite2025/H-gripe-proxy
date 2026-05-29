import { forwardRef, type ReactNode } from 'react'

import { cn } from '@/utils/cn'

export interface FormGroupProps {
  children?: ReactNode
  className?: string
  row?: boolean
}

export const FormGroup = forwardRef<HTMLDivElement, FormGroupProps>(
  ({ children, className, row }, ref) => {
    return (
      <div
        ref={ref}
        className={cn(
          'flex gap-4',
          row ? 'flex-row' : 'flex-col',
          className
        )}
      >
        {children}
      </div>
    )
  }
)

FormGroup.displayName = 'FormGroup'
