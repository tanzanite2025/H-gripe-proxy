/**
 * DNS 泄漏检测卡片组件
 * 显示 DNS 是否泄漏，提供修复建议
 */

import { useQuery } from '@tanstack/react-query'
import {
  AlertCircle,
  RefreshCw,
  Shield,
} from 'lucide-react'
import { forwardRef, useState } from 'react'

import { buildHomeDnsLeakViewModel } from '@/components/setting/dns-leak-test-view-model'
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
    const { refetch } = useDNSLeakDetection()

    return (
      <EnhancedCard
        title="DNS 安全检测"
        icon={<Shield className="h-5 w-5" />}
        iconColor="info"
        ref={ref}
        fixedHeight={280}
        action={
          <IconButton size="small" onClick={() => refetch()}>
            <RefreshCw className="h-4 w-4" />
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
        <AlertCircle className="mb-2 h-10 w-10" />
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
  const leakView = buildHomeDnsLeakViewModel(result)

  return (
    <div className="flex flex-col gap-3">
      {/* 风险状态 */}
      <div className="flex items-center gap-2">
        <Shield className={`h-5 w-5 shrink-0 ${riskInfo.color}`} />
        <p className={`text-sm font-medium ${riskInfo.color}`}>
          {riskInfo.title} — {riskInfo.description}
        </p>
      </div>

      <div className="grid grid-cols-4 gap-2 text-sm">
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">结果判定</p>
          <p className="text-sm font-medium">
            {leakView.assessment.label}
          </p>
        </div>
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">结果置信度</p>
          <p className="text-sm font-medium">
            {leakView.confidence.label}
          </p>
        </div>
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

      {!result.locationComparable && (
        <div className="p-2 bg-surface-variant rounded text-xs text-text-secondary">
          当前 DNS 位置与出口位置尚不可直接比较，结果主要基于现有外部观测与运行态风险信号。
        </div>
      )}

      {showDetails && result.warnings.length > 0 && (
        <div className="p-2 bg-surface-variant rounded text-xs">
          <p className="font-medium mb-1">观测提示：</p>
          <div className="space-y-0.5 text-text-secondary">
            {result.warnings.map((warning) => (
              <p key={warning}>{warning}</p>
            ))}
          </div>
        </div>
      )}

      {showDetails && result.runtimeRiskType.length > 0 && (
        <div className="p-2 bg-surface-variant rounded text-xs">
          <p className="font-medium mb-1">运行态风险：</p>
          <div className="space-y-0.5 text-text-secondary">
            {result.runtimeRiskType.map((item) => (
              <p key={item}>{item}</p>
            ))}
          </div>
        </div>
      )}

      {showDetails && result.observedLeakType.length > 0 && (
        <div className="p-2 bg-surface-variant rounded text-xs">
          <p className="font-medium mb-1">外部观测信号：</p>
          <div className="space-y-0.5 text-text-secondary">
            {result.observedLeakType.map((item) => (
              <p key={item}>{item}</p>
            ))}
          </div>
        </div>
      )}

      {/* DNS 服务器列表 */}
      {showDetails && result.dnsServers.length > 0 && (
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs font-medium mb-2">DNS 服务器：</p>
          <div className="space-y-1.5">
            {result.dnsServers.map((dns) => (
              <div key={`${dns.ip}-${dns.hostname ?? ''}`} className="text-xs">
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
            {result.recommendations.map((item) => (
              <p key={item}>{item}</p>
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
