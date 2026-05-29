/**
 * 网络速度测试卡片组件
 * 测试下载/上传速度、延迟、丢包率
 */

import {
  AlertCircle,
  Gauge,
  Play,
  Square,
} from 'lucide-react'
import { forwardRef, useCallback, useRef, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import {
  formatLatency,
  formatSpeed,
  getLatencyGrade,
  getPacketLossGrade,
  getSpeedGrade,
  SpeedTestService,
  type SpeedTestProgress,
  type SpeedTestResult,
} from '@/services/speed-test'

import { EnhancedCard } from './enhanced-card'

const SpeedTestCardContainer = forwardRef<HTMLElement, React.PropsWithChildren>(
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

  const handleStart = useCallback(async () => {
    setIsRunning(true)
    setError(null)
    setResult(null)
    setProgress(null)

    try {
      const service = new SpeedTestService((prog) => {
        setProgress(prog)
      })
      serviceRef.current = service

      const testResult = await service.runFullTest()
      setResult(testResult)
      setProgress(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : '测试失败')
    } finally {
      setIsRunning(false)
      serviceRef.current = null
    }
  }, [])

  const handleStop = useCallback(() => {
    if (serviceRef.current) {
      serviceRef.current.abort()
      setIsRunning(false)
      setProgress(null)
      setError('测试已取消')
    }
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
  // 错误状态
  if (error && !isRunning) {
    return (
      <div className="flex flex-col items-center justify-center py-6 text-error">
        <AlertCircle className="mb-2 h-10 w-10" />
        <p className="text-base text-error mb-4">{error}</p>
        <Button onClick={onStart} startIcon={<Play className="h-4 w-4" />}>
          重新测试
        </Button>
      </div>
    )
  }

  // 测试中状态
  if (isRunning && progress) {
    return (
      <div className="flex flex-col gap-3">
        {/* 当前阶段 */}
        <div className="text-center">
          <p className="text-lg font-medium mb-1">
            {getPhaseLabel(progress.phase)}
          </p>
          {progress.currentSpeed !== undefined && (
            <p className="text-2xl font-bold text-primary">
              {formatSpeed(progress.currentSpeed)}
            </p>
          )}
          {progress.message && (
            <p className="text-xs text-text-secondary mt-1">
              {progress.message}
            </p>
          )}
        </div>

        {/* 进度条 */}
        <div>
          <LinearProgress
            variant="determinate"
            value={progress.progress}
            className="h-2"
          />
          <p className="text-xs text-text-secondary text-center mt-1">
            {Math.round(progress.progress)}%
          </p>
        </div>

        {/* 停止按钮 */}
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

  // 结果显示
  if (result) {
    const downloadGrade = getSpeedGrade(result.download.speed)
    const uploadGrade = getSpeedGrade(result.upload.speed)
    const latencyGrade = getLatencyGrade(result.latency.avg)
    const packetLossGrade = getPacketLossGrade(result.packetLoss.lossRate)

    return (
      <div className="flex flex-col gap-3">
        {/* 下载速度 */}
        <div className="p-3 bg-surface-variant rounded">
          <div className="flex justify-between items-center mb-2">
            <p className="text-sm text-text-secondary">下载速度</p>
            <span className={`text-xs font-medium ${downloadGrade.color}`}>
              {downloadGrade.label}
            </span>
          </div>
          <div className="flex items-baseline gap-2">
            <p className="text-2xl font-bold">
              {formatSpeed(result.download.speed)}
            </p>
            <p className="text-xs text-text-secondary">
              稳定性: {result.download.stability}%
            </p>
          </div>
          <div className="mt-2">
            <LinearProgress
              variant="determinate"
              value={Math.min((result.download.speed / 100) * 100, 100)}
              className="h-1.5"
            />
          </div>
        </div>

        {/* 上传速度 */}
        <div className="p-3 bg-surface-variant rounded">
          <div className="flex justify-between items-center mb-2">
            <p className="text-sm text-text-secondary">上传速度</p>
            <span className={`text-xs font-medium ${uploadGrade.color}`}>
              {uploadGrade.label}
            </span>
          </div>
          <div className="flex items-baseline gap-2">
            <p className="text-2xl font-bold">
              {formatSpeed(result.upload.speed)}
            </p>
            <p className="text-xs text-text-secondary">
              稳定性: {result.upload.stability}%
            </p>
          </div>
          <div className="mt-2">
            <LinearProgress
              variant="determinate"
              value={Math.min((result.upload.speed / 100) * 100, 100)}
              className="h-1.5"
            />
          </div>
        </div>

        {/* 延迟和丢包 */}
        <div className="grid grid-cols-2 gap-2">
          {/* 延迟 */}
          <div className="p-2 bg-surface-variant rounded">
            <p className="text-xs text-text-secondary mb-1">延迟</p>
            <p className={`text-lg font-bold ${latencyGrade.color}`}>
              {formatLatency(result.latency.avg)}
            </p>
            <p className="text-xs text-text-secondary mt-1">
              抖动: {formatLatency(result.latency.jitter)}
            </p>
          </div>

          {/* 丢包率 */}
          <div className="p-2 bg-surface-variant rounded">
            <p className="text-xs text-text-secondary mb-1">丢包率</p>
            <p className={`text-lg font-bold ${packetLossGrade.color}`}>
              {result.packetLoss.lossRate.toFixed(2)}%
            </p>
            <p className="text-xs text-text-secondary mt-1">
              {result.packetLoss.received}/{result.packetLoss.sent}
            </p>
          </div>
        </div>

        {/* 重新测试按钮 */}
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

  // 初始状态
  return (
    <div className="flex flex-col items-center justify-center py-6">
      <Gauge className="mb-2 h-10 w-10 text-primary" />
      <p className="text-base text-text-secondary mb-4">
        点击下方按钮开始测试网络速度
      </p>
      <Button onClick={onStart} startIcon={<Play className="h-4 w-4" />}>
        开始测试
      </Button>
    </div>
  )
}

function getPhaseLabel(phase: SpeedTestProgress['phase']): string {
  switch (phase) {
    case 'download':
      return '下载速度测试'
    case 'upload':
      return '上传速度测试'
    case 'latency':
      return '延迟测试'
    case 'packet-loss':
      return '丢包测试'
    case 'complete':
      return '测试完成'
    default:
      return '测试中'
  }
}