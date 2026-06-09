import { CheckCircle2 } from 'lucide-react'

import { ListItemButton } from '@/components/tailwind/ListItemButton'
import { cn } from '@/utils/cn'

import { ProxyCardActions } from './proxy-item/proxy-card-actions'
import { ProxyCardContent } from './proxy-item/proxy-card-content'
import { ProxyCardFixedPin } from './proxy-item/proxy-card-fixed-pin'
import type { ProxyCardProps } from './proxy-item/types'
import { useProxyCardState } from './proxy-item/use-proxy-card-state'

export const ProxyItemMini = (props: ProxyCardProps) => {
  const {
    group,
    proxy,
    selected,
    showType = true,
    clickable = true,
    onClick,
    onConfigure,
  } = props
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

  return (
    <ListItemButton
      selected={selected}
      disabled={!clickable}
      onClick={clickable ? () => onClick?.(proxy.name) : undefined}
      className={cn(
        'group relative h-14 items-center justify-between rounded-xl bg-white pl-3 pr-2 dark:bg-[#24252f]',
        selected &&
          '-ml-[3px] w-[calc(100%+3px)] border-l-[3px] border-primary bg-primary/15 dark:bg-primary/35',
        !clickable &&
          'hover:bg-white active:bg-white dark:hover:bg-[#24252f] dark:active:bg-[#24252f]',
      )}
    >
      <ProxyCardContent proxy={proxy} showType={showType} variant="compact" />

      <ProxyCardActions
        configurableStrategyGroup={configurableStrategyGroup}
        delayValue={delayValue}
        isPreset={isPreset}
        onConfigure={onConfigure}
        onDelay={onDelay}
        proxy={proxy}
        selected={selected}
        selectedIcon={CheckCircle2}
        showSelectedIcon={selected && proxy.type !== 'Direct'}
        timeout={timeout}
        variant="compact"
      />

      <ProxyCardFixedPin group={group} proxy={proxy} />
    </ListItemButton>
  )
}
