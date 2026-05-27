import { cn } from '@/utils/cn'

import type { TooltipData } from '../hooks/use-graph-interaction'

interface GraphTooltipProps {
  tooltipData: TooltipData
}

/**
 * 图表悬浮提示框组件
 */
export const GraphTooltip = ({ tooltipData }: GraphTooltipProps) => {
  if (!tooltipData.visible) return null

  return (
    <div
      className={cn(
        'pointer-events-none absolute z-[1000] whitespace-nowrap rounded border border-gray-200 bg-white px-1 py-0.5 text-[10px] leading-tight opacity-100 shadow-lg dark:border-gray-700 dark:bg-gray-800',
      )}
      style={{
        left: tooltipData.x + 8,
        top: tooltipData.y - 8,
        transform: tooltipData.x > 200 ? 'translateX(-100%)' : 'translateX(0)',
      }}
    >
      <div className="mb-0.5 text-gray-500 dark:text-gray-400">
        {tooltipData.timestamp}
      </div>
      <div className="font-medium text-secondary-500">
        ↑ {tooltipData.upSpeed}
      </div>
      <div className="font-medium text-primary-500">
        ↓ {tooltipData.downSpeed}
      </div>
    </div>
  )
}
