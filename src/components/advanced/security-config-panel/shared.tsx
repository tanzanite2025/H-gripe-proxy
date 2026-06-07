import type { ReactNode } from 'react'

import { Switch } from '@/components/tailwind'

interface SectionCardProps {
  title: string
  description: string
  aside?: ReactNode
  children: ReactNode
}

export function SectionCard({
  title,
  description,
  aside,
  children,
}: SectionCardProps) {
  return (
    <section className="rounded-lg border border-border bg-card p-4">
      <div className="mb-4 flex items-start justify-between gap-3">
        <div>
          <h3 className="text-lg font-semibold">{title}</h3>
          <p className="mt-1 text-sm text-muted-foreground">{description}</p>
        </div>
        {aside}
      </div>
      <div className="space-y-4">{children}</div>
    </section>
  )
}

interface ToggleRowProps {
  title: string
  description?: string
  checked: boolean
  disabled?: boolean
  onCheckedChange: (checked: boolean) => void
}

export function ToggleRow({
  title,
  description,
  checked,
  disabled = false,
  onCheckedChange,
}: ToggleRowProps) {
  return (
    <div className="flex items-start justify-between gap-3">
      <div className="flex-1">
        <p className="text-sm font-medium">{title}</p>
        {description ? (
          <p className="mt-1 text-xs text-muted-foreground">{description}</p>
        ) : null}
      </div>
      <Switch
        checked={checked}
        disabled={disabled}
        onCheckedChange={onCheckedChange}
      />
    </div>
  )
}

interface ChoiceButtonProps {
  active: boolean
  onClick: () => void
  children: ReactNode
}

export function ChoiceButton({
  active,
  onClick,
  children,
}: ChoiceButtonProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`rounded-full px-3 py-1 text-sm transition-colors ${
        active
          ? 'bg-primary text-primary-foreground'
          : 'bg-secondary text-secondary-foreground hover:bg-secondary/80'
      }`}
    >
      {children}
    </button>
  )
}
