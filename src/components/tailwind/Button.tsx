import { Loader2 } from 'lucide-react'
import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from 'react'

export interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'outlined' | 'text' | 'default' | 'destructive' | 'ghost' | 'outline' | 'contained' | 'danger'
  size?: 'small' | 'medium' | 'large' | 'sm'
  color?: 'inherit' | 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success'
  loading?: boolean
  startIcon?: ReactNode
  endIcon?: ReactNode
  loadingPosition?: 'start' | 'end' | 'center'
  fullWidth?: boolean
  children: ReactNode
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(
  (
    {
      variant = 'primary',
      size = 'medium',
      color,
      loading = false,
      startIcon,
      endIcon,
      loadingPosition = 'start',
      fullWidth = false,
      disabled,
      className = '',
      children,
      ...props
    },
    ref,
  ) => {
    const baseClasses =
      'inline-flex items-center justify-center gap-2 rounded-button font-black uppercase tracking-widest transition-all duration-300 ease-smooth focus:outline-none focus:ring-2 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed'

    const containedColorClasses = {
      primary:
        'bg-primary dark:bg-primary-dark-mode text-white hover:opacity-90 hover:-translate-y-0.5 shadow-button focus:ring-primary dark:focus:ring-primary-dark-mode',
      secondary:
        'bg-purple-600 dark:bg-purple-700 text-white hover:bg-purple-700 dark:hover:bg-purple-800 focus:ring-purple-500',
      error:
        'bg-red-600 dark:bg-red-700 text-white hover:bg-red-700 dark:hover:bg-red-800 focus:ring-red-500',
      warning:
        'bg-yellow-500 dark:bg-yellow-600 text-white hover:bg-yellow-600 dark:hover:bg-yellow-700 focus:ring-yellow-500',
      info:
        'bg-cyan-600 dark:bg-cyan-700 text-white hover:bg-cyan-700 dark:hover:bg-cyan-800 focus:ring-cyan-500',
      success:
        'bg-green-600 dark:bg-green-700 text-white hover:bg-green-700 dark:hover:bg-green-800 focus:ring-green-500',
      inherit:
        'bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-100 hover:bg-gray-200 dark:hover:bg-gray-700 focus:ring-gray-300 dark:focus:ring-gray-600',
    }

    const outlinedColorClasses = {
      primary:
        'border border-dashed border-primary text-primary dark:text-primary-dark-mode hover:bg-primary/10 hover:-translate-y-0.5 focus:ring-primary dark:focus:ring-primary-dark-mode',
      secondary:
        'border border-dashed border-purple-500 text-purple-600 dark:text-purple-400 hover:bg-purple-500/10 hover:-translate-y-0.5 focus:ring-purple-500',
      error:
        'border border-dashed border-red-500 text-red-600 dark:text-red-400 hover:bg-red-500/10 hover:-translate-y-0.5 focus:ring-red-500',
      warning:
        'border border-dashed border-yellow-500 text-yellow-600 dark:text-yellow-400 hover:bg-yellow-500/10 hover:-translate-y-0.5 focus:ring-yellow-500',
      info:
        'border border-dashed border-cyan-500 text-cyan-600 dark:text-cyan-400 hover:bg-cyan-500/10 hover:-translate-y-0.5 focus:ring-cyan-500',
      success:
        'border border-dashed border-green-500 text-green-600 dark:text-green-400 hover:bg-green-500/10 hover:-translate-y-0.5 focus:ring-green-500',
      inherit:
        'border border-dashed border-gray-300 dark:border-gray-600 text-gray-900 dark:text-gray-100 hover:bg-gray-50 dark:hover:bg-gray-800 hover:-translate-y-0.5 focus:ring-gray-300 dark:focus:ring-gray-600',
    }

    const textColorClasses = {
      primary: 'text-primary dark:text-primary-dark-mode hover:bg-gray-100 dark:hover:bg-gray-800 focus:ring-primary dark:focus:ring-primary-dark-mode',
      secondary: 'text-purple-600 dark:text-purple-400 hover:bg-purple-500/10 focus:ring-purple-500',
      error: 'text-red-600 dark:text-red-400 hover:bg-red-500/10 focus:ring-red-500',
      warning: 'text-yellow-600 dark:text-yellow-400 hover:bg-yellow-500/10 focus:ring-yellow-500',
      info: 'text-cyan-600 dark:text-cyan-400 hover:bg-cyan-500/10 focus:ring-cyan-500',
      success: 'text-green-600 dark:text-green-400 hover:bg-green-500/10 focus:ring-green-500',
      inherit: 'text-gray-900 dark:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-800 focus:ring-gray-300 dark:focus:ring-gray-600',
    }

    const staticVariantClasses = {
      default:
        'bg-gray-100 dark:bg-gray-800 text-gray-900 dark:text-gray-100 hover:bg-gray-200 dark:hover:bg-gray-700 focus:ring-gray-300 dark:focus:ring-gray-600',
      ghost:
        'text-gray-900 dark:text-gray-100 hover:bg-gray-100 dark:hover:bg-gray-800 focus:ring-gray-300 dark:focus:ring-gray-600',
    }

    const normalizedVariant =
      variant === 'contained'
        ? 'contained'
        : variant === 'outline'
          ? 'outlined'
          : variant === 'primary'
            ? 'contained'
            : variant === 'danger' || variant === 'destructive'
              ? 'contained'
              : variant

    const normalizedColor =
      variant === 'danger' || variant === 'destructive'
        ? 'error'
        : color ?? 'primary'

    const variantClasses =
      normalizedVariant === 'contained'
        ? containedColorClasses[normalizedColor]
        : normalizedVariant === 'outlined'
          ? outlinedColorClasses[normalizedColor]
          : normalizedVariant === 'text'
            ? textColorClasses[normalizedColor]
            : staticVariantClasses[normalizedVariant]

    const sizeClasses = {
      small: 'px-4 py-2 text-[10px]',
      medium: 'px-6 py-3 text-xs',
      large: 'px-8 py-4 text-sm',
    }

    const normalizedSize = size === 'sm' ? 'small' : size

    const widthClass = fullWidth ? 'w-full' : ''

    const isDisabled = disabled || loading

    const renderLoadingIcon = () => (
      <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
    )

    return (
      <button
        ref={ref}
        disabled={isDisabled}
        className={`${baseClasses} ${variantClasses} ${sizeClasses[normalizedSize]} ${widthClass} ${className}`}
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
