/**
 * 代理检测卡片组件
 * 显示代理是否生效，对比直连和代理的 IP 信息
 */

import {
  CheckCircleOutlined,
  ErrorOutlined,
  InfoOutlined,
  RefreshOutlined,
  SaveOutlined,
  WarningOutlined,
} from '@mui/icons-material'
import { useQuery } from '@tanstack/react-query'
import { forwardRef, useCallback, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import { IconButton } from '@/components/tailwind/IconButton'
import { Skeleton } from '@/components/tailwind/Skeleton'
import {
  clearDirectIP,
  detectProxy,
  getProxyDetectionAdvice,
  saveDirectIP,
  type ProxyDetectionResult,
} from '@/services/proxy-detection'

import { EnhancedCard } from './enhanced-card'

const ProxyDetectionCardContainer = forwardRef<
  HTMLElement,
  React.PropsWithChildren
>(({ children }, ref) => {
  const { t } = useTranslation()
  const { refetch } = useProxyDetection()

  return (
    <EnhancedCard
      title="代理检测"
      icon={<InfoOutlined />}
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

ProxyDetectionCardContainer.displayName = 'ProxyDetectionCardContainer'

export const ProxyDetectionCard = () => {
  const { data, error, isLoading, refetch } = useProxyDetection()
  const [showAdvice, setShowAdvice] = useState(false)

  const handleSaveDirectIP = useCallback(() => {
    if (data && data.proxyIP !== 'Unknown') {
      saveDirectIP({
        ip: data.proxyIP,
        country: data.proxyLocation.country,
        country_code: data.proxyLocation.country_code,
        city: data.proxyLocation.city,
        region: data.proxyLocation.region,
      })
      // 重新检测
      refetch()
    }
  }, [data, refetch])

  const handleClearDirectIP = useCallback(() => {
    clearDirectIP()
    refetch()
  }, [refetch])

  return (
    <ProxyDetectionCardContainer>
      <ProxyDetectionCardUI
        result={data}
        error={error}
        isLoading={isLoading}
        showAdvice={showAdvice}
        onToggleAdvice={() => setShowAdvice(!showAdvice)}
        onSaveDirectIP={handleSaveDirectIP}
        onClearDirectIP={handleClearDirectIP}
        onRetry={() => refetch()}
      />
    </ProxyDetectionCardContainer>
  )
}

interface ProxyDetectionCardUIProps {
  result?: ProxyDetectionResult
  error?: Error | null
  isLoading: boolean
  showAdvice: boolean
  onToggleAdvice: () => void
  onSaveDirectIP: () => void
  onClearDirectIP: () => void
  onRetry: () => void
}

const ProxyDetectionCardUI = ({
  result,
  error,
  isLoading,
  showAdvice,
  onToggleAdvice,
  onSaveDirectIP,
  onClearDirectIP,
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

  const advice = getProxyDetectionAdvice(result)

  return (
    <div className="flex flex-col gap-3">
      {/* 代理状态 */}
      <div className="flex items-center gap-2">
        {result.isProxyWorking ? (
          <>
            <CheckCircleOutlined className="text-success text-2xl" />
            <div>
              <p className="text-base font-medium text-success">
                ✅ 代理已生效
              </p>
              <p className="text-xs text-text-secondary">
                {result.ipChanged && 'IP 地址已改变'}
                {result.ipChanged && result.locationChanged && ' • '}
                {result.locationChanged && '地理位置已改变'}
              </p>
            </div>
          </>
        ) : (
          <>
            <WarningOutlined className="text-warning text-2xl" />
            <div>
              <p className="text-base font-medium text-warning">
                ⚠️ 未检测到代理
              </p>
              <p className="text-xs text-text-secondary">
                可能未启用代理或使用本地代理
              </p>
            </div>
          </>
        )}
      </div>

      {/* IP 信息对比 */}
      <div className="grid grid-cols-2 gap-2 text-sm">
        {/* 直连 IP */}
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">直连 IP</p>
          {result.directIP ? (
            <>
              <p className="uds-mono text-xs font-medium">
                {result.directIP}
              </p>
              <p className="text-xs text-text-secondary mt-1">
                {result.directLocation?.country} {result.directLocation?.city}
              </p>
            </>
          ) : (
            <p className="text-xs text-text-secondary">未记录</p>
          )}
        </div>

        {/* 代理 IP */}
        <div className="p-2 bg-surface-variant rounded">
          <p className="text-xs text-text-secondary mb-1">当前 IP</p>
          <p className="uds-mono text-xs font-medium">{result.proxyIP}</p>
          <p className="text-xs text-text-secondary mt-1">
            {result.proxyLocation.country} {result.proxyLocation.city}
          </p>
        </div>
      </div>

      {/* 建议 */}
      {showAdvice && (
        <div className="p-2 bg-surface-variant rounded text-xs">
          <p className="font-medium mb-1">检测建议：</p>
          <ul className="list-disc list-inside space-y-0.5 text-text-secondary">
            {advice.map((item, index) => (
              <li key={index}>{item}</li>
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
        {result.directIP ? (
          <Button
            size="small"
            variant="outlined"
            onClick={onClearDirectIP}
            className="flex-1"
          >
            清除记录
          </Button>
        ) : (
          <Button
            size="small"
            variant="outlined"
            onClick={onSaveDirectIP}
            startIcon={<SaveOutlined />}
            className="flex-1"
          >
            保存为直连 IP
          </Button>
        )}
      </div>
    </div>
  )
}

function useProxyDetection() {
  return useQuery({
    queryKey: ['proxy-detection'],
    queryFn: detectProxy,
    staleTime: 5 * 60 * 1000, // 5分钟
    gcTime: 10 * 60 * 1000, // 10分钟
    refetchOnWindowFocus: false,
    retry: 1,
  })
}
