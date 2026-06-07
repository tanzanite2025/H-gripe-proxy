import type { ReactNode } from 'react'

import { Switch } from '@/components/tailwind/Switch'

interface SectionCardProps {
  title: string
  description: string
  children: ReactNode
}

export function SectionCard({
  title,
  description,
  children,
}: SectionCardProps) {
  return (
    <section className="rounded-xl border border-border bg-card px-4 py-4">
      <div className="mb-4">
        <h4 className="text-sm font-semibold text-text-primary">{title}</h4>
        <p className="mt-1 text-xs text-text-secondary">{description}</p>
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
        <p className="text-sm font-medium text-text-primary">{title}</p>
        {description ? (
          <p className="mt-1 text-xs text-text-secondary">{description}</p>
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
