import React, { type ButtonHTMLAttributes } from 'react'

export interface FabProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  children?: React.ReactNode
  size?: 'small' | 'medium' | 'large'
  variant?: 'circular' | 'extended'
  color?: 'default' | 'primary' | 'secondary' | 'inherit'
  className?: string
}

export const Fab = React.forwardRef<HTMLButtonElement, FabProps>(
  ({ children, size = 'large', variant = 'circular', color = 'default', className = '', onClick, ...props }, ref) => {
    const sizeClasses = {
      small: variant === 'circular' ? 'w-10 h-10' : 'h-10 px-4',
      medium: variant === 'circular' ? 'w-12 h-12' : 'h-12 px-5',
      large: variant === 'circular' ? 'w-14 h-14' : 'h-14 px-6',
    }

    const colorClasses = {
      default: 'bg-card text-text-primary hover:bg-white/10',
      primary: 'bg-blue-500 text-white hover:bg-blue-600',
      secondary: 'bg-purple-500 text-white hover:bg-purple-600',
      inherit: 'bg-transparent text-inherit hover:bg-black/5 dark:hover:bg-white/5',
    }

    return (
      <button
        ref={ref}
        className={`inline-flex items-center justify-center gap-2 rounded-full font-medium shadow-lg transition-all hover:shadow-xl ${sizeClasses[size]} ${colorClasses[color]} ${className}`}
        onClick={onClick}
        {...props}
      >
        {children}
      </button>
    )
  }
)

Fab.displayName = 'Fab'
