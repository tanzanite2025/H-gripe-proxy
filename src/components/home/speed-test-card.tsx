import { AlertCircle, Gauge, Play, Square } from 'lucide-react'
import {
  forwardRef,
  useCallback,
  useRef,
  useState,
  type PropsWithChildren,
} from 'react'

import { Button } from '@/components/tailwind/Button'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import {
  formatLatency,
  formatSpeed,
  getLatencyGrade,
  getPacketLossGrade,
  getSpeedGrade,
  getSpeedTestPhaseLabel,
  SpeedTestService,
  type SpeedTestProgress,
  type SpeedTestResult,
} from '@/services/speed-test'

import { EnhancedCard } from './enhanced-card'

const SpeedTestCardContainer = forwardRef<HTMLElement, PropsWithChildren>(
  ({ children }, ref) => {
    return (
      <EnhancedCard
        title="网络速度测试"
        icon={<Gauge className="h-5 w-5" />}
        iconColor="info"
        ref={ref}
      >
        {children}
      </EnhancedCard>
    )
  },
)

SpeedTestCardContainer.displayName = 'SpeedTestCardContainer'

export const SpeedTestCard = () => {
  const [result, setResult] = useState<SpeedTestResult | null>(null)
  const [progress, setProgress] = useState<SpeedTestProgress | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [isRunning, setIsRunning] = useState(false)
  const serviceRef = useRef<SpeedTestService | null>(null)
  const cancelledRef = useRef(false)

  const handleStart = useCallback(async () => {
    cancelledRef.current = false
    setIsRunning(true)
    setError(null)
    setResult(null)
    setProgress(null)

    try {
      const service = new SpeedTestService((nextProgress) => {
        setProgress(nextProgress)
      })
      serviceRef.current = service

      const nextResult = await service.runFullTest()
      if (cancelledRef.current) {
        return
      }

      setResult(nextResult)
      setProgress(null)
    } catch (nextError) {
      if (!cancelledRef.current) {
        setError(nextError instanceof Error ? nextError.message : '测速失败')
      }
    } finally {
      setIsRunning(false)
      serviceRef.current = null
    }
  }, [])

  const handleStop = useCallback(() => {
    cancelledRef.current = true
    serviceRef.current?.abort()
    setIsRunning(false)
    setProgress(null)
    setError('测试已取消')
  }, [])

  return (
    <SpeedTestCardContainer>
      <SpeedTestCardUI
        result={result}
        progress={progress}
        error={error}
        isRunning={isRunning}
        onStart={handleStart}
        onStop={handleStop}
      />
    </SpeedTestCardContainer>
  )
}

interface SpeedTestCardUIProps {
  result: SpeedTestResult | null
  progress: SpeedTestProgress | null
  error: string | null
  isRunning: boolean
  onStart: () => void
  onStop: () => void
}

