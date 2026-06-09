import {
  Clock as AccessTimeRounded,
  ChevronRight,
  Activity as NetworkCheckRounded,
  SortAsc as SortByAlphaRounded,
  ArrowUpDown as SortRounded,
} from 'lucide-react'
import type { ReactElement } from 'react'

import { Button } from '@/components/tailwind/Button'
import { IconButton } from '@/components/tailwind/IconButton'
import { Tooltip } from '@/components/tailwind/Tooltip'

import type { ProxySortType } from '../hooks/current-proxy-data/shared'

interface CurrentProxyCardActionsProps {
  onCheckAllDelay: () => void
  onOpenProxies: () => void
  onSortTypeChange: () => void
  proxiesLabel: string
  refreshDelayLabel: string
  sortTooltip: string
  sortType: ProxySortType
}

const SORT_ICONS: Record<ProxySortType, ReactElement> = {
  0: <SortRounded className="h-4 w-4" />,
  1: <AccessTimeRounded className="h-4 w-4" />,
  2: <SortByAlphaRounded className="h-4 w-4" />,
}

export function CurrentProxyCardActions({
  onCheckAllDelay,
  onOpenProxies,
  onSortTypeChange,
  proxiesLabel,
  refreshDelayLabel,
  sortTooltip,
  sortType,
}: CurrentProxyCardActionsProps) {
  return (
    <div className="flex items-center gap-1">
      <Tooltip title={refreshDelayLabel}>
        <span>
          <IconButton size="small" color="inherit" onClick={onCheckAllDelay}>
            <NetworkCheckRounded className="h-5 w-5" />
          </IconButton>
        </span>
      </Tooltip>

      <Tooltip title={sortTooltip}>
        <IconButton size="small" color="inherit" onClick={onSortTypeChange}>
          {SORT_ICONS[sortType]}
        </IconButton>
      </Tooltip>

      <Button
        variant="outlined"
        size="small"
        onClick={onOpenProxies}
        className="rounded-xl"
        endIcon={<ChevronRight className="h-4 w-4" />}
      >
        {proxiesLabel}
      </Button>
    </div>
  )
}
