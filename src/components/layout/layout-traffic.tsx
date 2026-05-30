import { useEffect, useRef } from 'react'
import { useTranslation } from 'react-i18next'

import { LightweightTrafficErrorBoundary } from '@/components/ui/traffic-error-boundary'
import { useMemoryData, useTrafficData } from '@/hooks/data'
import { useVerge } from '@/hooks/system'
import { useVisibility } from '@/hooks/ui'
import { cn } from '@/utils/cn'
import parseTraffic from '@/utils/format'

import { TrafficGraph, type TrafficRef } from './traffic-graph'

interface LayoutTrafficProps {
  horizontal?: boolean
}

export const LayoutTraffic = ({ horizontal = false }: LayoutTrafficProps) => {
  const { t } = useTranslation()
  const { verge } = useVerge()

  // whether hide traffic graph
  const trafficGraph = verge?.traffic_graph ?? true

  const trafficRef = useRef<TrafficRef>(null)
  const pageVisible = useVisibility()

  const {
    response: { data: traffic },
  } = useTrafficData({ enabled: trafficGraph && pageVisible && !horizontal })
  const {
    response: { data: memory },
  } = useMemoryData()

  // 监听数据变化，为图表添加数据点
  useEffect(() => {
    if (trafficRef.current && !horizontal) {
      trafficRef.current.appendData({
        up: traffic?.up || 0,
        down: traffic?.down || 0,
      })
    }
  }, [traffic, horizontal])

  // 显示内存使用情况的设置
  const displayMemory = verge?.enable_memory_usage ?? true

  // 使用parseTraffic统一处理转换，保持与首页一致的显示格式
  const [up, upUnit] = parseTraffic(traffic?.up || 0)
  const [down, downUnit] = parseTraffic(traffic?.down || 0)
  const [inuse, inuseUnit] = parseTraffic(memory?.inuse || 0)

  if (horizontal) {
    return (
      <LightweightTrafficErrorBoundary>
        <div className="flex flex-row items-center gap-4 select-none">
          {/* 上传速度 */}
          <div className="flex items-center gap-1">
            <span
              className={cn(
                'uds-mono text-sm font-semibold',
                (traffic?.up || 0) > 0
                  ? 'text-purple-500 opacity-100'
                  : 'text-gray-500 opacity-60',
              )}
            >
              {up}
              {upUnit}/s
            </span>
          </div>

          {/* 下载速度 */}
          <div className="flex items-center gap-1">
            <span
              className={cn(
                'uds-mono text-sm font-semibold',
                (traffic?.down || 0) > 0
                  ? 'text-blue-500 opacity-100'
                  : 'text-gray-500 opacity-60',
              )}
            >
              {down}
              {downUnit}/s
            </span>
          </div>

          {/* 内存占用 */}
          {displayMemory && (
            <div className="flex items-center gap-1">
              <span className="uds-mono text-sm font-semibold text-gray-500 opacity-60">
                {inuse}
                {inuseUnit}
              </span>
            </div>
          )}
        </div>
      </LightweightTrafficErrorBoundary>
    )
  }

  return (
    <LightweightTrafficErrorBoundary>
      <div className="relative">
        {trafficGraph && pageVisible && (
          <div
            style={{ width: '100%', height: 60, marginBottom: 6 }}
            onClick={trafficRef.current?.toggleStyle}
          >
            <TrafficGraph ref={trafficRef} />
          </div>
        )}

        <div className="flex flex-col gap-1.5">
          <div
            title={`${t('home.components.traffic.metrics.uploadSpeed')}`}
            className="flex items-center whitespace-nowrap"
          >
            <span className="uds-mono flex-1 basis-14 select-none text-center text-purple-500">
              {up}
            </span>
            <span className="uds-mono flex-none basis-7 select-none text-right text-xs text-gray-500">
              {upUnit}/s
            </span>
          </div>

          <div
            title={`${t('home.components.traffic.metrics.downloadSpeed')}`}
            className="flex items-center whitespace-nowrap"
          >
            <span className="uds-mono flex-1 basis-14 select-none text-center text-blue-500">
              {down}
            </span>
            <span className="uds-mono flex-none basis-7 select-none text-right text-xs text-gray-500">
              {downUnit}/s
            </span>
          </div>

          {displayMemory && (
            <div
              title={`${t('home.components.traffic.metrics.memoryUsage')} `}
              className="flex cursor-auto items-center whitespace-nowrap"
              onClick={async () => {
                // isDebug && (await gc());
              }}
            >
              <span className="uds-mono flex-1 basis-14 select-none text-center">
                {inuse}
              </span>
              <span className="uds-mono flex-none basis-7 select-none text-right text-xs text-gray-500">
                {inuseUnit}
              </span>
            </div>
          )}
        </div>
      </div>
    </LightweightTrafficErrorBoundary>
  )
}
