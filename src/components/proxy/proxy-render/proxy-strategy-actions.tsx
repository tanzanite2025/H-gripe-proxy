import { SlidersHorizontal } from 'lucide-react'

import {
  Button,
  IconButton,
  Tooltip,
} from '@/components/tailwind'

interface ProxyStrategyActionsProps {
  group: IProxyGroupItem
  onConfigureStrategyGroup: (group: IProxyGroupItem) => void
}

export function ProxyStrategyActions({
  group,
  onConfigureStrategyGroup,
}: ProxyStrategyActionsProps) {
  const handleConfigure = (event?: {
    preventDefault?: () => void
    stopPropagation?: () => void
  }) => {
    event?.preventDefault?.()
    event?.stopPropagation?.()
    onConfigureStrategyGroup(group)
  }

  return (
    <div className="flex items-center gap-2">
      <Tooltip title="配置策略池成员" arrow>
        <IconButton
          size="small"
          color="primary"
          className="h-7 w-7"
          onClick={handleConfigure}
          onKeyDown={(event) => {
            event.stopPropagation()
          }}
        >
          <SlidersHorizontal className="h-4 w-4" />
        </IconButton>
      </Tooltip>
      <Button
        type="button"
        variant="outlined"
        size="small"
        color="warning"
        className="px-2 py-1 text-[10px]"
        onClick={handleConfigure}
        onKeyDown={(event) => {
          event.stopPropagation()
        }}
      >
        配置成员
      </Button>
    </div>
  )
}
