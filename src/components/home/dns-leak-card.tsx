/**
 * DNS 泄漏检测卡片组件
 * 显示 DNS 是否泄漏，提供修复建议
 */

import {
  ErrorOutlined,
  RefreshOutlined,
  SecurityOutlined,
} from '@mui/icons-material'
import { useQuery } from '@tanstack/react-query'
import { forwardRef, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import { IconButton } from '@/components/tailwind/IconButton'
import { Skeleton } from '@/components/tailwind/Skeleton'
import {
  detectDNSLeak,
  getDNSLeakRiskDescription,
  type DNSLeakResult,
} from '@/services/dns-leak-detection'

import { EnhancedCard } from './enhanced-card'

const DNSLeakCardContainer = forwardRef<HTMLElement, React.PropsWithChildren>(
  ({ children }, ref) => {
    const { t } = useTranslation()
    const { refetch } = useDNSLeakDetection()

    return (
      <EnhancedCard
        title="DNS 安全检测"
        icon={<SecurityOutlined />}
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
  }
)

DNSLeakCardContainer.displayName = 'DNSLeakCardContainer'

export const DNSLeakCard = () => {
  const { data, error, isLoading, refetch } = useDNSLeakDetection()
  const [showDetails, setShowDetails] = useState(false)

  return (
    <DNSLeakCardContainer>
      <DNSLeakCardUI
        result={data}
        error={error}
        isLoading={isLoading}
        showDetails={showDetails}
        onToggleDetails={() => setShowDetails(!showDetails)}
        onRetry={() => refetch()}
      />
    </DNSLeakCardContainer>
  )
}

interface DNSLeakCardUIProps {
  result?: DNSLeakResult
  error?: Error | null
  isLoading: boolean
  showDetails: boolean
  onToggleDetails: () => void
  onRetry: () => void
}

const DNSLeakCardUI = ({
  result,
  error,
  isLoading,
  showDetails,
  onToggleDetails,
  onRetry,
}: DNSLeakCardUIProps) => {
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

  const riskInfo = getDNSLeakRiskDescription(result.riskLevel)

  return (
    <div className="flex flex-col gap-3">
      {/* 风险状态 */}
      <div className="flex items-start gap-2">
        <SecurityOutlined className={`text-2xl ${riskInfo.color}`} />
        <div className="flex-1">
          <p className={`text-base font-medium ${riskInfo.color}`}>
            {riskInfo.title}
          </p>
          <p className="text-xs text-text-secondary mt-0.5">
            {riskInfo.description}
          </p>
        </div>
      </div>

      {/* 位置对比 */}
      <div className="grid grid-cols-2 gap-2 text-sm">
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">DNS 位置</p>
          <p className="text-sm font-medium">
            {result.dnsLocation || 'Unknown'}
          </p>
        </div>
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">代理位置</p>
          <p className="text-sm font-medium">{result.ipLocation}</p>
        </div>
      </div>

      {/* DNS 服务器列表 */}
      {showDetails && result.dnsServers.length > 0 && (
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs font-medium mb-2">DNS 服务器：</p>
          <div className="space-y-1.5">
            {result.dnsServers.map((dns, index) => (
              <div key={index} className="text-xs">
                <p className="uds-mono font-medium">{dns.ip}</p>
                {dns.hostname && (
                  <p className="text-text-secondary">{dns.hostname}</p>
                )}
                {dns.country && (
                  <p className="text-text-secondary">
                    {dns.country}
                    {dns.city && ` • ${dns.city}`}
                    {dns.isp && ` • ${dns.isp}`}
                  </p>
                )}
              </div>
            ))}
          </div>
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

function useDNSLeakDetection() {
  return useQuery({
    queryKey: ['dns-leak-detection'],
    queryFn: detectDNSLeak,
    staleTime: 5 * 60 * 1000, // 5分钟
    gcTime: 10 * 60 * 1000, // 10分钟
    refetchOnWindowFocus: false,
    retry: 1,
  })
}
