import {
  ArrowDown,
  ArrowUp,
  Download,
  HardDrive,
  Link2,
  Upload,
} from 'lucide-react'
import { ReactNode, memo, useMemo, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { Paper } from '@/components/tailwind/Paper'
import { TrafficErrorBoundary } from '@/components/ui/traffic-error-boundary'
import { useConnectionData, useMemoryData, useTrafficData } from '@/hooks/data'
import { useVisibility } from '@/hooks/ui'
import { cn } from '@/utils/cn'
import parseTraffic from '@/utils/format'

import {
  EnhancedCanvasTrafficGraph,
  type EnhancedCanvasTrafficGraphRef,
} from './enhanced-canvas-traffic-graph'

interface StatCardProps {
  icon: ReactNode
  title: string
  value: string | number
  unit: string
  color: 'primary' | 'secondary' | 'error' | 'warning' | 'info' | 'success'
  onClick?: () => void
}

// 全局变量类型定义
declare global {
  interface Window {
    animationFrameId?: number
    lastTrafficData?: {
      up: number
      down: number
    }
  }
}

// 颜色映射
const colorMap = {
  primary: 'text-primary',
  secondary: 'text-secondary',
  error: 'text-error',
  warning: 'text-warning',
  info: 'text-info',
  success: 'text-success',
}

const bgColorMap = {
  primary: 'bg-primary/5 border-primary/15 hover:bg-primary/10 hover:border-primary/30',
  secondary: 'bg-secondary/5 border-secondary/15 hover:bg-secondary/10 hover:border-secondary/30',
  error: 'bg-error/5 border-error/15 hover:bg-error/10 hover:border-error/30',
  warning: 'bg-warning/5 border-warning/15 hover:bg-warning/10 hover:border-warning/30',
  info: 'bg-info/5 border-info/15 hover:bg-info/10 hover:border-info/30',
  success: 'bg-success/5 border-success/15 hover:bg-success/10 hover:border-success/30',
}

const iconBgMap = {
  primary: 'bg-primary/10',
  secondary: 'bg-secondary/10',
  error: 'bg-error/10',
  warning: 'bg-warning/10',
  info: 'bg-info/10',
  success: 'bg-success/10',
}

// 统计卡片组件 - 使用memo优化
const CompactStatCard = memo(
  ({ icon, title, value, unit, color, onClick }: StatCardProps) => {
    return (
      <Paper
        elevation={0}
        className={cn(
          'flex items-center justify-center gap-1.5 rounded-lg border px-2 py-1.5 transition-all duration-200',
          bgColorMap[color],
          onClick ? 'cursor-pointer hover:shadow-md' : 'cursor-default'
        )}
        onClick={onClick}
      >
        <div className={cn('flex items-center justify-center w-5 h-5 rounded-full', iconBgMap[color], colorMap[color])}>
          {icon}
        </div>
        <div className="flex items-baseline min-w-0 gap-0.5">
          <span className="text-xs text-text-secondary truncate">{title}</span>
          <span className="text-sm font-bold truncate">{value}</span>
          <span className="text-[10px] text-text-secondary">{unit}</span>
        </div>
      </Paper>
    )
  },
)

// 添加显示名称
CompactStatCard.displayName = 'CompactStatCard'

export const EnhancedTrafficStats = () => {
  const { t } = useTranslation()
  const trafficRef = useRef<EnhancedCanvasTrafficGraphRef>(null)
  const pageVisible = useVisibility()

  // 是否显示流量图表
  const trafficGraph = true

  const {
    response: { data: traffic },
  } = useTrafficData({ enabled: trafficGraph && pageVisible })

  const {
    response: { data: memory },
  } = useMemoryData()

  const {
    response: { data: connections },
  } = useConnectionData()

  // 使用useMemo计算解析后的流量数据
  const parsedData = useMemo(() => {
    const [up, upUnit] = parseTraffic(traffic?.up || 0)
    const [down, downUnit] = parseTraffic(traffic?.down || 0)
    const [inuse, inuseUnit] = parseTraffic(memory?.inuse || 0)
    const [uploadTotal, uploadTotalUnit] = parseTraffic(
      connections?.uploadTotal,
    )
    const [downloadTotal, downloadTotalUnit] = parseTraffic(
      connections?.downloadTotal,
    )

    return {
      up,
      upUnit,
      down,
      downUnit,
      inuse,
      inuseUnit,
      uploadTotal,
      uploadTotalUnit,
      downloadTotal,
      downloadTotalUnit,
      connectionsCount: connections?.activeConnections.length,
    }
  }, [traffic, memory, connections])

  // 渲染流量图表 - 使用useMemo缓存渲染结果
  const trafficGraphComponent = useMemo(() => {
    if (!trafficGraph || !pageVisible) return null

    return (
      <Paper
        elevation={0}
        className="h-[130px] cursor-pointer border border-divider/20 rounded-lg overflow-hidden"
        onClick={() => trafficRef.current?.toggleStyle()}
      >
        <div className="h-full relative">
          <EnhancedCanvasTrafficGraph ref={trafficRef} />
        </div>
      </Paper>
    )
  }, [trafficGraph, pageVisible])

  // 使用useMemo计算统计卡片配置
  const statCards = useMemo(
    () => [
      {
        icon: <ArrowUp className="h-4 w-4" />,
        title: t('home.components.traffic.metrics.uploadSpeed'),
        value: parsedData.up,
        unit: `${parsedData.upUnit}/s`,
        color: 'secondary' as const,
      },
      {
        icon: <ArrowDown className="h-4 w-4" />,
        title: t('home.components.traffic.metrics.downloadSpeed'),
        value: parsedData.down,
        unit: `${parsedData.downUnit}/s`,
        color: 'primary' as const,
      },
      {
        icon: <Link2 className="h-4 w-4" />,
        title: t('home.components.traffic.metrics.activeConnections'),
        value: parsedData.connectionsCount,
        unit: '',
        color: 'success' as const,
      },
      {
        icon: <Upload className="h-4 w-4" />,
        title: t('shared.labels.uploaded'),
        value: parsedData.uploadTotal,
        unit: parsedData.uploadTotalUnit,
        color: 'secondary' as const,
      },
      {
        icon: <Download className="h-4 w-4" />,
        title: t('shared.labels.downloaded'),
        value: parsedData.downloadTotal,
        unit: parsedData.downloadTotalUnit,
        color: 'primary' as const,
      },
      {
        icon: <HardDrive className="h-4 w-4" />,
        title: t('home.components.traffic.metrics.memoryUsage'),
        value: parsedData.inuse,
        unit: parsedData.inuseUnit,
        color: 'error' as const,
        onClick: undefined,
      },
    ],
    [t, parsedData],
  )

  return (
    <TrafficErrorBoundary
      onError={(error, errorInfo) => {
        console.error('[EnhancedTrafficStats] 组件错误:', error, errorInfo)
      }}
    >
      <div className="grid grid-cols-8 sm:grid-cols-8 md:grid-cols-12 gap-2">
        {trafficGraph && (
          <div className="col-span-12">
            {/* 流量图表区域 */}
            {trafficGraphComponent}
          </div>
        )}
        {/* 统计卡片区域 */}
        {statCards.map((card) => (
          <div key={card.title} className="col-span-2">
            <CompactStatCard {...(card as StatCardProps)} />
          </div>
        ))}
      </div>
    </TrafficErrorBoundary>
  )
}
