import { useQuery } from '@tanstack/react-query'
import {
  AlertCircle as ErrorOutlined,
  CheckCircle2 as CheckCircleOutlined,
  Info as InfoOutlined,
  RefreshCw as RefreshOutlined,
  TriangleAlert as WarningOutlined,
} from 'lucide-react'
import { forwardRef, useCallback, useState } from 'react'
import { useTranslation } from 'react-i18next'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { Dialog, DialogActions, DialogContent, DialogTitle } from '@/components/tailwind/Dialog'
import { IconButton } from '@/components/tailwind/IconButton'
import { Skeleton } from '@/components/tailwind/Skeleton'
import {
  testProxyDetection,
  type ProxyDetectionResult,
} from '@/services/cmds'

import { EnhancedCard } from './enhanced-card'
import { buildProxyDetectionViewModel } from './proxy-detection-view-model'

const ProxyDetectionCardContainer = forwardRef<
  HTMLElement,
  React.PropsWithChildren<{ onRefresh: () => void; title: string }>
>(({ children, onRefresh, title }, ref) => (
  <EnhancedCard
    title={title}
    icon={<InfoOutlined />}
    iconColor="info"
    ref={ref}
    fixedHeight={280}
    action={
      <IconButton size="small" onClick={onRefresh}>
        <RefreshOutlined className="h-4 w-4" />
      </IconButton>
    }
  >
    {children}
  </EnhancedCard>
))

ProxyDetectionCardContainer.displayName = 'ProxyDetectionCardContainer'

