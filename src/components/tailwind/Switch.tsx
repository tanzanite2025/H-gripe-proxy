import { Switch as HeadlessSwitch } from '@headlessui/react'
import React from 'react'

import { cn } from '@/utils/cn'

export interface SwitchProps {
  checked?: boolean
  onChange?:
    | ((event: React.ChangeEvent<HTMLInputElement>) => void)
    | ((event: React.ChangeEvent<HTMLInputElement>, checked: boolean) => void)
  onCheckedChange?: (checked: boolean) => void
  label?: string
  disabled?: boolean
  name?: string
  value?: string | boolean
  onBlur?: React.FocusEventHandler<HTMLButtonElement>
  size?: 'small' | 'medium'
  className?: string
}

export const Switch = ({ checked = false, onChange, onCheckedChange, label, disabled = false, name, value, onBlur, size = 'medium', className = '' }: SwitchProps) => {
  const handleChange = (newChecked: boolean) => {
    if (onCheckedChange) {
      onCheckedChange(newChecked)
    }

    if (onChange) {
      const syntheticEvent = {
        target: { checked: newChecked, value },
      } as React.ChangeEvent<HTMLInputElement>

      if (onChange.length >= 2) {
        ;(onChange as (event: React.ChangeEvent<HTMLInputElement>, checked: boolean) => void)(
          syntheticEvent,
          newChecked,
        )
      } else {
        ;(onChange as (event: React.ChangeEvent<HTMLInputElement>) => void)(syntheticEvent)
      }
    }
  }

  const sizeClasses = {
    small: {
      track: 'h-5 w-9',
      thumb: checked ? 'translate-x-5 h-3.5 w-3.5' : 'translate-x-0.5 h-3.5 w-3.5',
    },
    medium: {
      track: 'h-6 w-11',
      thumb: checked ? 'translate-x-6 h-4 w-4' : 'translate-x-1 h-4 w-4',
    },
  }

  return (
    <HeadlessSwitch.Group>
      <div className={cn('flex items-center gap-3', className)}>
        <HeadlessSwitch
          checked={checked}
          onChange={handleChange}
          disabled={disabled}
          name={name}
          onBlur={onBlur}
          data-value={value}
          className={cn(
            checked
              ? 'bg-primary dark:bg-primary-dark-mode'
              : 'bg-gray-200 dark:bg-gray-700'
            ,
            'relative inline-flex items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-primary dark:focus:ring-primary-dark-mode focus:ring-offset-2',
            sizeClasses[size].track,
            disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'
          )}
        >
          <span
            className={cn(
              'inline-block transform rounded-full bg-white transition-transform',
              sizeClasses[size].thumb,
            )}
          />
        </HeadlessSwitch>
        {label && (
          <HeadlessSwitch.Label className="text-sm font-semibold text-gray-900 dark:text-gray-100 cursor-pointer">
            {label}
          </HeadlessSwitch.Label>
        )}
      </div>
    </HeadlessSwitch.Group>
  )
}

Switch.displayName = 'Switch'
