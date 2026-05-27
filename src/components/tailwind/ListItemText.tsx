import { forwardRef, type ReactNode } from 'react'
import { cn } from '@/utils/cn'

export interface ListItemTextProps {
  primary?: ReactNode
  secondary?: ReactNode
  className?: string
  slotProps?: {
    primary?: { className?: string }
    secondary?: { component?: string; className?: string }
  }
}

export const ListItemText = forwardRef<HTMLDivElement, ListItemTextProps>(
  ({ primary, secondary, className, slotProps }, ref) => {
    return (
      <div ref={ref} className={cn('flex-1 min-w-0', className)}>
        {primary && (
          <div className={cn('text-sm font-medium', slotProps?.primary?.className)}>
            {primary}
          </div>
        )}
        {secondary && (
          <div className={cn('text-xs text-secondary mt-1', slotProps?.secondary?.className)}>
            {secondary}
          </div>
        )}
      </div>
    )
  }
)

ListItemText.displayName = 'ListItemText'
