import { Chip } from '@/components/tailwind/Chip'
import { MenuItem } from '@/components/tailwind/Select'
import delayManager from '@/services/delay'

import type { ProxyOption } from '../hooks/current-proxy-data/shared'
import { convertDelayColor } from '../utils/delay-visuals'

interface CurrentProxyOptionItemProps {
  defaultLatencyTimeout: number
  proxy: ProxyOption
  records: Record<string, any>
  selectedGroup: string
}

const getProxyOptionPrefix = (kind: ProxyOption['kind']) =>
  kind === 'strategy' ? '[Strategy]' : ''

export function CurrentProxyOptionItem({
  defaultLatencyTimeout,
  proxy,
  records,
  selectedGroup,
}: CurrentProxyOptionItemProps) {
  const delayValue =
    records[proxy.name] && selectedGroup
      ? delayManager.getDelayFix(records[proxy.name], selectedGroup)
      : -1
  const prefix = getProxyOptionPrefix(proxy.kind)

  return (
    <MenuItem
      key={proxy.name}
      value={proxy.name}
      className="flex w-full items-center justify-between pr-1"
    >
      <div className="mr-1 flex-1 truncate">
        {prefix ? `${prefix} ` : ''}
        {proxy.name}
      </div>
      <Chip
        size="small"
        label={delayManager.formatDelay(delayValue, defaultLatencyTimeout)}
        color={convertDelayColor(delayValue, defaultLatencyTimeout)}
        className="h-[22px] min-w-[60px] shrink-0"
      />
    </MenuItem>
  )
}
