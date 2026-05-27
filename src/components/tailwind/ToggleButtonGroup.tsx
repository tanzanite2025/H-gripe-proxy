import React from 'react'

import { cn } from '@/utils/cn'

interface ToggleButtonGroupProps {
  value: string | string[]
  exclusive?: boolean
  onChange: (event: React.MouseEvent<HTMLElement>, value: string | string[]) => void
  children: React.ReactNode
  fullWidth?: boolean
  className?: string
}

interface ToggleButtonProps {
  value: string
  children: React.ReactNode
  className?: string
  disabled?: boolean
}

const ToggleButtonGroupContext = React.createContext<{
  value: string | string[]
  exclusive?: boolean
  onChange: (value: string) => void
} | null>(null)

export const ToggleButtonGroup: React.FC<ToggleButtonGroupProps> = ({
  value,
  exclusive = false,
  onChange,
  children,
  fullWidth = false,
  className,
}) => {
  const handleChange = (buttonValue: string) => {
    if (exclusive) {
      onChange(null as any, buttonValue)
    } else {
      const currentValues = Array.isArray(value) ? value : [value]
      const newValues = currentValues.includes(buttonValue)
        ? currentValues.filter((v) => v !== buttonValue)
        : [...currentValues, buttonValue]
      onChange(null as any, newValues)
    }
  }

  return (
    <ToggleButtonGroupContext.Provider value={{ value, exclusive, onChange: handleChange }}>
      <div
        className={cn(
          'inline-flex rounded-lg border border-gray-300 dark:border-gray-600',
          fullWidth && 'w-full',
          className,
        )}
      >
        {children}
      </div>
    </ToggleButtonGroupContext.Provider>
  )
}

export const ToggleButton: React.FC<ToggleButtonProps> = ({
  value,
  children,
  className,
  disabled = false,
}) => {
  const context = React.useContext(ToggleButtonGroupContext)

  if (!context) {
    throw new Error('ToggleButton must be used within ToggleButtonGroup')
  }

  const isSelected = Array.isArray(context.value)
    ? context.value.includes(value)
    : context.value === value

  const handleClick = () => {
    if (!disabled) {
      context.onChange(value)
    }
  }

  return (
    <button
      type="button"
      onClick={handleClick}
      disabled={disabled}
      className={cn(
        'flex flex-1 items-center justify-center px-4 py-2 text-sm font-medium transition-colors',
        'first:rounded-l-lg last:rounded-r-lg',
        'border-r border-gray-300 last:border-r-0 dark:border-gray-600',
        isSelected
          ? 'bg-primary-500 text-white dark:bg-primary-600'
          : 'bg-white text-gray-700 hover:bg-gray-50 dark:bg-gray-800 dark:text-gray-300 dark:hover:bg-gray-700',
        disabled && 'cursor-not-allowed opacity-50',
        className,
      )}
    >
      {children}
    </button>
  )
}
