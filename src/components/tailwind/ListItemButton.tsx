import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface ListItemButtonProps {
  children?: ReactNode
  className?: string
  selected?: boolean
  disabled?: boolean
  onClick?: () => void
}

export const ListItemButton = forwardRef<HTMLDivElement, ListItemButtonProps>(
  ({ children, className, selected, disabled, onClick }, ref) => {
    return (
      <div
        ref={ref}
        role="button"
        tabIndex={disabled ? -1 : 0}
        onClick={disabled ? undefined : onClick}
        className={cn(
          'flex items-center px-4 py-2 cursor-pointer transition-colors',
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
