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
import { Dialog, DialogActions, DialogContent, DialogTitle } from '@/components/tailwind/Dialog'
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
      return 'Exit changed'
    case 'same-egress':
      return 'Same exit'
    case 'runtime-risk':
      return 'Runtime risk'
    case 'inconclusive':
      return 'Inconclusive'
    default:
      return assessment || 'Unknown'
  }
}

const getAssessmentColor = (assessment?: string) => {
  switch (assessment) {
    case 'effective':
      return 'success' as const
    case 'same-egress':
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
      return 'High confidence'
    case 'medium':
      return 'Medium confidence'
    case 'low':
      return 'Low confidence'
    default:
      return confidence || 'Unknown'
  }
}

const formatObservationPath = (observationPath?: string) => {
  switch (observationPath) {
    case 'direct-vs-core-proxy':
      return 'Direct vs core'
    case 'direct-only':
      return 'Direct only'
    case 'core-proxy-only':
      return 'Core only'
    default:
      return observationPath || 'Unknown'
  }
}

const formatRuntimeRiskLabel = (risk: string) => {
  switch (risk) {
    case 'core-not-running':
      return 'Local core is not running'
    case 'direct-egress-unavailable':
      return 'Direct egress unavailable'
    case 'local-core-proxy-unreachable':
      return 'Core proxy egress unavailable'
    case 'proxy-reputation-unavailable':
      return 'Proxy reputation unavailable'
    default:
      return risk
  }
}

const formatLocation = (location?: ProxyDetectionResult['direct_location']) => {
  if (!location) {
    return 'Not observed'
  }

  return [location.country, location.region, location.city].filter(Boolean).join(' ') || 'Unknown'
}

const getRiskColor = (riskLevel?: string) => {
  switch (riskLevel) {
    case 'Low':
      return 'success' as const
    case 'Medium':
      return 'info' as const
    case 'High':
    case 'VeryHigh':
      return 'warning' as const
    default:
      return 'default' as const
  }
}

const ProxyDetectionCardContainer = forwardRef<
  HTMLElement,
  React.PropsWithChildren<{ onRefresh: () => void }>
>(({ children, onRefresh }, ref) => (
  <EnhancedCard
    title="Proxy Detection"
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
  const { data, error, isLoading, isFetching, refetch } = useProxyDetection()
  const [adviceDialogOpen, setAdviceDialogOpen] = useState(false)

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
          {error instanceof Error ? error.message : 'Detection failed'}
        </p>
        <Button onClick={onRetry} className="mt-4">
          Retry
        </Button>
      </div>
    )
  }

  if (!result) {
    return null
  }

  const reputation = result.proxy_reputation
  const advice = result.recommendations

  return (
    <div className="flex flex-col gap-3">
      <div className="flex items-center gap-2">
        {result.proxy_effective ? (
          <>
            <CheckCircleOutlined className="h-8 w-8 text-success" />
            <div>
              <p className="text-base font-medium text-success">
                Proxy exit changed
              </p>
              <p className="text-xs text-text-secondary">
                {result.ip_changed && 'IP changed'}
                {result.ip_changed && result.location_changed && ' / '}
                {result.location_changed && 'Location changed'}
              </p>
            </div>
          </>
        ) : result.assessment === 'same-egress' ? (
          <>
            <WarningOutlined className="h-8 w-8 text-warning" />
            <div>
              <p className="text-base font-medium text-warning">
                Same egress observed
              </p>
              <p className="text-xs text-text-secondary">
                Direct and local-core proxy paths currently look identical.
              </p>
            </div>
          </>
        ) : (
          <>
            <InfoOutlined className="h-8 w-8 text-info" />
            <div>
              <p className="text-base font-medium text-info">
                Observation incomplete
              </p>
              <p className="text-xs text-text-secondary">
                Direct and proxy paths were not both observed.
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
          label={result.core_running ? 'Core running' : 'Core stopped'}
          color={result.core_running ? 'success' : 'warning'}
          size="small"
        />
        {reputation ? (
          <Chip
            label={`${reputation.ipType} / score ${reputation.fraudScore}`}
            color={getRiskColor(reputation.riskLevel)}
            size="small"
          />
        ) : null}
      </div>

      <div className="flex flex-col gap-1.5 text-sm">
        <div className="flex items-center gap-2">
          <span className="shrink-0 text-xs text-text-secondary">Direct</span>
          <p className="uds-mono text-xs font-medium">
            {result.direct_ip || 'Not observed'}
          </p>
          {result.direct_ip && result.direct_location && (
            <p className="text-xs text-text-secondary">
              {formatLocation(result.direct_location)}
            </p>
          )}
        </div>
        <div className="flex items-center gap-2">
          <span className="shrink-0 text-xs text-text-secondary">Proxy</span>
          <p className="uds-mono text-xs font-medium">
            {result.proxy_ip || 'Not observed'}
          </p>
          {result.proxy_ip && result.proxy_location && (
            <p className="text-xs text-text-secondary">
              {formatLocation(result.proxy_location)}
            </p>
          )}
        </div>
        {reputation ? (
          <p className="truncate text-xs text-text-secondary">
            ASN {reputation.asn} · {reputation.asnOrg}
          </p>
        ) : null}
      </div>

      {result.observation_incomplete ? (
        <Alert severity="info" className="text-xs">
          Only part of the egress observation is available. Retry when both direct
          and local-core proxy paths are reachable.
        </Alert>
      ) : null}

      {result.runtime_risk_detected && result.runtime_risk_type.length > 0 ? (
        <Alert severity="warning" className="text-xs">
          {result.runtime_risk_type.map(formatRuntimeRiskLabel).join('; ')}
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
        <DialogTitle>Detection Advice</DialogTitle>
        <DialogContent>
          <ul className="list-inside list-disc space-y-1 text-sm text-text-secondary">
            {advice.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </DialogContent>
        <DialogActions>
          <Button onClick={onCloseAdviceDialog}>Close</Button>
        </DialogActions>
      </Dialog>

      <div className="mt-2 flex gap-2">
        <Button
          size="small"
          variant="outlined"
          onClick={onToggleAdvice}
          className="flex-1"
        >
          Advice
        </Button>
        <Button
          size="small"
          variant="outlined"
          onClick={onRetry}
          loading={isFetching}
          className="flex-1"
        >
          Refresh
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
