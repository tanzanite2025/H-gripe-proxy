import type { ComponentType } from 'react'

import { BaseLoading } from '@/components/base'
import delayManager from '@/services/delay'
import type { IProxyItem } from '@/types/proxy'
import { cn } from '@/utils/cn'

interface ProxyCardDelayActionProps {
  checkClassName: string
  delayClassName: string
  delayValue: number
  loadingClassName: string
  onDelay: () => void
  proxy: IProxyItem
  selectedIcon: ComponentType<{ className?: string }>
  selectedIconClassName: string
  showSelectedIcon: boolean
  timeout: number
}

export function ProxyCardDelayAction({
  checkClassName,
  delayClassName,
  delayValue,
  loadingClassName,
  onDelay,
  proxy,
  selectedIcon: SelectedIcon,
  selectedIconClassName,
  showSelectedIcon,
  timeout,
}: ProxyCardDelayActionProps) {
  if (delayValue === -2) {
    return (
      <div className={loadingClassName}>
        <BaseLoading />
      </div>
    )
  }

  if (!proxy.provider && delayValue !== -2) {
    return (
      <>
        <div
          className={checkClassName}
          onClick={(event) => {
            event.preventDefault()
            event.stopPropagation()
            onDelay()
          }}
        >
          Check
        </div>

        {delayValue >= 0 && (
          <div
            className={cn(delayClassName, 'cursor-pointer hover:bg-primary/15')}
            style={{ color: delayManager.formatDelayColor(delayValue, timeout) }}
            onClick={(event) => {
              event.preventDefault()
              event.stopPropagation()
              onDelay()
            }}
          >
            {delayManager.formatDelay(delayValue, timeout)}
          </div>
        )}
      </>
    )
  }

  if (delayValue >= 0) {
    return (
      <div
        className={delayClassName}
        style={{ color: delayManager.formatDelayColor(delayValue, timeout) }}
      >
        {delayManager.formatDelay(delayValue, timeout)}
      </div>
    )
  }

  if (showSelectedIcon) {
    return <SelectedIcon className={selectedIconClassName} />
  }

  return null
}
