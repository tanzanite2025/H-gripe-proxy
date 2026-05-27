import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react'
import { Loader2 } from 'lucide-react'

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'outlined' | 'text'
  size?: 'small' | 'medium' | 'large'
  loading?: boolean
  startIcon?: ReactNode
  endIcon?: ReactNode
  loadingPosition?: 'start' | 'end' | 'center'
  children: ReactNode
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = 'primary',
      size = 'medium',
      loading = false,
      startIcon,
      endIcon,
      loadingPosition = 'start',
      disabled,
      className = '',
      children,
      ...props
    },
    ref,
  ) => {
    const baseClasses =
      'inline-flex items-center justify-center gap-2 rounded-button font-black uppercase tracking-widest transition-all duration-300 ease-smooth focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed'

    const variantClasses = {
      primary:
        'bg-primary dark:bg-primary-dark-mode text-white hover:opacity-90 hover:-translate-y-0.5 shadow-button focus:ring-primary dark:focus:ring-primary-dark-mode',
      outlined:
        'border border-dashed border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 hover:bg-gray-50 dark:hover:bg-gray-800 hover:-translate-y-0.5 focus:ring-gray-300 dark:focus:ring-gray-600',
      text: 'text-primary dark:text-primary-dark-mode hover:bg-gray-100 dark:hover:bg-gray-800 focus:ring-primary dark:focus:ring-primary-dark-mode',
    }

    const sizeClasses = {
      small: 'px-4 py-2 text-[10px]',
      medium: 'px-6 py-3 text-xs',
      large: 'px-8 py-4 text-sm',
    }

    const isDisabled = disabled || loading

    const renderLoadingIcon = () => (
      <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
    )

    return (
      <button
        ref={ref}
        disabled={isDisabled}
        className={`${baseClasses} ${variantClasses[variant]} ${sizeClasses[size]} ${className}`}
        aria-busy={loading}
        {...props}
      >
        {loading && loadingPosition === 'start' && renderLoadingIcon()}
        {!loading && startIcon && <span className="inline-flex">{startIcon}</span>}
        {children}
        {loading && loadingPosition === 'end' && renderLoadingIcon()}
        {!loading && endIcon && <span className="inline-flex">{endIcon}</span>}
      </button>
    )
  },
)

Button.displayName = 'Button'
