import { Listbox, Transition } from '@headlessui/react'
import { Check, ChevronDown } from 'lucide-react'
import { Fragment, type ChangeEvent, type CSSProperties, type ReactNode } from 'react'

import { cn } from '@/utils/cn'

export interface SelectOption {
  value: string | number
  label: string
  disabled?: boolean
}

export type SelectPrimitiveValue = string | number
export type SelectValue = SelectPrimitiveValue | string[]

export type SelectChangeEvent<T = string> = ChangeEvent<HTMLSelectElement> & {
  target: HTMLSelectElement & { value: T }
}

interface SelectBaseProps {
  options?: SelectOption[]
  label?: string
  placeholder?: string
  error?: string
  disabled?: boolean
  fullWidth?: boolean
  size?: 'small' | 'medium' | 'large'
  className?: string
  labelId?: string
  variant?: 'outlined' | 'filled' | 'standard'
  renderValue?: (selected: SelectPrimitiveValue) => ReactNode
  MenuProps?: {
    slotProps?: {
      paper?: {
        className?: string
        style?: CSSProperties
      }
    }
  }
}

export interface NativeSelectProps extends Omit<SelectBaseProps, 'options'> {
  children: ReactNode
  options?: never
  value?: SelectValue
  onChange?: (event: SelectChangeEvent<SelectValue>) => void
  multiple?: boolean
}

export interface OptionSelectProps extends SelectBaseProps {
  children?: never
  options: SelectOption[]
  value?: SelectPrimitiveValue
  onChange?: (value: SelectPrimitiveValue) => void
  multiple?: never
}

export type SelectProps = NativeSelectProps | OptionSelectProps

