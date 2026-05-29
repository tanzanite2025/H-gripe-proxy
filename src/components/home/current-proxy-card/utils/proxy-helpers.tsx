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

/**
 * 将延迟值转换为 MUI 颜色
 */
export function convertDelayColor(
  delayValue: number,
): 'success' | 'warning' | 'error' | 'primary' | 'default' {
  const colorStr = delayManager.formatDelayColor(delayValue)
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

/**
 * 根据延迟值获取信号图标和描述
 */
export function getSignalIcon(delay: number): {
  icon: ReactElement
  text: string
  color: string
} {
  if (delay === -2)
    return { icon: <SignalZero className="h-5 w-5" />, text: '测试中', color: '#9ca3af' }
  if (delay === -1)
    return { icon: <SignalZero className="h-5 w-5" />, text: '未测试', color: '#9ca3af' }
  if (delay > 1e5)
    return { icon: <SignalError className="h-5 w-5" />, text: '错误', color: '#dc2626' }
  if (delay === 0 || delay >= 10000)
    return { icon: <SignalError className="h-5 w-5" />, text: '超时', color: '#dc2626' }
  if (delay >= 500)
    return { icon: <SignalLow className="h-5 w-5" />, text: '延迟较高', color: '#dc2626' }
  if (delay >= 300)
    return { icon: <SignalMedium className="h-5 w-5" />, text: '延迟中等', color: '#d97706' }
  if (delay >= 200)
    return { icon: <SignalHigh className="h-5 w-5" />, text: '延迟良好', color: '#2563eb' }
  return { icon: <Signal className="h-5 w-5" />, text: '延迟极佳', color: '#16a34a' }
}

/**
 * 规范化策略名称
 */
export function normalizePolicyName(value?: string | null): string {
  return typeof value === 'string' ? value.trim() : ''
}

/**
 * 延迟分类（用于排序）
 */
export function categorizeDelay(
  delay: number,
  effectiveTimeout: number,
): [number, number] {
  if (!Number.isFinite(delay)) return [5, Number.MAX_SAFE_INTEGER]
  if (delay > 1e5) return [4, delay]
  if (delay === 0 || (delay >= effectiveTimeout && delay <= 1e5)) {
    return [3, delay || effectiveTimeout]
  }
  if (delay < 0) return [5, Number.MAX_SAFE_INTEGER]
  return [0, delay]
}
