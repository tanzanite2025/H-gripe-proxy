import type { ReactNode } from 'react'

import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import { LinearProgress } from '@/components/tailwind/LinearProgress'

import type { DnsStatusColor } from '../dns-runtime-view-model'

interface DnsCardStateProps {
  title: string
  icon: ReactNode
  message: string
  loading?: boolean
}

interface DnsSectionHeadingProps {
  title: string
  icon?: ReactNode
}

interface DnsTextRowProps {
  label: string
  value: ReactNode
  valueTitle?: string
  valueClassName?: string
}

interface DnsChipRowProps {
  label: string
  chipLabel: string
  chipColor: DnsStatusColor
  chipIcon?: ReactNode
}

export function DnsCardState({
  title,
  icon,
  message,
  loading = false,
}: DnsCardStateProps) {
  return (
    <Card>
      <div className="flex min-h-[200px] flex-col p-4">
        <div className="mb-2 flex items-center gap-1 text-sm font-semibold">
          {icon}
          {title}
        </div>
        <div className="flex flex-1 items-center justify-center">
          <div
            className={
              loading
                ? 'text-xs text-muted-foreground'
                : 'text-sm text-muted-foreground'
            }
          >
            {message}
          </div>
        </div>
        {loading && <LinearProgress />}
      </div>
    </Card>
  )
}

export function DnsSectionHeading({
  title,
  icon,
}: DnsSectionHeadingProps) {
  return (
    <div className="mb-1 flex items-center gap-0.5 text-xs text-gray-500 dark:text-gray-400">
      {icon}
      {title}
    </div>
  )
}

export function DnsTextRow({
  label,
  value,
  valueTitle,
  valueClassName,
}: DnsTextRowProps) {
  return (
    <div className="flex items-center justify-between">
      <div className="text-sm">{label}</div>
      <div
        className={
          valueClassName ??
          'text-sm font-bold'
        }
        title={valueTitle}
      >
        {value}
      </div>
    </div>
  )
}

export function DnsChipRow({
  label,
  chipLabel,
  chipColor,
  chipIcon,
}: DnsChipRowProps) {
  return (
    <div className="flex items-center justify-between">
      <div className="text-sm">{label}</div>
      <Chip
        icon={chipIcon}
        label={chipLabel}
        size="small"
        color={chipColor}
      />
    </div>
  )
}

export function DnsDivider() {
  return <div className="my-2 border-t border-gray-200 dark:border-gray-700" />
}
