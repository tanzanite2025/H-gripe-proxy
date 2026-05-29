import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react'

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  size?: 'small' | 'medium' | 'large'
  color?: 'default' | 'primary' | 'secondary' | 'inherit' | 'warning' | 'error'
  edge?: 'start' | 'end' | false
  children?: ReactNode
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(
  ({ size = 'medium', color = 'inherit', edge = false, className = '', children, ...props }, ref) => {
    const baseClasses =
      'inline-flex items-center justify-center rounded-full transition-all duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed'

    const sizeClasses = {
      small: 'h-8 w-8 text-sm',
      medium: 'h-10 w-10 text-base',
      large: 'h-12 w-12 text-lg',
    }

    const colorClasses = {
      default: 'text-gray-600 dark:text-gray-400 hover:bg-gray-100 dark:hover:bg-gray-800 focus:ring-gray-300',
      primary: 'text-primary dark:text-primary-dark-mode hover:bg-primary/10 focus:ring-primary',
      secondary: 'text-secondary hover:bg-secondary/10 focus:ring-secondary',
      inherit: 'text-inherit hover:bg-gray-100 dark:hover:bg-gray-800 focus:ring-primary',
      warning: 'text-yellow-600 dark:text-yellow-400 hover:bg-yellow-100 dark:hover:bg-yellow-900/20 focus:ring-yellow-500',
      error: 'text-red-600 dark:text-red-400 hover:bg-red-100 dark:hover:bg-red-900/20 focus:ring-red-500',
    }

    const edgeClasses = edge === 'start' ? '-ml-2' : edge === 'end' ? '-mr-2' : ''

    return (
      <button
        ref={ref}
        className={`${baseClasses} ${sizeClasses[size]} ${colorClasses[color]} ${edgeClasses} ${className}`}
        {...props}
      >
        {children}
      </button>
    )
  },
)

IconButton.displayName = 'IconButton'
