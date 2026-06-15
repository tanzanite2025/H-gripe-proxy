import type { SelectChangeEvent } from '@/components/tailwind/Select'
import { Select, type SelectPrimitiveValue } from '@/components/tailwind/Select'

import type { ProxyOption } from '../hooks/current-proxy-data/shared'

import { CurrentProxyOptionItem } from './current-proxy-option-item'
import { CURRENT_PROXY_SELECT_CLASSNAME } from './current-proxy-select-styles'

interface CurrentProxyProxySelectProps {
  defaultLatencyTimeout: number
  onChange: (event: SelectChangeEvent<string>) => void
  proxyOptions: ProxyOption[]
  records: Record<string, any>
  selectedGroup: string
  selectedProxy: string
}

const PROXY_SELECT_MENU_PROPS = {
  slotProps: {
    paper: {
      style: {
        maxHeight: 500,
      },
    },
  },
} as const

export function CurrentProxyProxySelect({
  defaultLatencyTimeout,
  onChange,
  proxyOptions,
  records,
  selectedGroup,
  selectedProxy,
}: CurrentProxyProxySelectProps) {
  return (
    <div className="min-w-0 flex-1">
      <Select
        value={selectedProxy}
        onChange={onChange}
        size="small"
        renderValue={(selected: SelectPrimitiveValue) => (
          <div className="truncate">{String(selected)}</div>
        )}
        className={CURRENT_PROXY_SELECT_CLASSNAME}
        MenuProps={PROXY_SELECT_MENU_PROPS}
      >
        {proxyOptions.map((proxy) => (
          <CurrentProxyOptionItem
            key={proxy.name}
            defaultLatencyTimeout={defaultLatencyTimeout}
            proxy={proxy}
            records={records}
            selectedGroup={selectedGroup}
          />
        ))}
      </Select>
    </div>
  )
}
