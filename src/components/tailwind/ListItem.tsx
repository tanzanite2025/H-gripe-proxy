import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface ListItemProps {
  children?: ReactNode
  className?: string
  disableGutters?: boolean
  divider?: boolean
  onClick?: () => void
}

export const ListItem = forwardRef<HTMLLIElement, ListItemProps>(
  ({ children, className, disableGutters, divider, onClick }, ref) => {
    return (
      <li
        ref={ref}
        onClick={onClick}
        className={cn(
          'flex items-center',
          !disableGutters && 'px-4 py-2',
          divider && 'border-b border-divider',
          onClick && 'cursor-pointer hover:bg-action-hover',
          className
        )}
      >
        {children}
      </li>
    )
  }
)

ListItem.displayName = 'ListItem'
