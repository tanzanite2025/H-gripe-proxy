/**
 * 代理检测卡片组件
 * 显示代理是否生效，对比直连和代理的 IP 信息
 */

import { useQuery } from '@tanstack/react-query'
import {
  AlertCircle as ErrorOutlined,
  CheckCircle2 as CheckCircleOutlined,
  Info as InfoOutlined,
  RefreshCw as RefreshOutlined,
  TriangleAlert as WarningOutlined,
} from 'lucide-react'
import { forwardRef, useCallback, useState } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { IconButton } from '@/components/tailwind/IconButton'
import { Skeleton } from '@/components/tailwind/Skeleton'
import {
  testProxyDetection,
  type ProxyDetectionResult,
} from '@/services/cmds'

import { EnhancedCard } from './enhanced-card'

const getAssessmentLabel = (assessment?: string) => {
  switch (assessment) {
    case 'effective':
      return '出口已变化'
    case 'same-egress':
      return '出口未变化'
    case 'runtime-risk':
      return '存在运行风险'
    case 'inconclusive':
      return '结果不确定'
    default:
      return assessment || '未知'
  }
}

const getAssessmentColor = (assessment?: string) => {
  switch (assessment) {
    case 'effective':
      return 'success' as const
    case 'same-egress':
      return 'warning' as const
    case 'runtime-risk':
      return 'warning' as const
    case 'inconclusive':
      return 'info' as const
    default:
      return 'default' as const
  }
}

const getConfidenceLabel = (confidence?: string) => {
  switch (confidence) {
    case 'high':
      return '高置信度'
    case 'medium':
      return '中置信度'
    case 'low':
      return '低置信度'
    default:
      return confidence || '未知'
  }
}

const formatObservationPath = (observationPath?: string) => {
  switch (observationPath) {
    case 'direct-vs-core-proxy':
      return '直连 vs 本地 core 代理'
    case 'direct-only':
      return '仅直连'
    case 'core-proxy-only':
      return '仅本地 core 代理'
    default:
      return observationPath || '未知'
  }
}

const formatRuntimeRiskLabel = (risk: string) => {
  switch (risk) {
    case 'core-not-running':
      return '本地 core 未运行'
    case 'direct-egress-unavailable':
      return '直连出口不可观测'
    case 'local-core-proxy-unreachable':
      return '本地 core 代理出口不可观测'
    default:
      return risk
  }
}

const formatLocation = (location?: ProxyDetectionResult['direct_location']) => {
  if (!location) {
    return '未观测到'
  }

  return [location.country, location.region, location.city].filter(Boolean).join(' ') || '未知'
}

const ProxyDetectionCardContainer = forwardRef<
  HTMLElement,
  React.PropsWithChildren<{ onRefresh: () => void }>
>(({ children, onRefresh }, ref) => {

  return (
    <EnhancedCard
      title="代理检测"
      icon={<InfoOutlined />}
      iconColor="info"
      ref={ref}
      action={
        <IconButton size="small" onClick={onRefresh}>
          <RefreshOutlined className="h-4 w-4" />
        </IconButton>
      }
    >
      {children}
    </EnhancedCard>
  )
})

ProxyDetectionCardContainer.displayName = 'ProxyDetectionCardContainer'

export const ProxyDetectionCard = () => {
  const { data, error, isLoading, isFetching, refetch } = useProxyDetection()
  const [showAdvice, setShowAdvice] = useState(false)

  const handleRefresh = useCallback(() => {
    void refetch()
  }, [refetch])

  return (
    <ProxyDetectionCardContainer onRefresh={handleRefresh}>
      <ProxyDetectionCardUI
        result={data}
        error={error}
        isLoading={isLoading}
        isFetching={isFetching}
        showAdvice={showAdvice}
        onToggleAdvice={() => setShowAdvice(!showAdvice)}
        onRetry={handleRefresh}
      />
    </ProxyDetectionCardContainer>
  )
}

interface ProxyDetectionCardUIProps {
  result?: ProxyDetectionResult
  error?: Error | null
  isLoading: boolean
  isFetching: boolean
  showAdvice: boolean
  onToggleAdvice: () => void
  onRetry: () => void
}

