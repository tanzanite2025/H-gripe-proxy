import { forwardRef, type ReactNode } from 'react'

import { cn } from '@/utils/cn'

export interface CollapseProps {
  children?: ReactNode
  className?: string
  in?: boolean
  open?: boolean
  timeout?: number | 'auto'
}

export const Collapse = forwardRef<HTMLDivElement, CollapseProps>(
  ({ children, className, in: isOpen, open }, ref) => {
    // Support both 'in' and 'open' props
    const isExpanded = isOpen ?? open ?? false
    
    return (
      <div
        ref={ref}
        className={cn(
          'transition-all duration-300 overflow-hidden',
          isExpanded ? 'max-h-screen opacity-100' : 'max-h-0 opacity-0',
          className
        )}
      >
        {children}
      </div>
    )
  }
)

Collapse.displayName = 'Collapse'
