import type { SelectChangeEvent } from '@/components/tailwind/Select'

import type {
  ProxyGroupOption,
  ProxyOption,
} from '../hooks/current-proxy-data/shared'
import { CurrentProxyGroupSelect } from './current-proxy-group-select'
import { CurrentProxyProxySelect } from './current-proxy-proxy-select'

interface CurrentProxySelectorsProps {
  defaultLatencyTimeout: number
  groups: ProxyGroupOption[]
  isGlobalMode: boolean
  onGroupChange: (event: { target: { value: string } }) => void
  onProxyChange: (event: SelectChangeEvent<string>) => void
  proxyOptions: ProxyOption[]
  records: Record<string, any>
  selectedGroup: string
  selectedProxy: string
}

export function CurrentProxySelectors({
  defaultLatencyTimeout,
  groups,
  isGlobalMode,
  onGroupChange,
  onProxyChange,
  proxyOptions,
  records,
  selectedGroup,
  selectedProxy,
}: CurrentProxySelectorsProps) {
  return (
    <>
      <CurrentProxyGroupSelect
        groups={groups}
        isGlobalMode={isGlobalMode}
        onChange={onGroupChange}
        selectedGroup={selectedGroup}
      />

      <CurrentProxyProxySelect
        defaultLatencyTimeout={defaultLatencyTimeout}
        onChange={onProxyChange}
        proxyOptions={proxyOptions}
        records={records}
        selectedGroup={selectedGroup}
        selectedProxy={selectedProxy}
      />
    </>
  )
}
