import { AlertCircle, Bug, RefreshCw } from 'lucide-react'
import { Button, Alert, Collapse } from '@/components/tailwind'
import React, { Component, ErrorInfo, ReactNode } from 'react'
import { useTranslation } from 'react-i18next'

interface Props {
  children: ReactNode
  fallbackComponent?: ReactNode
  onError?: (error: Error, errorInfo: ErrorInfo) => void
}

interface State {
  hasError: boolean
  error: Error | null
  errorInfo: ErrorInfo | null
  showDetails: boolean
}

/**
 * 流量统计专用错误边界组件
 * 处理图表和流量统计组件的错误，提供优雅的降级体验
 */
export class TrafficErrorBoundary extends Component<Props, State> {
  private retryCount = 0
  private maxRetries = 3

  constructor(props: Props) {
    super(props)
    this.state = {
      hasError: false,
      error: null,
      errorInfo: null,
      showDetails: false,
    }
  }

  static getDerivedStateFromError(error: Error): Partial<State> {
    // 更新状态以显示降级UI
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('[TrafficErrorBoundary] 捕获到组件错误:', error, errorInfo)

    this.setState({
      error,
      errorInfo,
    })

    // 调用错误回调
    if (this.props.onError) {
      this.props.onError(error, errorInfo)
    }

    // 发送错误到监控系统（如果有的话）
    this.reportError(error, errorInfo)
  }

  private reportError = (error: Error, errorInfo: ErrorInfo) => {
    // 这里可以集成错误监控服务
    const errorReport = {
      message: error.message,
      stack: error.stack,
      componentStack: errorInfo.componentStack,
      timestamp: new Date().toISOString(),
      userAgent: navigator.userAgent,
      url: window.location.href,
    }

    console.error('[TrafficErrorBoundary] 错误报告:', errorReport)
    // TODO: 发送到错误监控服务
    // sendErrorReport(errorReport);
  }

  private handleRetry = () => {
    if (this.retryCount < this.maxRetries) {
      this.retryCount++
      console.log(
        `[TrafficErrorBoundary] 尝试重试 (${this.retryCount}/${this.maxRetries})`,
      )

      this.setState({
        hasError: false,
        error: null,
        errorInfo: null,
        showDetails: false,
      })
    } else {
      console.warn('[TrafficErrorBoundary] 已达到最大重试次数')
    }
  }

  private handleRefresh = () => {
    window.location.reload()
  }

  private toggleDetails = () => {
    this.setState((prev) => ({ showDetails: !prev.showDetails }))
  }

  render() {
    if (this.state.hasError) {
      // 如果提供了自定义降级组件，使用它
      if (this.props.fallbackComponent) {
        return this.props.fallbackComponent
      }

      // 默认错误UI
      return (
        <TrafficErrorFallback
          error={this.state.error}
          errorInfo={this.state.errorInfo}
          showDetails={this.state.showDetails}
          canRetry={this.retryCount < this.maxRetries}
          retryCount={this.retryCount}
          maxRetries={this.maxRetries}
          onRetry={this.handleRetry}
          onRefresh={this.handleRefresh}
          onToggleDetails={this.toggleDetails}
        />
      )
    }

    return this.props.children
  }
}

/**
 * 错误降级UI组件
 */
interface TrafficErrorFallbackProps {
  error: Error | null
  errorInfo: ErrorInfo | null
  showDetails: boolean
  canRetry: boolean
  retryCount: number
  maxRetries: number
  onRetry: () => void
  onRefresh: () => void
  onToggleDetails: () => void
}

const TrafficErrorFallback: React.FC<TrafficErrorFallbackProps> = ({
  error,
  errorInfo,
  showDetails,
  canRetry,
  retryCount,
  maxRetries,
  onRetry,
  onRefresh,
  onToggleDetails,
}) => {
  const { t } = useTranslation()

  return (
    <div className="p-4 min-h-[200px] flex flex-col items-center justify-center border-2 border-dashed border-red-500 rounded-lg bg-red-50 dark:bg-red-950/20">
      <AlertCircle className="w-12 h-12 mb-4 text-red-500" />

      <h3 className="text-lg font-semibold mb-2">
        {t('shared.feedback.errors.trafficStats')}
      </h3>

      <p className="text-sm text-muted-foreground mb-4 text-center">
        {t('shared.feedback.errors.trafficStatsDescription')}
      </p>

      <Alert variant="destructive" className="mb-4 max-w-md">
        <p className="text-sm">
          <strong>Error:</strong>{' '}
          {error instanceof Error ? error.message : 'Unknown error'}
        </p>
        {retryCount > 0 && (
          <p className="text-xs mt-2">
            {t('shared.labels.retryAttempts')}: {retryCount}/{maxRetries}
          </p>
        )}
      </Alert>

      <div className="flex gap-2 mb-4">
        {canRetry && (
          <Button variant="default" size="sm" onClick={onRetry}>
            <RefreshCw className="w-4 h-4 mr-1" />
            {t('shared.actions.retry')}
          </Button>
        )}

        <Button variant="outline" size="sm" onClick={onRefresh}>
          {t('shared.actions.refreshPage')}
        </Button>

        <Button variant="ghost" size="sm" onClick={onToggleDetails}>
          <Bug className="w-4 h-4 mr-1" />
          {showDetails
            ? t('shared.actions.hideDetails')
            : t('shared.actions.showDetails')}
        </Button>
      </div>

      <Collapse open={showDetails} className="w-full max-w-2xl">
        <div className="p-4 bg-card border border-border rounded-lg">
          <h4 className="text-sm font-semibold mb-2">Error Details:</h4>
          <pre className="text-xs font-mono text-muted-foreground whitespace-pre-wrap break-words">
            {error?.stack}
          </pre>

          {errorInfo?.componentStack && (
            <>
              <h4 className="text-sm font-semibold mb-2 mt-4">Component Stack:</h4>
              <pre className="text-xs font-mono text-muted-foreground whitespace-pre-wrap break-words">
                {errorInfo.componentStack}
              </pre>
            </>
          )}
        </div>
      </Collapse>
    </div>
  )
}

/**
 * 轻量级流量统计错误边界
 * 用于小型流量显示组件，提供最小化的错误UI
 */
export const LightweightTrafficErrorBoundary: React.FC<{
  children: ReactNode
}> = ({ children }) => {
  return (
    <TrafficErrorBoundary
      fallbackComponent={
        <div className="p-2 flex items-center justify-center min-h-[60px] bg-red-50 dark:bg-red-950/20 rounded text-red-600 dark:text-red-400">
          <AlertCircle className="w-5 h-5 mr-2" />
          <span className="text-xs">Traffic data unavailable</span>
        </div>
      }
    >
      {children}
    </TrafficErrorBoundary>
  )
}
