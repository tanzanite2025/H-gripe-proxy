import { AlertCircle, CheckCircle } from 'lucide-react'

import { Chip } from '@/components/tailwind'

import type { MonitorCardViewModel } from './helpers'

interface StatusCardProps {
  card: MonitorCardViewModel
}

export function StatusCard({ card }: StatusCardProps) {
  const isPositive = card.badgeColor === 'success'
  const isError = card.badgeColor === 'error'

  return (
    <div className="rounded-lg border border-border bg-card p-4">
      <div className="mb-3 flex items-start justify-between gap-3">
        <div className="flex items-center gap-2">
          {isPositive ? (
            <CheckCircle className="h-4 w-4 text-green-500" />
          ) : (
            <AlertCircle
              className={`h-4 w-4 ${
                isError ? 'text-red-500' : 'text-yellow-500'
              }`}
            />
          )}
          <h3 className="text-sm font-semibold">{card.title}</h3>
        </div>
        <Chip
          size="small"
          color={card.badgeColor}
          label={card.badgeLabel}
        />
      </div>

      <div className="space-y-2">
        {card.metrics.map((metric) => (
          <div
            key={`${card.title}-${metric.label}`}
            className="flex items-center justify-between gap-3"
          >
            <p className="text-sm text-muted-foreground">{metric.label}</p>
            <Chip
              size="small"
              color={metric.color ?? 'default'}
              label={metric.value}
            />
          </div>
        ))}
      </div>

      {card.description ? (
        <p className="mt-3 text-xs text-muted-foreground">{card.description}</p>
      ) : null}
    </div>
  )
}