export const ProxyDetectionCard = () => {
  const { t } = useTranslation()
  const { data, error, isLoading, isFetching, refetch } = useProxyDetection()
  const [adviceDialogOpen, setAdviceDialogOpen] = useState(false)

  const handleRefresh = useCallback(() => {
    void refetch()
  }, [refetch])

  return (
    <ProxyDetectionCardContainer
      onRefresh={handleRefresh}
      title={t('home.components.proxyDetection.title')}
    >
      <ProxyDetectionCardUI
        result={data}
        error={error}
        isLoading={isLoading}
        isFetching={isFetching}
        onToggleAdvice={() => setAdviceDialogOpen(true)}
        adviceDialogOpen={adviceDialogOpen}
        onCloseAdviceDialog={() => setAdviceDialogOpen(false)}
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
  onToggleAdvice: () => void
  adviceDialogOpen: boolean
  onCloseAdviceDialog: () => void
  onRetry: () => void
}

const ProxyDetectionCardUI = ({
  result,
  error,
  isLoading,
  isFetching,
  onToggleAdvice,
  adviceDialogOpen,
  onCloseAdviceDialog,
  onRetry,
}: ProxyDetectionCardUIProps) => {
  const { t } = useTranslation()

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
      <div className="flex h-full flex-col items-center justify-center text-error">
        <ErrorOutlined className="mb-2 h-10 w-10" />
        <p className="text-base text-error">
          {error instanceof Error
            ? error.message
            : t('home.components.proxyDetection.errors.failed')}
        </p>
        <Button onClick={onRetry} className="mt-4">
          {t('home.components.proxyDetection.actions.retry')}
        </Button>
      </div>
    )
  }

  if (!result) {
    return null
  }

  const reputation = result.proxy_reputation
  const view = buildProxyDetectionViewModel(result, t)
  const advice = view.recommendations

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-2">
        {view.summary.state === 'effective' ? (
          <>
            <CheckCircleOutlined className="h-8 w-8 text-success" />
            <div>
              <p className={`text-base font-medium ${view.summary.colorClass}`}>
                {view.summary.title}
              </p>
              <p className="text-xs text-text-secondary">
                {view.summary.description}
              </p>
            </div>
          </>
        ) : view.summary.state === 'same-egress' ? (
          <>
            <WarningOutlined className="h-8 w-8 text-warning" />
            <div>
              <p className={`text-base font-medium ${view.summary.colorClass}`}>
                {view.summary.title}
              </p>
              <p className="text-xs text-text-secondary">
                {view.summary.description}
              </p>
            </div>
          </>
        ) : (
          <>
            <InfoOutlined className="h-8 w-8 text-info" />
            <div>
              <p className={`text-base font-medium ${view.summary.colorClass}`}>
                {view.summary.title}
              </p>
              <p className="text-xs text-text-secondary">
                {view.summary.description}
              </p>
            </div>
          </>
        )}
      </div>

      <div className="flex flex-wrap gap-2 text-sm">
        <Chip
          label={view.assessment.label}
          color={view.assessment.color}
          size="small"
        />
        <Chip label={view.confidence.label} color={view.confidence.color} size="small" />
        <Chip label={view.observationPath.label} size="small" />
        <Chip
          label={view.core.label}
          color={view.core.color}
          size="small"
        />
        {reputation ? (
          <Chip
            label={view.reputation?.label}
            color={view.reputation?.color}
            size="small"
          />
        ) : null}
      </div>

      <div className="flex flex-col gap-1.5 text-sm">
        <div className="flex items-center gap-2">
          <span className="shrink-0 text-xs text-text-secondary">
            {t('home.components.proxyDetection.labels.direct')}
          </span>
          <p className="uds-mono text-xs font-medium">
            {view.direct.ip}
          </p>
          {view.direct.observed && (
            <p className="text-xs text-text-secondary">
              {view.direct.location}
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          <span className="shrink-0 text-xs text-text-secondary">
            {t('home.components.proxyDetection.labels.proxy')}
          </span>
          <p className="uds-mono text-xs font-medium">
            {view.proxy.ip}
          </p>
          {view.proxy.observed && (
            <p className="text-xs text-text-secondary">
              {view.proxy.location}
            </p>
          )}
        </div>
        {reputation ? (
          <p className="truncate text-xs text-text-secondary">
            {view.reputation?.asnLabel}
          </p>
        ) : null}
      </div>

      {result.observation_incomplete ? (
        <Alert severity="info" className="text-xs">
          {t('home.components.proxyDetection.alerts.observationIncomplete')}
        </Alert>
      ) : null}

      {result.runtime_risk_detected && result.runtime_risk_type.length > 0 ? (
        <Alert severity="warning" className="text-xs">
          {view.runtimeRiskText}
        </Alert>
      ) : null}

      {result.warnings.length ? (
        <Alert severity="warning" className="text-xs">
          {result.warnings.join('; ')}
        </Alert>
      ) : null}

      {result.error ? (
        <Alert severity="error" className="text-xs">
          {result.error}
        </Alert>
      ) : null}

      <Dialog open={adviceDialogOpen} onClose={onCloseAdviceDialog}>
        <DialogTitle>
          {t('home.components.proxyDetection.adviceDialog.title')}
        </DialogTitle>
        <DialogContent>
          <ul className="list-inside list-disc space-y-1 text-sm text-text-secondary">
            {advice.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </DialogContent>
        <DialogActions>
          <Button onClick={onCloseAdviceDialog}>
            {t('home.components.proxyDetection.actions.close')}
          </Button>
        </DialogActions>
      </Dialog>

      <div className="mt-2 flex gap-2">
        <Button
          size="small"
          variant="outlined"
          onClick={onToggleAdvice}
          className="flex-1"
        >
          {t('home.components.proxyDetection.actions.advice')}
        </Button>
        <Button
          size="small"
          variant="outlined"
          onClick={onRetry}
          loading={isFetching}
          className="flex-1"
        >
          {t('home.components.proxyDetection.actions.refresh')}
        </Button>
      </div>
    </div>
  )
}

function useProxyDetection() {
  return useQuery({
    queryKey: ['proxy-detection'],
    queryFn: testProxyDetection,
    staleTime: 5 * 60 * 1000,
    gcTime: 10 * 60 * 1000,
    refetchOnWindowFocus: false,
    retry: 1,
  })
}
