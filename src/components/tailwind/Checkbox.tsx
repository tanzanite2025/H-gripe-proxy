import { Check } from 'lucide-react'
import { forwardRef, type ChangeEvent } from 'react'

import { cn } from '@/utils/cn'

export interface CheckboxProps {
  checked?: boolean
  onChange?: (event: ChangeEvent<HTMLInputElement>, checked: boolean) => void
  disabled?: boolean
  className?: string
  color?: 'primary' | 'secondary' | 'default'
  size?: 'small' | 'medium'
  edge?: 'start' | 'end' | false
}

export const Checkbox = forwardRef<HTMLInputElement, CheckboxProps>(
  ({ checked, onChange, disabled, className, color = 'primary', size = 'medium', edge = false }, ref) => {
    const sizeClasses = {
      small: 'h-4 w-4',
      medium: 'h-5 w-5',
    }

    const colorClasses = {
      primary: 'border-primary checked:bg-primary',
      secondary: 'border-secondary checked:bg-secondary',
      default: 'border-gray-400 checked:bg-gray-600',
    }

    const edgeClasses = edge === 'start' ? '-ml-2' : edge === 'end' ? '-mr-2' : ''

    return (
      <div className={cn('relative inline-flex items-center', edgeClasses)}>
        <input
          ref={ref}
          type="checkbox"
          checked={checked}
          onChange={(e) => onChange?.(e, e.target.checked)}
          disabled={disabled}
          className={cn(
            'appearance-none border-2 rounded cursor-pointer transition-colors',
            'focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2',
            sizeClasses[size],
            colorClasses[color],
            disabled && 'opacity-50 cursor-not-allowed',
            className
          )}
        />
        {checked && (
          <Check
            className={cn(
              'absolute pointer-events-none text-white',
              size === 'small' ? 'h-3 w-3' : 'h-4 w-4'
            )}
            style={{ left: '50%', top: '50%', transform: 'translate(-50%, -50%)' }}
          />
        )}
      </div>
    )
  }
)

Checkbox.displayName = 'Checkbox'