const SpeedTestCardUI = ({
  result,
  progress,
  error,
  isRunning,
  onStart,
  onStop,
}: SpeedTestCardUIProps) => {
  if (error && !isRunning) {
    return (
      <div className="flex flex-col items-center justify-center py-6 text-error">
        <AlertCircle className="mb-2 h-10 w-10" />
        <p className="mb-4 text-base text-error">{error}</p>
        <Button onClick={onStart} startIcon={<Play className="h-4 w-4" />}>
          重新测试
        </Button>
      </div>
    )
  }

  if (isRunning && progress) {
    return (
      <div className="flex flex-col gap-3">
        <div className="text-center">
          <p className="mb-1 text-lg font-medium">
            {getSpeedTestPhaseLabel(progress.phase)}
          </p>
          {progress.currentSpeed !== undefined && (
            <p className="text-2xl font-bold text-primary">
              {formatSpeed(progress.currentSpeed)}
            </p>
          )}
          {progress.message && (
            <p className="mt-1 text-xs text-text-secondary">
              {progress.message}
            </p>
          )}
        </div>

        <div>
          <LinearProgress
            variant="determinate"
            value={progress.progress}
            className="h-2"
          />
          <p className="mt-1 text-center text-xs text-text-secondary">
            {Math.round(progress.progress)}%
          </p>
        </div>

        <Button
          onClick={onStop}
          variant="outlined"
          startIcon={<Square className="h-4 w-4" />}
          className="mt-2"
        >
          停止测试
        </Button>
      </div>
    )
  }

  if (result) {
    const downloadGrade = getSpeedGrade(result.download.speed)
    const uploadGrade = getSpeedGrade(result.upload.speed)
    const latencyGrade = getLatencyGrade(result.latency.avg)
    const packetLossGrade = getPacketLossGrade(result.packetLoss.lossRate)

    return (
      <div className="flex flex-col gap-3">
        <SpeedMetricPanel
          title="下载速度"
          value={formatSpeed(result.download.speed)}
          stability={result.download.stability}
          gradeColor={downloadGrade.color}
          gradeLabel={downloadGrade.label}
          progressValue={Math.min(result.download.speed, 100)}
        />
        <SpeedMetricPanel
          title="上传速度"
          value={formatSpeed(result.upload.speed)}
          stability={result.upload.stability}
          gradeColor={uploadGrade.color}
          gradeLabel={uploadGrade.label}
          progressValue={Math.min(result.upload.speed, 100)}
        />

        <div className="grid grid-cols-2 gap-2">
          <MetricPanel
            title="延迟"
            value={formatLatency(result.latency.avg)}
            detail={`抖动: ${formatLatency(result.latency.jitter)}`}
            valueColor={latencyGrade.color}
          />
          <MetricPanel
            title="丢包率"
            value={`${result.packetLoss.lossRate.toFixed(2)}%`}
            detail={`${result.packetLoss.received}/${result.packetLoss.sent}`}
            valueColor={packetLossGrade.color}
          />
        </div>

        <Button
          onClick={onStart}
          startIcon={<Play className="h-4 w-4" />}
          className="mt-4"
        >
          重新测试
        </Button>
      </div>
    )
  }

  return (
    <div className="flex flex-col items-center justify-center py-6">
      <Gauge className="mb-2 h-10 w-10 text-primary" />
      <p className="mb-4 text-base text-text-secondary">
        点击下方按钮开始测试网络速度
      </p>
      <Button onClick={onStart} startIcon={<Play className="h-4 w-4" />}>
        开始测试
      </Button>
    </div>
  )
}

interface SpeedMetricPanelProps {
  title: string
  value: string
  stability: number
  gradeColor: string
  gradeLabel: string
  progressValue: number
}

const SpeedMetricPanel = ({
  title,
  value,
  stability,
  gradeColor,
  gradeLabel,
  progressValue,
}: SpeedMetricPanelProps) => {
  return (
    <div className="rounded bg-surface-variant p-3">
      <div className="mb-2 flex items-center justify-between">
        <p className="text-sm text-text-secondary">{title}</p>
        <span className={`text-xs font-medium ${gradeColor}`}>{gradeLabel}</span>
      </div>
      <div className="flex items-baseline gap-2">
        <p className="text-2xl font-bold">{value}</p>
        <p className="text-xs text-text-secondary">稳定性 {stability}%</p>
      </div>
      <div className="mt-2">
        <LinearProgress
          variant="determinate"
          value={progressValue}
          className="h-1.5"
        />
      </div>
    </div>
  )
}

interface MetricPanelProps {
  title: string
  value: string
  detail: string
  valueColor: string
}

const MetricPanel = ({
  title,
  value,
  detail,
  valueColor,
}: MetricPanelProps) => {
  return (
    <div className="rounded bg-surface-variant p-2">
      <p className="mb-1 text-xs text-text-secondary">{title}</p>
      <p className={`text-lg font-bold ${valueColor}`}>{value}</p>
      <p className="mt-1 text-xs text-text-secondary">{detail}</p>
    </div>
  )
}
