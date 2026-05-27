/**
 * WebRTC 泄漏检测卡片组件
 * 显示 WebRTC 是否泄漏真实 IP
 */

import {
  ErrorOutlined,
  RefreshOutlined,
  VpnLockOutlined,
} from '@mui/icons-material'
import { useQuery } from '@tanstack/react-query'
import { forwardRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import { IconButton } from '@/components/tailwind/IconButton'
import { Skeleton } from '@/components/tailwind/Skeleton'
import {
  detectWebRTCLeak,
  getWebRTCLeakRiskDescription,
  isWebRTCSupported,
  type WebRTCLeakResult,
} from '@/services/webrtc-leak-detection'

import { EnhancedCard } from './enhanced-card'

const WebRTCLeakCardContainer = forwardRef<
  HTMLElement,
  React.PropsWithChildren
>(({ children }, ref) => {
  const { t } = useTranslation()
  const { refetch } = useWebRTCLeakDetection()

  return (
    <EnhancedCard
      title="WebRTC 泄漏检测"
      icon={<VpnLockOutlined />}
      iconColor="info"
      ref={ref}
      action={
        <IconButton size="small" onClick={() => refetch()}>
          <RefreshOutlined />
        </IconButton>
      }
    >
      {children}
    </EnhancedCard>
  )
})

WebRTCLeakCardContainer.displayName = 'WebRTCLeakCardContainer'

export const WebRTCLeakCard = () => {
  const { data, error, isLoading, refetch } = useWebRTCLeakDetection()
  const [showDetails, setShowDetails] = useState(false)

  // 检查浏览器支持
  if (!isWebRTCSupported()) {
    return (
      <WebRTCLeakCardContainer>
        <div className="flex flex-col items-center justify-center py-6 text-warning">
          <ErrorOutlined className="text-4xl mb-2" />
          <p className="text-base text-warning">
            您的浏览器不支持 WebRTC
          </p>
          <p className="text-xs text-text-secondary mt-2">
            无法进行 WebRTC 泄漏检测
          </p>
        </div>
      </WebRTCLeakCardContainer>
    )
  }

  return (
    <WebRTCLeakCardContainer>
      <WebRTCLeakCardUI
        result={data}
        error={error}
        isLoading={isLoading}
        showDetails={showDetails}
        onToggleDetails={() => setShowDetails(!showDetails)}
        onRetry={() => refetch()}
      />
    </WebRTCLeakCardContainer>
  )
}

interface WebRTCLeakCardUIProps {
  result?: WebRTCLeakResult
  error?: Error | null
  isLoading: boolean
  showDetails: boolean
  onToggleDetails: () => void
  onRetry: () => void
}

const WebRTCLeakCardUI = ({
  result,
  error,
  isLoading,
  showDetails,
  onToggleDetails,
  onRetry,
}: WebRTCLeakCardUIProps) => {
  if (isLoading) {
    return (
      <div className="flex flex-col gap-2">
        <Skeleton variant="text" width="60%" height={30} />
        <Skeleton variant="text" width="80%" height={24} />
        <Skeleton variant="text" width="70%" height={24} />
      </div>
    )
  }

  if (error || result?.error) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-error">
        <ErrorOutlined className="text-4xl mb-2" />
        <p className="text-base text-error">
          {error instanceof Error ? error.message : result?.error || '检测失败'}
        </p>
        <Button onClick={onRetry} className="mt-4">
          重试
        </Button>
      </div>
    )
  }

  if (!result) {
    return null
  }

  const riskInfo = getWebRTCLeakRiskDescription(result.riskLevel)

  return (
    <div className="flex flex-col gap-3">
      {/* 风险状态 */}
      <div className="flex items-start gap-2">
        <VpnLockOutlined className={`text-2xl ${riskInfo.color}`} />
        <div className="flex-1">
          <p className={`text-base font-medium ${riskInfo.color}`}>
            {riskInfo.title}
          </p>
          <p className="text-xs text-text-secondary mt-0.5">
            {riskInfo.description}
          </p>
        </div>
      </div>

      {/* IP 信息 */}
      <div className="grid grid-cols-2 gap-2 text-sm">
        {/* 代理 IP */}
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">代理 IP</p>
          <p className="uds-mono text-xs font-medium">{result.proxyIP}</p>
        </div>

        {/* 泄漏状态 */}
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">泄漏状态</p>
          <p className={`text-xs font-medium ${riskInfo.color}`}>
            {result.isLeaking ? '已泄漏' : '未泄漏'}
          </p>
        </div>
      </div>

      {/* 检测到的 IP */}
      {showDetails && (result.localIPs.length > 0 || result.publicIPs.length > 0) && (
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs font-medium mb-2">检测到的 IP：</p>
          
          {/* 本地 IP */}
          {result.localIPs.length > 0 && (
            <div className="mb-2">
              <p className="text-xs text-text-secondary mb-1">
                本地 IP ({result.localIPs.length}):
              </p>
              <div className="space-y-0.5">
                {result.localIPs.map((ip, index) => (
                  <p key={index} className="uds-mono text-xs">
                    {ip}
                  </p>
                ))}
              </div>
            </div>
          )}

          {/* 公网 IP */}
          {result.publicIPs.length > 0 && (
            <div>
              <p className="text-xs text-text-secondary mb-1">
                公网 IP ({result.publicIPs.length}):
              </p>
              <div className="space-y-0.5">
                {result.publicIPs.map((ip, index) => (
                  <p
                    key={index}
                    className={`uds-mono text-xs ${
                      result.leakedIPs.includes(ip) ? 'text-error font-medium' : ''
                    }`}
                  >
                    {ip}
                    {result.leakedIPs.includes(ip) && ' ⚠️ 泄漏'}
                  </p>
                ))}
              </div>
            </div>
          )}
        </div>
      )}

      {/* 建议 */}
      {showDetails && result.recommendations.length > 0 && (
        <div className="p-2 bg-surface-variant rounded text-xs">
          <p className="font-medium mb-1">修复建议：</p>
          <div className="space-y-0.5 text-text-secondary">
            {result.recommendations.map((item, index) => (
              <p key={index}>{item}</p>
            ))}
          </div>
        </div>
      )}

      {/* 操作按钮 */}
      <div className="flex gap-2 mt-2">
        <Button
          size="small"
          variant="outlined"
          onClick={onToggleDetails}
          className="flex-1"
        >
          {showDetails ? '隐藏详情' : '查看详情'}
        </Button>
        <Button
          size="small"
          variant="outlined"
          onClick={onRetry}
          className="flex-1"
        >
          重新检测
        </Button>
      </div>

      {/* 检测时间 */}
      <p className="text-xs text-text-secondary text-center opacity-70">
        检测时间: {new Date(result.timestamp).toLocaleTimeString()}
      </p>
    </div>
  )
}

function useWebRTCLeakDetection() {
  return useQuery({
    queryKey: ['webrtc-leak-detection'],
    queryFn: detectWebRTCLeak,
    staleTime: 5 * 60 * 1000, // 5分钟
    gcTime: 10 * 60 * 1000, // 10分钟
    refetchOnWindowFocus: false,
    retry: 1,
    enabled: isWebRTCSupported(), // 只在支持 WebRTC 时启用
  })
}
