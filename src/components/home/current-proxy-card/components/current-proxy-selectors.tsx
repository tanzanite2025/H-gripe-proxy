import type { SelectChangeEvent } from '@/components/tailwind/Select'
import { Chip } from '@/components/tailwind/Chip'
import {
  MenuItem,
  Select,
  type SelectPrimitiveValue,
} from '@/components/tailwind/Select'
import delayManager from '@/services/delay'

import type {
  ProxyGroupOption,
  ProxyOption,
} from '../hooks/current-proxy-data/shared'
import { convertDelayColor } from '../utils/delay-visuals'

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

const getProxyOptionPrefix = (kind: ProxyOption['kind']) =>
  kind === 'strategy' ? '[Strategy]' : ''

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
      <div className="min-w-0 flex-1">
        <Select
          value={selectedGroup}
          onChange={onGroupChange}
          disabled={isGlobalMode}
          size="small"
          className="h-[38px] rounded-2xl border border-solid border-gray-200 bg-gray-50/20 dark:border-gray-700 dark:bg-gray-800/20 [&_select]:border-0 [&_select]:bg-transparent"
        >
          {groups.map((group) => (
            <MenuItem key={group.name} value={group.name}>
              {group.name}
            </MenuItem>
          ))}
        </Select>
      </div>

      <div className="min-w-0 flex-1">
        <Select
          value={selectedProxy}
          onChange={onProxyChange}
          size="small"
          renderValue={(selected: SelectPrimitiveValue) => (
            <div className="truncate">{String(selected)}</div>
          )}
          className="h-[38px] rounded-2xl border border-solid border-gray-200 bg-gray-50/20 dark:border-gray-700 dark:bg-gray-800/20 [&_select]:border-0 [&_select]:bg-transparent"
          MenuProps={{
            slotProps: {
              paper: {
                style: {
                  maxHeight: 500,
                },
              },
            },
          }}
        >
          {proxyOptions.map((proxy) => {
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
                  label={delayManager.formatDelay(
                    delayValue,
                    defaultLatencyTimeout,
                  )}
                  color={convertDelayColor(
                    delayValue,
                    defaultLatencyTimeout,
                  )}
                  className="h-[22px] min-w-[60px] shrink-0"
                />
              </MenuItem>
            )
          })}
        </Select>
      </div>
    </>
  )
}
