/**
 * 网络状态指示器组件
 * 显示当前网络状态（在线/离线/弱网）
 */

import { Alert, Collapse } from '@mui/material'
import { useEffect, useState } from 'react'
import { useTranslation } from 'react-i18next'

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
    <Collapse in={showIndicator}>
      {status.quality === 'offline' ? (
        <Alert severity="error" sx={{ mb: 2 }}>
          网络已断开，部分功能不可用
        </Alert>
      ) : status.quality === 'poor' ? (
        <Alert severity="warning" sx={{ mb: 2 }}>
          网络较慢，请耐心等待
        </Alert>
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
    good: 'success.main',
    poor: 'warning.main',
    offline: 'error.main',
  }

  const labelMap = {
    good: '网络良好',
    poor: '网络较慢',
    offline: '离线',
  }

  return (
    <span
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: '4px',
        fontSize: '12px',
      }}
    >
      <span
        style={{
          width: '8px',
          height: '8px',
          borderRadius: '50%',
          backgroundColor: colorMap[quality],
        }}
      />
      {labelMap[quality]}
    </span>
  )
}
