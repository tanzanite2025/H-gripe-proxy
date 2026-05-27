import { Switch as HeadlessSwitch } from '@headlessui/react'

export interface SwitchProps {
  checked: boolean
  onChange: (checked: boolean) => void
  label?: string
  disabled?: boolean
}

export const Switch = ({ checked, onChange, label, disabled = false }: SwitchProps) => {
  return (
    <HeadlessSwitch.Group>
      <div className="flex items-center gap-3">
        <HeadlessSwitch
          checked={checked}
          onChange={onChange}
          disabled={disabled}
          className={`${
            checked
              ? 'bg-primary dark:bg-primary-dark-mode'
              : 'bg-gray-200 dark:bg-gray-700'
          } relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-primary dark:focus:ring-primary-dark-mode focus:ring-offset-2 ${
            disabled ? 'cursor-not-allowed opacity-50' : 'cursor-pointer'
          }`}
        >
          <span
            className={`${
              checked ? 'translate-x-6' : 'translate-x-1'
            } inline-block h-4 w-4 transform rounded-full bg-white transition-transform`}
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
