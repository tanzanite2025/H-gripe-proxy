import React from 'react'
import { useTranslation } from 'react-i18next'

import type { ChartStyle, TimeRange } from '../utils/graph-config'

interface GraphOverlayProps {
  timeRange: TimeRange
  chartStyle: ChartStyle
  displayDataLength: number
  compressedBufferSize: number
  currentFPS: number
  colors: {
    up: string
    down: string
  }
  onTimeRangeClick: (event: React.MouseEvent) => void
}

/**
 * 图表覆盖层组件
 * 包含时间范围按钮、图例、样式指示器、数据统计等
 */
export const GraphOverlay = ({
  timeRange,
  chartStyle,
  displayDataLength,
  compressedBufferSize,
  currentFPS,
  colors,
  onTimeRangeClick,
}: GraphOverlayProps) => {
  const { t } = useTranslation()

  const getTimeRangeText = () => {
    return t('home.components.traffic.patterns.minutes', {
      time: timeRange,
    })
  }

  return (
    <div className="pointer-events-none absolute bottom-0 left-0 right-0 top-0">
      {/* 时间范围按钮 */}
      <div
        onClick={onTimeRangeClick}
        className="pointer-events-auto absolute left-10 top-1.5 cursor-pointer rounded bg-black/5 px-1 py-0.5 text-[11px] font-bold text-gray-500 hover:bg-black/10 dark:text-gray-400"
      >
        {getTimeRangeText()}
      </div>

      {/* 图例 */}
      <div className="absolute right-2 top-1.5 flex flex-col gap-0.5">
        <div
          className="text-right text-[11px] font-bold"
          style={{ color: colors.up }}
        >
          {t('home.components.traffic.legends.upload')}
        </div>
        <div
          className="text-right text-[11px] font-bold"
          style={{ color: colors.down }}
        >
          {t('home.components.traffic.legends.download')}
        </div>
      </div>

      {/* 样式指示器 */}
      <div className="absolute bottom-1.5 right-2 text-[10px] text-gray-400 opacity-70">
        {chartStyle === 'bezier' ? 'Smooth' : 'Linear'}
      </div>

      {/* 数据统计指示器（左下角） */}
      <div className="absolute bottom-1.5 left-2 text-[9px] leading-tight text-gray-400 opacity-60">
        Points: {displayDataLength} | Compressed: {compressedBufferSize} | FPS:{' '}
        {currentFPS}
      </div>
    </div>
  )
}