const ProxyDetectionCardUI = ({
  result,
  error,
  isLoading,
  isFetching,
  showAdvice,
  onToggleAdvice,
  onRetry,
}: ProxyDetectionCardUIProps) => {
  if (isLoading) {
    return (
      <div className="flex flex-col gap-2">
        <Skeleton variant="text" width="60%" height={30} />
        <Skeleton variant="text" width="80%" height={24} />
        <Skeleton variant="text" width="70%" height={24} />
      </div>
    )
  }

  if (error && !result) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-error">
        <ErrorOutlined className="mb-2 h-10 w-10" />
        <p className="text-base text-error">
          {error instanceof Error ? error.message : '检测失败'}
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

  const advice = result.recommendations

  return (
    <div className="flex flex-col gap-3">
      {/* 代理状态 */}
      <div className="flex items-center gap-2">
        {result.proxy_effective ? (
          <>
            <CheckCircleOutlined className="h-8 w-8 text-success" />
            <div>
              <p className="text-base font-medium text-success">
                ✅ 已观察到代理出口变化
              </p>
              <p className="text-xs text-text-secondary">
                {result.ip_changed && 'IP 地址已改变'}
                {result.ip_changed && result.location_changed && ' • '}
                {result.location_changed && '地理位置已改变'}
              </p>
            </div>
          </>
        ) : result.assessment === 'same-egress' ? (
          <>
            <WarningOutlined className="h-8 w-8 text-warning" />
            <div>
              <p className="text-base font-medium text-warning">
                ⚠️ 未观察到代理出口变化
              </p>
              <p className="text-xs text-text-secondary">
                当前软件流量的直连与本地 core 代理出口没有明显差异
              </p>
            </div>
          </>
        ) : (
          <>
            <InfoOutlined className="h-8 w-8 text-info" />
            <div>
              <p className="text-base font-medium text-info">
                ℹ️ 当前代理观测不完整
              </p>
              <p className="text-xs text-text-secondary">
                还未同时拿到软件自身流量的直连与代理两条出口观测
              </p>
            </div>
          </>
        )}
      </div>

      <div className="flex flex-wrap gap-2 text-sm">
        <Chip
          label={getAssessmentLabel(result.assessment)}
          color={getAssessmentColor(result.assessment)}
          size="small"
        />
        <Chip label={getConfidenceLabel(result.confidence)} color="info" size="small" />
        <Chip label={formatObservationPath(result.observation_path)} size="small" />
        <Chip
          label={result.core_running ? '本地 core 已运行' : '本地 core 未运行'}
          color={result.core_running ? 'success' : 'warning'}
          size="small"
        />
      </div>

      {/* IP 信息对比 */}
      <div className="grid grid-cols-2 gap-2 text-sm">
        {/* 直连 IP */}
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">直连出口</p>
          {result.direct_ip ? (
            <>
              <p className="uds-mono text-xs font-medium">
                {result.direct_ip}
              </p>
              <p className="text-xs text-text-secondary mt-1">
                {formatLocation(result.direct_location)}
              </p>
            </>
          ) : (
            <p className="text-xs text-text-secondary">未观测到</p>
          )}
        </div>

        {/* 代理 IP */}
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">代理出口</p>
          {result.proxy_ip ? (
            <>
              <p className="uds-mono text-xs font-medium">{result.proxy_ip}</p>
              <p className="text-xs text-text-secondary mt-1">
                {formatLocation(result.proxy_location)}
              </p>
            </>
          ) : (
            <p className="text-xs text-text-secondary">未观测到</p>
          )}
        </div>
      </div>

      {result.observation_incomplete ? (
        <Alert severity="info" className="text-xs">
          当前仅拿到了部分出口观测结果，建议在直连与本地 core 两条路径都可用时重新检测。
        </Alert>
      ) : null}

      {result.runtime_risk_detected && result.runtime_risk_type.length > 0 ? (
        <Alert severity="warning" className="text-xs">
          {result.runtime_risk_type.map(formatRuntimeRiskLabel).join('；')}
        </Alert>
      ) : null}

      {result.warnings.length ? (
        <Alert severity="warning" className="text-xs">
          {result.warnings.join('；')}
        </Alert>
      ) : null}

      {result.error ? (
        <Alert severity="error" className="text-xs">
          {result.error}
        </Alert>
      ) : null}

      {/* 建议 */}
      {showAdvice && (
        <div className="p-2 bg-surface-variant rounded text-xs">
          <p className="font-medium mb-1">检测建议：</p>
          <ul className="list-disc list-inside space-y-0.5 text-text-secondary">
            {advice.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </div>
      )}

      {/* 操作按钮 */}
      <div className="flex gap-2 mt-2">
        <Button
          size="small"
          variant="outlined"
          onClick={onToggleAdvice}
          className="flex-1"
        >
          {showAdvice ? '隐藏建议' : '查看建议'}
        </Button>
        <Button
          size="small"
          variant="outlined"
          onClick={onRetry}
          loading={isFetching}
          className="flex-1"
        >
          重新检测
        </Button>
      </div>
    </div>
  )
}

function useProxyDetection() {
  return useQuery({
    queryKey: ['proxy-detection'],
    queryFn: testProxyDetection,
    staleTime: 5 * 60 * 1000, // 5分钟
    gcTime: 10 * 60 * 1000, // 10分钟
    refetchOnWindowFocus: false,
    retry: 1,
  })
}
