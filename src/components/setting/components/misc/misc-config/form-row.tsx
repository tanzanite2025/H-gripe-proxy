import type { ReactNode } from 'react'

import { TooltipIcon } from '@/components/base'

interface MiscConfigFormRowProps {
  label: string
  tooltip?: string
  children: ReactNode
}

export function MiscConfigFormRow({
  label,
  tooltip,
  children,
}: MiscConfigFormRowProps) {
  return (
    <div className="flex items-center gap-3">
      <span className="shrink-0 text-sm font-medium text-text-primary">
        {label}
      </span>
      {tooltip ? <TooltipIcon title={tooltip} className="opacity-70" /> : null}
      <div className="ml-auto shrink-0">{children}</div>
    </div>
  )
}
