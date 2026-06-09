import { SlidersHorizontal } from 'lucide-react'

import { Tooltip } from '@/components/tailwind/Tooltip'

interface ProxyCardConfigureActionProps {
  className: string
  configurableStrategyGroup?: IProxyGroupItem | null
  onConfigure?: (group: IProxyGroupItem) => void
}

export function ProxyCardConfigureAction({
  className,
  configurableStrategyGroup,
  onConfigure,
}: ProxyCardConfigureActionProps) {
  if (!configurableStrategyGroup) {
    return null
  }

  return (
    <Tooltip title="配置策略池成员" arrow placement="top">
      <div
        className={className}
        onClick={(event) => {
          event.preventDefault()
          event.stopPropagation()
          onConfigure?.(configurableStrategyGroup)
        }}
      >
        <SlidersHorizontal className="h-3.5 w-3.5" />
      </div>
    </Tooltip>
  )
}
