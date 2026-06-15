import { MenuItem, Select } from '@/components/tailwind/Select'

import type { ProxyGroupOption } from '../hooks/current-proxy-data/shared'

import { CURRENT_PROXY_SELECT_CLASSNAME } from './current-proxy-select-styles'

interface CurrentProxyGroupSelectProps {
  groups: ProxyGroupOption[]
  onChange: (event: { target: { value: string } }) => void
  selectedGroup: string
}

export function CurrentProxyGroupSelect({
  groups,
  onChange,
  selectedGroup,
}: CurrentProxyGroupSelectProps) {
  return (
    <div className="min-w-0 flex-1">
      <Select
        value={selectedGroup}
        onChange={onChange}
        size="small"
        className={CURRENT_PROXY_SELECT_CLASSNAME}
      >
        {groups.map((group) => (
          <MenuItem key={group.name} value={group.name}>
            {group.name}
          </MenuItem>
        ))}
      </Select>
    </div>
  )
}
