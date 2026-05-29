import { type ReactNode } from 'react'

import { cn } from '@/utils/cn'

export interface BadgeProps {
  children: ReactNode
  variant?: 'standard' | 'dot'
  color?: 'default' | 'primary' | 'secondary' | 'error' | 'warning' | 'success' | 'info'
  className?: string
}

export const Badge = ({
  children,
  variant = 'standard',
  color = 'default',
  className,
}: BadgeProps) => {
  const dotColorClass = {
    default: 'bg-gray-400',
    primary: 'bg-primary',
    secondary: 'bg-secondary',
    error: 'bg-red-500',
    warning: 'bg-yellow-500',
    success: 'bg-green-500',
    info: 'bg-blue-500',
  }

  if (variant === 'dot') {
    return (
      <span className={cn('relative inline-flex', className)}>
        {children}
        <span
          className={cn(
            'absolute -right-0.5 -top-0.5 h-2.5 w-2.5 rounded-full ring-2 ring-white dark:ring-card-dark',
            dotColorClass[color],
          )}
        />
      </span>
    )
  }

  return <span className={className}>{children}</span>
}
