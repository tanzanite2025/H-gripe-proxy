import {
  Signal,
  SignalHigh,
  SignalLow,
  SignalMedium,
  SignalZero,
  WifiOff as SignalError,
} from 'lucide-react'
import type { ReactElement } from 'react'

import delayManager from '@/services/delay'
import { resolveDelayTimeout } from '@/services/delay-config'

const ERROR_DELAY_THRESHOLD = 1e5

export interface DelaySignalVisual {
  color: string
  icon: ReactElement
}

export function convertDelayColor(
  delayValue: number,
  timeout?: number,
): 'success' | 'warning' | 'error' | 'primary' | 'default' {
  const colorStr = delayManager.formatDelayColor(
    delayValue,
    resolveDelayTimeout(timeout),
  )
  if (!colorStr) return 'default'

  const mainColor = colorStr.split('.')[0]

  switch (mainColor) {
    case 'success':
      return 'success'
    case 'warning':
      return 'warning'
    case 'error':
      return 'error'
    case 'primary':
      return 'primary'
    default:
      return 'default'
  }
}

export function getDelaySignalVisual(
  delay: number,
  timeout?: number,
): DelaySignalVisual {
  const effectiveTimeout = resolveDelayTimeout(timeout)

  if (delay < 0) {
    return {
      color: '#9ca3af',
      icon: <SignalZero className="h-5 w-5" />,
    }
  }

  if (
    delay > ERROR_DELAY_THRESHOLD ||
    delay === 0 ||
    delay >= effectiveTimeout
  ) {
    return {
      color: '#dc2626',
      icon: <SignalError className="h-5 w-5" />,
    }
  }

  if (delay >= 500) {
    return {
      color: '#dc2626',
      icon: <SignalLow className="h-5 w-5" />,
    }
  }

  if (delay >= 300) {
    return {
      color: '#d97706',
      icon: <SignalMedium className="h-5 w-5" />,
    }
  }

  if (delay >= 200) {
    return {
      color: '#2563eb',
      icon: <SignalHigh className="h-5 w-5" />,
    }
  }

  return {
    color: '#16a34a',
    icon: <Signal className="h-5 w-5" />,
  }
}
