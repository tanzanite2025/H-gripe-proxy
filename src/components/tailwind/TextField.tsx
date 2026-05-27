import { forwardRef, type InputHTMLAttributes, type TextareaHTMLAttributes } from 'react'

interface BaseTextFieldProps {
  label?: string
  error?: string
  helperText?: string
  multiline?: boolean
  rows?: number
}

type TextFieldProps = BaseTextFieldProps &
  (
    | ({ multiline?: false } & InputHTMLAttributes<HTMLInputElement>)
    | ({ multiline: true } & TextareaHTMLAttributes<HTMLTextAreaElement>)
  )

export const TextField = forwardRef<
  HTMLInputElement | HTMLTextAreaElement,
  TextFieldProps
>(({ label, error, helperText, multiline = false, rows = 3, className = '', ...props }, ref) => {
  const baseClasses =
    'w-full px-4 rounded-input bg-gray-50 dark:bg-gray-800/50 border-0 text-sm font-semibold text-gray-900 dark:text-gray-100 placeholder:text-gray-400 dark:placeholder:text-gray-500 transition-all duration-300 ease-smooth focus:outline-none focus:ring-2 focus:bg-white dark:focus:bg-gray-800'

  const errorClasses = error
    ? 'ring-2 ring-red-500 dark:ring-red-400 focus:ring-red-500 dark:focus:ring-red-400'
    : 'focus:ring-primary dark:focus:ring-primary-dark-mode'

  const heightClasses = multiline ? '' : 'h-12'

  const inputClasses = `${baseClasses} ${errorClasses} ${heightClasses} ${className}`

  const id = props.id || label?.toLowerCase().replace(/\s+/g, '-')

  return (
    <div className="w-full">
      {label && (
        <label
          htmlFor={id}
          className="mb-2 block text-xs font-black uppercase tracking-widest text-gray-700 dark:text-gray-300"
        >
          {label}
        </label>
      )}
      {multiline ? (
        <textarea
          ref={ref as React.Ref<HTMLTextAreaElement>}
          id={id}
          rows={rows}
          className={inputClasses}
          aria-invalid={!!error}
          aria-describedby={error ? `${id}-error` : helperText ? `${id}-helper` : undefined}
          {...(props as TextareaHTMLAttributes<HTMLTextAreaElement>)}
        />
      ) : (
        <input
          ref={ref as React.Ref<HTMLInputElement>}
          id={id}
          className={inputClasses}
          aria-invalid={!!error}
          aria-describedby={error ? `${id}-error` : helperText ? `${id}-helper` : undefined}
          {...(props as InputHTMLAttributes<HTMLInputElement>)}
        />
      )}
      {error && (
        <p id={`${id}-error`} className="mt-1 text-xs text-red-500 dark:text-red-400">
          {error}
        </p>
      )}
      {helperText && !error && (
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
