import type { ComponentType } from 'react'

import type { IProxyGroupItem, IProxyItem } from '@/types/proxy'
import { cn } from '@/utils/cn'

import { ACTION_VARIANT_CLASS, type ProxyCardActionVariant } from './proxy-card-action-types'
import { ProxyCardConfigureAction } from './proxy-card-configure-action'
import { ProxyCardDelayAction } from './proxy-card-delay-action'

interface ProxyCardActionsProps {
  configurableStrategyGroup?: IProxyGroupItem | null
  delayValue: number
  isPreset: boolean
  onConfigure?: (group: IProxyGroupItem) => void
  onDelay: () => void
  proxy: IProxyItem
  selected: boolean
  selectedIcon: ComponentType<{ className?: string }>
  showSelectedIcon: boolean
  timeout: number
  variant: ProxyCardActionVariant
}

export function ProxyCardActions({
  configurableStrategyGroup,
  delayValue,
  isPreset,
  onConfigure,
  onDelay,
  proxy,
  selectedIcon: SelectedIcon,
  showSelectedIcon,
  timeout,
  variant,
}: ProxyCardActionsProps) {
  const classes = ACTION_VARIANT_CLASS[variant]

  return (
    <div className={cn(classes.wrapper, isPreset && 'hidden')}>
      <ProxyCardConfigureAction
        className={classes.configure}
        configurableStrategyGroup={configurableStrategyGroup}
        onConfigure={onConfigure}
      />

      <ProxyCardDelayAction
        checkClassName={classes.check}
        delayClassName={classes.delay}
        delayValue={delayValue}
        loadingClassName={classes.loading}
        onDelay={onDelay}
        proxy={proxy}
        selectedIcon={SelectedIcon}
        selectedIconClassName={classes.selectedIcon}
        showSelectedIcon={showSelectedIcon}
        timeout={timeout}
      />
    </div>
  )
}
