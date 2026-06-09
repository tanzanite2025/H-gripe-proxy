import { CheckCircle } from 'lucide-react'

import { ListItem, ListItemIcon } from '@/components/tailwind/List'
import { ListItemText } from '@/components/tailwind/ListItemText'

import { ProxyCardActions } from './proxy-item/proxy-card-actions'
import { ProxyCardContent } from './proxy-item/proxy-card-content'
import type { ProxyCardProps } from './proxy-item/types'
import { useProxyCardState } from './proxy-item/use-proxy-card-state'

export const ProxyItem = (props: ProxyCardProps) => {
  const {
    group,
    proxy,
    selected,
    showType = true,
    clickable = true,
    onClick,
    onConfigure,
  } = props
  const isDark = true

  const {
    configurableStrategyGroup,
    delayValue,
    isPreset,
    onDelay,
    timeout,
  } = useProxyCardState({
    group,
    proxy,
    onConfigure,
  })

  const bgcolor = isDark ? '#24252f' : '#ffffff'
  const selectColor = isDark ? '#90caf9' : '#1976d2'

  return (
    <ListItem className="py-0 pl-2">
      <div
        role={clickable ? 'button' : undefined}
        tabIndex={clickable ? 0 : -1}
        className={`group mb-2 h-10 rounded ${
          selected
            ? 'ml-[-3px] w-[calc(100%+3px)] border-l-[3px]'
            : ''
        } ${clickable ? 'cursor-pointer' : 'cursor-default'}`}
        style={{
          backgroundColor: bgcolor,
          ...(selected
            ? {
                borderLeftColor: selectColor,
                backgroundColor: isDark
                  ? 'rgba(25, 118, 210, 0.35)'
                  : 'rgba(25, 118, 210, 0.15)',
              }
            : {}),
        }}
        onClick={clickable ? () => onClick?.(proxy.name) : undefined}
        onKeyDown={
          clickable
            ? (event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  onClick?.(proxy.name)
                }
              }
            : undefined
        }
      >
        <ListItemText
          title={proxy.name}
          secondary={
            <ProxyCardContent
              proxy={proxy}
              showType={showType}
              variant="default"
            />
          }
        />

        <ListItemIcon className={`justify-end text-primary ${isPreset ? 'hidden' : ''}`}>
          <ProxyCardActions
            configurableStrategyGroup={configurableStrategyGroup}
            delayValue={delayValue}
            isPreset={isPreset}
            onConfigure={onConfigure}
            onDelay={onDelay}
            proxy={proxy}
            selected={selected}
            selectedIcon={CheckCircle}
            showSelectedIcon={selected}
            timeout={timeout}
            variant="default"
          />
        </ListItemIcon>
      </div>
    </ListItem>
  )
}
