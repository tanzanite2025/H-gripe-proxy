import { useTranslation } from 'react-i18next'

import { Chip } from '@/components/tailwind/Chip'
import {
  FormControl,
  MenuItem,
  Select,
  type SelectChangeEvent,
  type SelectPrimitiveValue,
} from '@/components/tailwind/Select'
import delayManager from '@/services/delay'
import { cn } from '@/utils/cn'

import type { ProxyState } from '../hooks/use-current-proxy-data'
import { convertDelayColor } from '../utils/proxy-helpers'

interface ProxySelectorsProps {
  state: ProxyState
  proxyOptions: Array<{ name: string }>
  isGlobalMode: boolean
  isDirectMode: boolean
  onGroupChange: (event: SelectChangeEvent<string>) => void
  onProxyChange: (event: SelectChangeEvent<string>) => void
}

/**
 * 代理选择器组件
 * 包含代理组选择器和代理节点选择器
 */
export const ProxySelectors = ({
  state,
  proxyOptions,
  isGlobalMode,
  isDirectMode,
  onGroupChange,
  onProxyChange,
}: ProxySelectorsProps) => {
  const { t } = useTranslation()

  // 自定义渲染选择框中的值
  const renderProxyValue = (selected: SelectPrimitiveValue) => {
    const selectedValue = String(selected)
    return <div className="truncate">{selectedValue}</div>
  }

  const selectClassName = cn(
    'rounded-2xl border-solid bg-card/20',
    'border-divider',
    'border-primary',
  )

  return (
    <>
      {/* 代理组选择器 */}
      <FormControl fullWidth variant="outlined" size="small" className="mb-1.5">
        <Select
          value={state.selection.group}
          onChange={onGroupChange}
          label={t('home.components.currentProxy.labels.group')}
          disabled={isGlobalMode || isDirectMode}
          className={selectClassName}
        >
          {state.proxyData.groups.map((group) => (
            <MenuItem key={group.name} value={group.name}>
              {group.name}
            </MenuItem>
          ))}
        </Select>
      </FormControl>

      {/* 代理节点选择器 */}
      <FormControl fullWidth variant="outlined" size="small" className="mb-0">
        <Select
          value={state.selection.proxy}
          onChange={onProxyChange}
          label={t('home.components.currentProxy.labels.proxy')}
          disabled={isDirectMode}
          renderValue={renderProxyValue}
          className={selectClassName}
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
          {isDirectMode
            ? null
            : proxyOptions.map((proxy) => {
                const delayValue =
                  state.proxyData.records[proxy.name] && state.selection.group
                    ? delayManager.getDelayFix(
                        state.proxyData.records[proxy.name],
                        state.selection.group,
                      )
                    : -1
                return (
                  <MenuItem
                    key={proxy.name}
                    value={proxy.name}
                    className="flex w-full items-center justify-between pr-1"
                  >
                    <div className="mr-1 flex-1 truncate">
                      {proxy.name}
                    </div>
                    <Chip
                      size="small"
                      label={delayManager.formatDelay(delayValue)}
                      color={convertDelayColor(delayValue)}
                      className="h-[22px] min-w-[60px] flex-shrink-0"
                    />
                  </MenuItem>
                )
              })}
        </Select>
      </FormControl>
    </>
  )
}
