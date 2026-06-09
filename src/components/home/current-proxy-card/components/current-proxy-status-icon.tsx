import { WifiOff as SignalNone } from 'lucide-react'

import { Tooltip } from '@/components/tailwind/Tooltip'
import delayManager from '@/services/delay'

import type { DelaySignalVisual } from '../utils/delay-visuals'

interface CurrentProxyStatusIconProps {
  currentDelay: number
  currentProxy: any
  noActiveNodeLabel: string
  refreshDelayLabel: string
  signalVisual: DelaySignalVisual | null
  timeout: number
}

export function CurrentProxyStatusIcon({
  currentDelay,
  currentProxy,
  noActiveNodeLabel,
  refreshDelayLabel,
  signalVisual,
  timeout,
}: CurrentProxyStatusIconProps) {
  return (
    <Tooltip
      title={
        currentProxy
          ? `${refreshDelayLabel}: ${delayManager.formatDelay(currentDelay, timeout)}`
          : noActiveNodeLabel
      }
    >
      <div style={{ color: signalVisual?.color }}>
        {currentProxy ? (
          signalVisual?.icon
        ) : (
          <SignalNone className="h-5 w-5 text-gray-400" />
        )}
      </div>
    </Tooltip>
  )
}
