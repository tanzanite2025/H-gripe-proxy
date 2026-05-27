import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface ListProps {
  children?: ReactNode
  className?: string
  disablePadding?: boolean
  subheader?: ReactNode
  component?: string
}

export interface ListItemProps {
  children?: ReactNode
  className?: string
  button?: boolean
  onClick?: () => void
}

export interface ListItemIconProps {
  children?: ReactNode
  className?: string
}

export interface ListItemTextProps {
  primary?: ReactNode
  secondary?: ReactNode
  className?: string
}

export const List = forwardRef<HTMLUListElement, ListProps>(
  ({ children, className, disablePadding, subheader, component = 'ul' }, ref) => {
    const Component = component as any

    return (
      <Component
        ref={ref}
        className={cn(
          'list-none',
          !disablePadding && 'py-2',
          className
        )}
      >
        {subheader}
        {children}
      </Component>
    )
  }
)

List.displayName = 'List'

export const ListItem = forwardRef<HTMLLIElement, ListItemProps>(
  ({ children, className, button, onClick }, ref) => {
    return (
      <li
        ref={ref}
        className={cn(
          'flex items-start py-2 px-4',
          button && 'cursor-pointer hover:bg-action-hover',
          className
        )}
        onClick={onClick}
      >
        {children}
      </li>
    )
  }
)

ListItem.displayName = 'ListItem'

export const ListItemIcon = forwardRef<HTMLDivElement, ListItemIconProps>(
  ({ children, className }, ref) => {
    return (
      <div
        ref={ref}
        className={cn('mr-4 flex-shrink-0 flex items-center', className)}
      >
        {children}
      </div>
    )
  }
)

ListItemIcon.displayName = 'ListItemIcon'

export const ListItemText = forwardRef<HTMLDivElement, ListItemTextProps>(
  ({ primary, secondary, className }, ref) => {
    return (
      <div ref={ref} className={cn('flex-1 min-w-0', className)}>
        {primary && (
          <div className="text-sm font-medium text-text-primary">
            {primary}
          </div>
        )}
        {secondary && (
          <div className="text-xs text-text-secondary mt-0.5">
            {secondary}
          </div>
        )}
      </div>
    )
  }
)

ListItemText.displayName = 'ListItemText'
