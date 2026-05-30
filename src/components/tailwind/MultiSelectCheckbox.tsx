import { Popover, PopoverButton, PopoverPanel, Transition } from '@headlessui/react'
import { ChevronDown } from 'lucide-react'
import { Fragment } from 'react'

import { Checkbox } from '@/components/tailwind/Checkbox'
import { cn } from '@/utils/cn'

export interface MultiSelectCheckboxProps {
  options: { value: string; label: string }[]
  selected: string[]
  onChange: (selected: string[]) => void
  className?: string
  placeholder?: string
  size?: 'small' | 'medium'
}

export const MultiSelectCheckbox = ({
  options,
  selected,
  onChange,
  className,
  placeholder = '请选择',
  size = 'small',
}: MultiSelectCheckboxProps) => {
  const sizeClasses = {
    small: 'h-10 text-xs',
    medium: 'h-12 text-sm',
  }

  const displayText = selected.length === 0
    ? placeholder
    : selected.length <= 3
      ? selected.join(', ')
      : `${selected.length} 项已选`

  return (
    <Popover className={cn('relative w-[300px]', className)}>
      <PopoverButton
        className={cn(
          'relative w-full px-4 pr-10 rounded-input bg-card border border-border text-left font-semibold text-text-primary transition-all duration-300 ease-smooth focus:outline-none focus:ring-2 focus:border-primary focus:ring-primary dark:focus:ring-primary-dark-mode cursor-pointer',
          sizeClasses[size],
        )}
      >
        <span className={cn('block truncate', selected.length === 0 && 'text-text-secondary')}>
          {displayText}
        </span>
        <span className="pointer-events-none absolute inset-y-0 right-0 flex items-center pr-3">
          <ChevronDown className="h-4 w-4 text-gray-400" />
        </span>
      </PopoverButton>

      <Transition
        as={Fragment}
        leave="transition ease-in duration-100"
        leaveFrom="opacity-100"
        leaveTo="opacity-0"
      >
        <PopoverPanel
          className="absolute z-50 mt-1 w-full overflow-y-auto rounded-input border border-border bg-card py-1 shadow-lg"
          style={{ maxHeight: 240 }}
        >
          {options.map((opt) => (
            <label
              key={opt.value}
              className="flex items-center gap-2 px-3 py-1.5 cursor-pointer hover:bg-white/5 text-xs text-text-primary"
            >
              <Checkbox
                size="small"
                checked={selected.includes(opt.value)}
                onChange={(_, checked) => {
                  if (checked) {
                    onChange([...selected, opt.value])
                  } else {
                    onChange(selected.filter((v) => v !== opt.value))
                  }
                }}
              />
              <span className="truncate">{opt.label}</span>
            </label>
          ))}
        </PopoverPanel>
      </Transition>
    </Popover>
  )
}