export const Select = ({
  value,
  onChange,
  options,
  label,
  placeholder = 'Select an option',
  error,
  disabled = false,
  children,
  fullWidth = false,
  size = 'medium',
  className,
  labelId,
  renderValue,
  MenuProps,
  multiple = false,
}: SelectProps) => {
  const sizeClasses = {
    small: 'h-10 text-xs',
    medium: 'h-12 text-sm',
    large: 'h-14 text-base',
  }

  const widthClass = fullWidth ? 'w-full' : ''

  // If children are provided, support both single and multiple selection
  if (children) {
    // Multiple-select with dropdown menu
    if (multiple) {
      const optionNodes = (Array.isArray(children) ? children : [children]) as React.ReactElement<
        { value: SelectPrimitiveValue; children: ReactNode; disabled?: boolean }
      >[]

      const optionList: SelectOption[] = optionNodes.map((child) => ({
        value: child.props.value,
        label: String(child.props.children ?? child.props.value),
        disabled: child.props.disabled,
      }))

      const selectedValues = (Array.isArray(value) ? value : []) as SelectPrimitiveValue[]

      const displayLabel =
        selectedValues.length === 0
          ? placeholder
          : selectedValues.map((v) => optionList.find((o) => o.value === v)?.label ?? String(v)).join(', ')

      return (
        <div className={cn(widthClass, className)}>
          {label && (
            <label className="mb-2 block text-xs font-semibold uppercase tracking-widest text-text-secondary">
              {label}
            </label>
          )}
          <Listbox
            value={selectedValues}
            onChange={(nextValues: SelectPrimitiveValue[]) => {
              const syntheticEvent = {
                target: {
                  value: nextValues,
                },
              } as SelectChangeEvent<SelectValue>

              ;(onChange as ((event: SelectChangeEvent<SelectValue>) => void) | undefined)?.(
                syntheticEvent,
              )
            }}
            multiple
            disabled={disabled}
          >
            <div className="relative">
              <Listbox.Button
                aria-labelledby={labelId}
                className={cn(
                  `relative w-full ${sizeClasses[size]} px-4 rounded-input bg-card border border-border text-left text-sm font-semibold text-text-primary transition-all duration-300 ease-smooth focus:outline-none focus:ring-2 focus:border-primary ${
                    error
                      ? 'ring-2 ring-red-500 dark:ring-red-400'
                      : 'focus:ring-primary dark:focus:ring-primary-dark-mode'
                  } ${disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`,
                )}
                aria-invalid={!!error}
              >
                <span className="block truncate">{displayLabel}</span>
                <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-4">
                  <ChevronDown className="h-4 w-4 text-gray-400" aria-hidden="true" />
                </span>
              </Listbox.Button>

              <Transition
                as={Fragment}
                leave="transition ease-in duration-100"
                leaveFrom="opacity-100"
                leaveTo="opacity-0"
              >
                <Listbox.Options className="absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-input bg-card py-1 shadow-lg ring-1 ring-white/5 focus:outline-none">
                  {optionList.map((option) => (
                    <Listbox.Option
                      key={option.value}
                      value={option.value}
                      disabled={option.disabled}
                      className={({ active }) =>
                        `relative cursor-pointer select-none py-2 pl-10 pr-4 ${
                          active ? 'bg-white/5' : ''
                        } ${option.disabled ? 'cursor-not-allowed opacity-50' : ''}`
                      }
                    >
                      {({ selected }) => (
                        <>
                          <span
                            className={`block truncate text-sm ${
                              selected ? 'font-bold' : 'font-normal'
                            } text-text-primary`}
                          >
                            {option.label}
                          </span>
                          {selected && (
                            <span className="absolute inset-y-0 left-0 flex items-center pl-3 text-primary dark:text-primary-dark-mode">
                              <Check className="h-4 w-4" aria-hidden="true" />
                            </span>
                          )}
                        </>
                      )}
                    </Listbox.Option>
                  ))}
                </Listbox.Options>
              </Transition>
            </div>
          </Listbox>
          {error && (
            <p className="mt-1 text-xs text-red-500 dark:text-red-400">{error}</p>
          )}
        </div>
      )
    }

    // Single-select native <select> (MUI-style API)
    return (
      <div className={cn(fullWidth ? 'w-full' : '', className)}>
        {label && (
          <label className="mb-2 block text-xs font-semibold uppercase tracking-widest text-text-secondary">
            {label}
          </label>
        )}
        <select
          value={value ?? ''}
          multiple={false}
          onChange={(e) => {
            const nextValue = e.target.value
            const syntheticEvent = {
              ...e,
              target: {
                ...e.target,
                value: nextValue,
              },
            } as SelectChangeEvent<SelectValue>

            ;(onChange as ((event: SelectChangeEvent<SelectValue>) => void) | undefined)?.(
              syntheticEvent,
            )
          }}
          disabled={disabled}
          aria-labelledby={labelId}
          className={cn(
            `relative w-full ${sizeClasses[size]} px-4 rounded-input bg-card border border-border font-semibold text-text-primary transition-all duration-300 ease-smooth focus:outline-none focus:ring-2 focus:border-primary ${
              error
                ? 'ring-2 ring-red-500 dark:ring-red-400'
                : 'focus:ring-primary dark:focus:ring-primary-dark-mode'
            } ${disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`,
          )}
          aria-invalid={!!error}
        >
          {children}
        </select>
        {error && (
          <p className="mt-1 text-xs text-red-500 dark:text-red-400">{error}</p>
        )}
      </div>
    )
  }

  // Original options-based API
  if (!options || !onChange) {
    return null
  }

  const selectedOption = options.find((opt) => opt.value === value)

  const id = label?.toLowerCase().replace(/\s+/g, '-')
  const paperClassName = MenuProps?.slotProps?.paper?.className
  const paperStyle = MenuProps?.slotProps?.paper?.style
  const renderedValue = renderValue
    ? renderValue(value ?? '')
    : selectedOption?.label || placeholder

  return (
    <div className={cn(fullWidth ? 'w-full' : '', className)}>
      {label && (
        <label
          htmlFor={id}
          className="mb-2 block text-xs font-semibold uppercase tracking-widest text-text-secondary"
        >
          {label}
        </label>
      )}
      <Listbox
        value={value}
        onChange={(nextValue) => {
          ;(onChange as ((value: SelectPrimitiveValue) => void))(nextValue)
        }}
        disabled={disabled}
      >
        <div className="relative">
          <Listbox.Button
            id={id}
            aria-labelledby={labelId}
            className={cn(
              `relative w-full h-12 px-4 rounded-input bg-card border border-border text-left text-sm font-semibold text-text-primary transition-all duration-300 ease-smooth focus:outline-none focus:ring-2 focus:border-primary ${
                error
                  ? 'ring-2 ring-red-500 dark:ring-red-400'
                  : 'focus:ring-primary dark:focus:ring-primary-dark-mode'
              } ${disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'}`
            )}
            aria-invalid={!!error}
          >
            <span className="block truncate">
              {renderedValue}
            </span>
            <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-4">
              <ChevronDown className="h-4 w-4 text-gray-400" aria-hidden="true" />
            </span>
          </Listbox.Button>

          <Transition
            as={Fragment}
            leave="transition ease-in duration-100"
            leaveFrom="opacity-100"
            leaveTo="opacity-0"
          >
            <Listbox.Options
              style={paperStyle}
              className={cn(
                'absolute z-10 mt-1 max-h-60 w-full overflow-auto rounded-input bg-card py-1 shadow-lg ring-1 ring-white/5 focus:outline-none',
                paperClassName,
              )}
            >
              {options.map((option) => (
                <Listbox.Option
                  key={option.value}
                  value={option.value}
                  disabled={option.disabled}
                  className={({ active }) =>
                    `relative cursor-pointer select-none py-2 pl-10 pr-4 ${
                      active ? 'bg-white/5' : ''
                    } ${option.disabled ? 'cursor-not-allowed opacity-50' : ''}`
                  }
                >
                  {({ selected }) => (
                    <>
                      <span
                        className={`block truncate text-sm ${
                          selected ? 'font-bold' : 'font-normal'
                        } text-text-primary`}
                      >
                        {option.label}
                      </span>
                      {selected && (
                        <span className="absolute inset-y-0 left-0 flex items-center pl-3 text-primary dark:text-primary-dark-mode">
                          <Check className="h-4 w-4" aria-hidden="true" />
                        </span>
                      )}
                    </>
                  )}
                </Listbox.Option>
              ))}
            </Listbox.Options>
          </Transition>
        </div>
      </Listbox>
      {error && (
        <p className="mt-1 text-xs text-red-500 dark:text-red-400">{error}</p>
      )}
    </div>
  )
}

Select.displayName = 'Select'


// MUI-compatible exports
export const FormControl = ({ children, fullWidth, variant, size, ...props }: { children: ReactNode; [key: string]: any }) => (
  <div {...props}>{children}</div>
)

export const InputLabel = ({ children, htmlFor, ...props }: { children: ReactNode; htmlFor?: string; [key: string]: any }) => (
  <label htmlFor={htmlFor} className="mb-2 block text-xs font-semibold uppercase tracking-widest text-text-secondary" {...props}>
    {children}
  </label>
)

export const MenuItem = ({ value, children, ...props }: { value: string | number; children: ReactNode; [key: string]: any }) => (
  <option value={value} {...props}>
    {children}
  </option>
)

export const SelectMenuItem = MenuItem

Select.displayName = 'Select'
