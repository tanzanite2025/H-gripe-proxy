import {
  forwardRef,
  type CSSProperties,
  type ChangeEventHandler,
  type FocusEventHandler,
  type InputHTMLAttributes,
  type ReactNode,
  type TextareaHTMLAttributes,
} from 'react'

interface BaseTextFieldProps {
  label?: string
  error?: ReactNode | boolean
  helperText?: string
  multiline?: boolean
  rows?: number
  variant?: 'outlined' | 'filled' | 'standard'
  fullWidth?: boolean
  size?: 'small' | 'medium' | 'large'
  inputClassName?: string
  onChange?:
    | ChangeEventHandler<HTMLInputElement>
    | ChangeEventHandler<HTMLTextAreaElement>
  onBlur?:
    | FocusEventHandler<HTMLInputElement>
    | FocusEventHandler<HTMLTextAreaElement>
  InputProps?: {
    startAdornment?: ReactNode
    endAdornment?: ReactNode
    className?: string
    style?: CSSProperties
  }
  slotProps?: {
    input?: {
      startAdornment?: ReactNode
      endAdornment?: ReactNode
      className?: string
      style?: CSSProperties
      sx?: unknown
    }
    htmlInput?: InputHTMLAttributes<HTMLInputElement>
  }
}

type TextFieldProps = BaseTextFieldProps &
  Omit<InputHTMLAttributes<HTMLInputElement>, 'size' | 'onChange' | 'onBlur'> &
  Omit<TextareaHTMLAttributes<HTMLTextAreaElement>, 'onChange' | 'onBlur'>

export const TextField = forwardRef<
  HTMLInputElement | HTMLTextAreaElement,
  TextFieldProps
>(({ label, error, helperText, multiline = false, rows = 3, variant = 'outlined', fullWidth = false, size = 'medium', className = '', inputClassName = '', type, InputProps, slotProps, ...props }, ref) => {
  const sizeClasses = {
    small: 'h-10 text-xs',
    medium: 'h-12 text-sm',
    large: 'h-14 text-base',
  }

  const baseClasses =
    'px-4 rounded-input bg-card border border-border font-semibold text-text-primary placeholder:text-text-secondary transition-all duration-300 ease-smooth focus:outline-none focus:ring-2 focus:border-primary focus:bg-card'

  const hasError = Boolean(error)
  const mergedInputProps = slotProps?.input ?? InputProps
  const startAdornment = mergedInputProps?.startAdornment
  const endAdornment = mergedInputProps?.endAdornment
  const adornmentClassName = mergedInputProps?.className ?? ''
  const adornmentStyle = mergedInputProps?.style
  const htmlInputProps = slotProps?.htmlInput

  const errorClasses = hasError
    ? 'ring-2 ring-red-500 dark:ring-red-400 focus:ring-red-500 dark:focus:ring-red-400'
    : 'focus:ring-primary dark:focus:ring-primary-dark-mode'

  const normalizedSize = size === 'small' || size === 'medium' || size === 'large' ? size : 'medium'
  const heightClasses = multiline ? '' : sizeClasses[normalizedSize]
  const widthClass = fullWidth ? 'w-full' : ''

  const inputClasses = `${baseClasses} ${errorClasses} ${heightClasses} ${widthClass} ${startAdornment ? 'pl-10' : ''} ${endAdornment ? 'pr-10' : ''} ${className} ${inputClassName} ${adornmentClassName}`
  const errorContent = error === true ? null : error

  const id = props.id || label?.toLowerCase().replace(/\s+/g, '-')

  return (
    <div className="w-full">
      {label && (
        <label
          htmlFor={id}
          className="mb-2 block text-xs font-semibold uppercase tracking-widest text-text-secondary"
        >
          {label}
        </label>
      )}
      <div className="relative w-full" style={adornmentStyle}>
        {startAdornment && (
          <div className="pointer-events-none absolute inset-y-0 left-3 flex items-center">
            {startAdornment}
          </div>
        )}
        {multiline ? (
          <textarea
            ref={ref as React.Ref<HTMLTextAreaElement>}
            id={id}
            rows={rows}
            className={inputClasses}
            aria-invalid={hasError}
            aria-describedby={hasError ? `${id}-error` : helperText ? `${id}-helper` : undefined}
            {...(props as TextareaHTMLAttributes<HTMLTextAreaElement>)}
          />
        ) : (
          <input
            ref={ref as React.Ref<HTMLInputElement>}
            id={id}
            type={type}
            className={inputClasses}
            aria-invalid={hasError}
            aria-describedby={hasError ? `${id}-error` : helperText ? `${id}-helper` : undefined}
            {...htmlInputProps}
            {...(props as InputHTMLAttributes<HTMLInputElement>)}
          />
        )}
        {endAdornment && (
          <div className="absolute inset-y-0 right-3 flex items-center">
            {endAdornment}
          </div>
        )}
      </div>
      {hasError && errorContent && (
        <p id={`${id}-error`} className="mt-1 text-xs text-red-500 dark:text-red-400">
          {errorContent}
        </p>
      )}
      {helperText && !hasError && (
        <p
          id={`${id}-helper`}
          className="mt-1 text-xs text-gray-500 dark:text-gray-400"
        >
          {helperText}
        </p>
      )}
    </div>
  )
})

TextField.displayName = 'TextField'
