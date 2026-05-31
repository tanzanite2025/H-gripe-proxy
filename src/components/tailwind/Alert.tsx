import { AlertCircle, AlertTriangle, Info, CheckCircle } from 'lucide-react'
import React from 'react'

import { cn } from '@/utils/cn'

export interface AlertProps {
  children?: React.ReactNode
  severity?: 'error' | 'warning' | 'info' | 'success'
  variant?: 'standard' | 'filled' | 'outlined' | 'default' | 'destructive'
  className?: string
  action?: React.ReactNode
  onContextMenu?: (event: React.MouseEvent<HTMLDivElement>) => void
  onClose?: () => void
}

export const Alert = React.forwardRef<HTMLDivElement, AlertProps>(
  (
    {
      children,
      severity = 'info',
      variant = 'standard',
      className = '',
      action,
      onContextMenu,
      onClose,
      ...props
    },
    ref,
  ) => {
    const severityConfig = {
      error: {
        standard: {
          bg: 'bg-red-50 dark:bg-red-900/20',
          border: 'border-red-200 dark:border-red-800',
          text: 'text-red-800 dark:text-red-200',
        },
        filled: {
          bg: 'bg-red-600 dark:bg-red-700',
          border: 'border-red-600 dark:border-red-700',
          text: 'text-white',
        },
        outlined: {
          bg: 'bg-transparent',
          border: 'border-red-600 dark:border-red-700',
          text: 'text-red-600 dark:text-red-400',
        },
        default: {
          bg: 'bg-red-50 dark:bg-red-900/20',
          border: 'border-red-200 dark:border-red-800',
          text: 'text-red-800 dark:text-red-200',
        },
        destructive: {
          bg: 'bg-red-600 dark:bg-red-700',
          border: 'border-red-600 dark:border-red-700',
          text: 'text-white',
        },
        icon: <AlertCircle className="h-5 w-5" />,
      },
      warning: {
        standard: {
          bg: 'bg-yellow-50 dark:bg-yellow-900/20',
          border: 'border-yellow-200 dark:border-yellow-800',
          text: 'text-yellow-800 dark:text-yellow-200',
        },
        filled: {
          bg: 'bg-yellow-600 dark:bg-yellow-700',
          border: 'border-yellow-600 dark:border-yellow-700',
          text: 'text-white',
        },
        outlined: {
          bg: 'bg-transparent',
          border: 'border-yellow-600 dark:border-yellow-700',
          text: 'text-yellow-600 dark:text-yellow-400',
        },
        default: {
          bg: 'bg-yellow-50 dark:bg-yellow-900/20',
          border: 'border-yellow-200 dark:border-yellow-800',
          text: 'text-yellow-800 dark:text-yellow-200',
        },
        destructive: {
          bg: 'bg-yellow-600 dark:bg-yellow-700',
          border: 'border-yellow-600 dark:border-yellow-700',
          text: 'text-white',
        },
        icon: <AlertTriangle className="h-5 w-5" />,
      },
      info: {
        standard: {
          bg: 'bg-teal-50 dark:bg-teal-900/20',
          border: 'border-teal-200 dark:border-teal-800',
          text: 'text-teal-800 dark:text-teal-200',
        },
        filled: {
          bg: 'bg-teal-600 dark:bg-teal-700',
          border: 'border-teal-600 dark:border-teal-700',
          text: 'text-white',
        },
        outlined: {
          bg: 'bg-transparent',
          border: 'border-teal-600 dark:border-teal-700',
          text: 'text-teal-600 dark:text-teal-400',
        },
        default: {
          bg: 'bg-teal-50 dark:bg-teal-900/20',
          border: 'border-teal-200 dark:border-teal-800',
          text: 'text-teal-800 dark:text-teal-200',
        },
        destructive: {
          bg: 'bg-teal-600 dark:bg-teal-700',
          border: 'border-teal-600 dark:border-teal-700',
          text: 'text-white',
        },
        icon: <Info className="h-5 w-5" />,
      },
      success: {
        standard: {
          bg: 'bg-green-50 dark:bg-green-900/20',
          border: 'border-green-200 dark:border-green-800',
          text: 'text-green-800 dark:text-green-200',
        },
        filled: {
          bg: 'bg-green-600 dark:bg-green-700',
          border: 'border-green-600 dark:border-green-700',
          text: 'text-white',
        },
        outlined: {
          bg: 'bg-transparent',
          border: 'border-green-600 dark:border-green-700',
          text: 'text-green-600 dark:text-green-400',
        },
        default: {
          bg: 'bg-green-50 dark:bg-green-900/20',
          border: 'border-green-200 dark:border-green-800',
          text: 'text-green-800 dark:text-green-200',
        },
        destructive: {
          bg: 'bg-green-600 dark:bg-green-700',
          border: 'border-green-600 dark:border-green-700',
          text: 'text-white',
        },
        icon: <CheckCircle className="h-5 w-5" />,
      },
    }

    const config = severityConfig[severity][variant]
    const icon = severityConfig[severity].icon

    return (
      <div
        ref={ref}
        className={cn(
          'flex items-start gap-3 rounded-lg border p-4',
          config.bg,
          config.border,
          config.text,
          className,
        )}
        role="alert"
        onContextMenu={onContextMenu}
        {...props}
      >
        <div className="mt-0.5 flex-shrink-0">{icon}</div>
        <div className="flex-1">{children}</div>
        {action && <div className="ml-auto flex-shrink-0">{action}</div>}
        {!action && onClose && (
          <button
            type="button"
            onClick={onClose}
            className="ml-auto flex-shrink-0 text-current/70 transition-opacity hover:opacity-100"
            aria-label="Close alert"
          >
            ×
          </button>
        )}
      </div>
    )
  },
)

Alert.displayName = 'Alert'
