import { forwardRef, type HTMLAttributes, type ReactNode } from 'react'

import { cn } from '@/utils/cn'

export interface ListItemButtonProps
  extends Omit<HTMLAttributes<HTMLDivElement>, 'children' | 'className'> {
  children?: ReactNode
  className?: string
  selected?: boolean
  disabled?: boolean
  dense?: boolean
}

export const ListItemButton = forwardRef<HTMLDivElement, ListItemButtonProps>(
  ({ children, className, selected, disabled, onClick, dense = false, ...props }, ref) => {
    return (
      <div
        ref={ref}
        {...props}
        role="button"
        tabIndex={disabled ? -1 : 0}
        onClick={disabled ? undefined : onClick}
        className={cn(
          dense ? 'flex items-center px-3 py-1.5 cursor-pointer transition-colors' : 'flex items-center px-4 py-2 cursor-pointer transition-colors',
          'hover:bg-action-hover active:bg-action-selected',
          selected && 'bg-action-selected',
          disabled && 'opacity-50 cursor-not-allowed pointer-events-none',
          className
        )}
      >
        {children}
      </div>
    )
  }
)

ListItemButton.displayName = 'ListItemButton'
