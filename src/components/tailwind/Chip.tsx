import React from 'react'

export interface ChipProps extends React.HTMLAttributes<HTMLDivElement> {
  label: React.ReactNode
  color?: 'default' | 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success'
  size?: 'small' | 'medium'
  variant?: 'filled' | 'outlined'
  icon?: React.ReactNode
  className?: string
}

export const Chip = React.forwardRef<HTMLDivElement, ChipProps>(
  ({ label, color = 'default', size = 'medium', variant = 'filled', icon, className = '', ...props }, ref) => {
    const colorClasses = {
      default: variant === 'filled' 
        ? 'bg-gray-200 dark:bg-gray-700 text-gray-900 dark:text-gray-100'
        : 'border border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100',
      primary: variant === 'filled'
        ? 'bg-blue-500 text-white'
        : 'border border-blue-500 text-blue-500',
      secondary: variant === 'filled'
        ? 'bg-purple-500 text-white'
        : 'border border-purple-500 text-purple-500',
      error: variant === 'filled'
        ? 'bg-red-500 text-white'
        : 'border border-red-500 text-red-500',
      warning: variant === 'filled'
        ? 'bg-yellow-500 text-white'
        : 'border border-yellow-500 text-yellow-500',
      info: variant === 'filled'
        ? 'bg-cyan-500 text-white'
        : 'border border-cyan-500 text-cyan-500',
      success: variant === 'filled'
        ? 'bg-green-500 text-white'
        : 'border border-green-500 text-green-500',
    }

    const sizeClasses = {
      small: 'text-xs px-2 py-0.5 h-6',
      medium: 'text-sm px-3 py-1 h-8',
    }

    return (
      <div
        ref={ref}
        className={`inline-flex items-center gap-1 rounded-full font-medium ${colorClasses[color]} ${sizeClasses[size]} ${className}`}
        {...props}
      >
        {icon && <span className="flex items-center">{icon}</span>}
        <span>{label}</span>
      </div>
    )
  }
)

Chip.displayName = 'Chip'
