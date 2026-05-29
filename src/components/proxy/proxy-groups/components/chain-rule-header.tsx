import { ChevronDown } from 'lucide-react'
import type { MouseEvent } from 'react'

import { Chip, IconButton } from '@/components/tailwind'

interface ProxyGroupOption {
  name: string
  type: string
  all?: unknown[]
}

interface ChainRuleHeaderProps {
  title: string
  selectLabel: string
  currentGroup: ProxyGroupOption | null
  canSelectGroup: boolean
  onMenuOpen: (event: MouseEvent<HTMLElement>) => void
}

/**
 * 链式代理模式下的规则头部组件
 */
export function ChainRuleHeader({
  title,
  selectLabel,
  currentGroup,
  canSelectGroup,
  onMenuOpen,
}: ChainRuleHeaderProps) {
  return (
    <div className="border-b border-gray-200 dark:border-gray-700">
      <div className="flex items-center justify-between border-b border-gray-200 px-4 py-3 dark:border-gray-700">
        <div className="flex items-center gap-4">
          <h6 className="text-base font-semibold">{title}</h6>

          {currentGroup && (
            <Chip
              size="small"
              label={`${currentGroup.name} (${currentGroup.type})`}
              variant="outlined"
              className="max-w-[200px] overflow-hidden text-ellipsis whitespace-nowrap text-xs"
            />
          )}
        </div>

        {canSelectGroup && (
          <IconButton
            size="small"
            onClick={onMenuOpen}
            className="rounded border border-gray-200 px-2 py-1 dark:border-gray-700"
          >
            <span className="mr-1 text-xs">{selectLabel}</span>
            <ChevronDown className="h-4 w-4" />
          </IconButton>
        )}
      </div>
    </div>
  )
}
