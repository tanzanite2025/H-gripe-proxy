import {
  createContext,
  useContext,
  useMemo,
  type ChangeEvent,
  type ReactNode,
} from 'react'

import { cn } from '@/utils/cn'

type RadioGroupContextValue = {
  value?: string | number
  onChange?: (event: ChangeEvent<HTMLInputElement>) => void
  name?: string
}

const RadioGroupContext = createContext<RadioGroupContextValue>({})

export interface RadioGroupProps {
  children: ReactNode
  value?: string | number
  onChange?: (event: ChangeEvent<HTMLInputElement>) => void
  name?: string
  className?: string
}

export const RadioGroup = ({
  children,
  value,
  onChange,
  name,
  className,
}: RadioGroupProps) => {
  const contextValue = useMemo(() => ({ value, onChange, name }), [name, onChange, value])
  return (
    <RadioGroupContext.Provider value={contextValue}>
      <div role="radiogroup" className={className}>
        {children}
      </div>
    </RadioGroupContext.Provider>
  )
}

export interface RadioProps {
  value: string | number
  checked?: boolean
  onChange?: (event: ChangeEvent<HTMLInputElement>) => void
  name?: string
  disabled?: boolean
  className?: string
}

export const Radio = ({
  value,
  checked,
  onChange,
  name,
  disabled = false,
  className,
}: RadioProps) => {
  const group = useContext(RadioGroupContext)
  const isChecked = checked ?? group.value === value
  const handleChange = (event: ChangeEvent<HTMLInputElement>) => {
    onChange?.(event)
    group.onChange?.(event)
  }

  return (
    <input
      type="radio"
      value={value}
      checked={isChecked}
      onChange={handleChange}
      name={name ?? group.name}
      disabled={disabled}
      className={cn(
        'mt-0.5 h-4 w-4 border-gray-300 text-primary focus:ring-primary dark:border-gray-600 dark:bg-gray-800',
        disabled && 'cursor-not-allowed opacity-50',
        className,
      )}
    />
  )
}
