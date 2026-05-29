/**
 * 网络状态指示器组件
 * 显示当前网络状态（在线/离线/弱网）
 */

import { AlertTriangle, WifiOff } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Collapse } from '@/components/tailwind'
import {
  networkMonitor,
  type NetworkQuality,
  type NetworkStatus,
} from '@/services/network-monitor'

export const NetworkStatusIndicator = () => {
  const { t } = useTranslation()
  const [status, setStatus] = useState<NetworkStatus>(
    networkMonitor.getStatus(),
  )

  useEffect(() => {
    return networkMonitor.subscribe((newStatus) => {
      setStatus(newStatus)
    })
  }, [])

  // 只在离线或弱网时显示
  const showIndicator = status.quality !== 'good'

  return (
    <Collapse open={showIndicator}>
      {status.quality === 'offline' ? (
        <div className="mb-2 flex items-center gap-2 rounded-lg bg-red-50 p-3 text-red-800 dark:bg-red-900/20 dark:text-red-200">
          <WifiOff className="h-5 w-5" />
          <span>网络已断开，部分功能不可用</span>
        </div>
      ) : status.quality === 'poor' ? (
        <div className="mb-2 flex items-center gap-2 rounded-lg bg-yellow-50 p-3 text-yellow-800 dark:bg-yellow-900/20 dark:text-yellow-200">
          <AlertTriangle className="h-5 w-5" />
          <span>网络较慢，请耐心等待</span>
        </div>
      ) : null}
    </Collapse>
  )
}

/**
 * 简单的网络质量徽章
 */
export const NetworkQualityBadge = () => {
  const [quality, setQuality] = useState<NetworkQuality>(
    networkMonitor.getQuality(),
  )

  useEffect(() => {
    return networkMonitor.subscribe((status) => {
      setQuality(status.quality)
    })
  }, [])

  const colorMap = {
    good: 'bg-green-500',
    poor: 'bg-yellow-500',
    offline: 'bg-red-500',
  }

  const labelMap = {
    good: '网络良好',
    poor: '网络较慢',
    offline: '离线',
  }

  return (
    <span className="inline-flex items-center gap-1 text-xs">
      <span className={`h-2 w-2 rounded-full ${colorMap[quality]}`} />
      {labelMap[quality]}
    </span>
  )
}
