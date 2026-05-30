import { forwardRef, type ReactNode } from 'react'

import { cn } from '@/utils/cn'

export interface ListProps {
  children?: ReactNode
  className?: string
  disablePadding?: boolean
  dense?: boolean
  subheader?: ReactNode
  component?: string
}

export interface ListItemProps {
  children?: ReactNode
  className?: string
  button?: boolean
  onClick?: () => void
  secondaryAction?: ReactNode
  style?: React.CSSProperties
  ref?: React.Ref<HTMLLIElement>
}

export interface ListItemIconProps {
  children?: ReactNode
  className?: string
}

export interface ListItemTextProps {
  primary?: ReactNode
  secondary?: ReactNode
  className?: string
  onClick?: () => void
  secondaryClassName?: string
  ref?: React.Ref<HTMLDivElement>
}

export const List = forwardRef<HTMLUListElement, ListProps>(
  ({ children, className, disablePadding, dense, subheader, component = 'ul' }, ref) => {
    const Component = component as any

    return (
      <Component
        ref={ref}
        className={cn(
          'list-none',
          !disablePadding && (dense ? 'py-1' : 'py-2'),
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
  ({ children, className, button, onClick, secondaryAction, style }, ref) => {
    return (
      <li
        ref={ref}
        style={style}
        className={cn(
          'flex items-center py-2 px-4 relative',
          button && 'cursor-pointer hover:bg-action-hover',
          className
        )}
        onClick={onClick}
      >
        {children}
        {secondaryAction && <div className="ml-auto flex-shrink-0">{secondaryAction}</div>}
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
  ({ primary, secondary, className, onClick, secondaryClassName }, ref) => {
    return (
      <div ref={ref} className={cn('flex-1 min-w-0', className)} onClick={onClick}>
        {primary && (
          <div className="text-sm font-medium text-text-primary">
            {primary}
          </div>
        )}
        {secondary && (
          <div className={cn('text-xs text-text-secondary mt-0.5', secondaryClassName)}>
            {secondary}
          </div>
        )}
      </div>
    )
  }
)

ListItemText.displayName = 'ListItemText'
