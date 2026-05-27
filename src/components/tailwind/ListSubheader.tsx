import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface ListSubheaderProps {
  children?: ReactNode
  className?: string
  disableSticky?: boolean
}

export const ListSubheader = forwardRef<HTMLLIElement, ListSubheaderProps>(
  ({ children, className, disableSticky }, ref) => {
    return (
      <li
        ref={ref}
        className={cn(
          'px-4 py-2 text-xs font-semibold text-secondary uppercase tracking-wider',
          !disableSticky && 'sticky top-0 bg-background z-10',
          className
        )}
      >
        {children}
      </li>
    )
  }
)

ListSubheader.displayName = 'ListSubheader'
