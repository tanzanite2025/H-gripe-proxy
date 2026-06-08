import type { ReactNode } from 'react'

interface SettingActionButtonProps {
  children: ReactNode
  disabled?: boolean
  onClick: () => void | Promise<unknown>
}

export function SettingActionButton({
  children,
  disabled = false,
  onClick,
}: SettingActionButtonProps) {
  return (
    <button
      type="button"
      className="cursor-pointer whitespace-nowrap rounded-full border border-border px-3 py-0.5 text-xs text-text-secondary transition-colors hover:bg-white/5 disabled:cursor-not-allowed disabled:opacity-50"
      onClick={() => {
        void onClick()
      }}
      disabled={disabled}
    >
      {children}
    </button>
  )
}
