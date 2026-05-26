import { Box } from '@mui/material'
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
    <Box
      sx={{
        position: 'absolute',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        pointerEvents: 'none',
      }}
    >
      {/* 时间范围按钮 */}
      <Box
        component="div"
        onClick={onTimeRangeClick}
        sx={{
          position: 'absolute',
          top: 6,
          left: 40,
          fontSize: '11px',
          fontWeight: 'bold',
          color: 'text.secondary',
          cursor: 'pointer',
          pointerEvents: 'all',
          px: 1,
          py: 0.5,
          borderRadius: 0.5,
          bgcolor: 'rgba(0,0,0,0.05)',
          '&:hover': {
            bgcolor: 'rgba(0,0,0,0.1)',
          },
        }}
      >
        {getTimeRangeText()}
      </Box>

      {/* 图例 */}
      <Box
        sx={{
          position: 'absolute',
          top: 6,
          right: 8,
          display: 'flex',
          flexDirection: 'column',
          gap: 0.5,
        }}
      >
        <Box
          sx={{
            fontSize: '11px',
            fontWeight: 'bold',
            color: colors.up,
            textAlign: 'right',
          }}
        >
          {t('home.components.traffic.legends.upload')}
        </Box>
        <Box
          sx={{
            fontSize: '11px',
            fontWeight: 'bold',
            color: colors.down,
            textAlign: 'right',
          }}
        >
          {t('home.components.traffic.legends.download')}
        </Box>
      </Box>

      {/* 样式指示器 */}
      <Box
        sx={{
          position: 'absolute',
          bottom: 6,
          right: 8,
          fontSize: '10px',
          color: 'text.disabled',
          opacity: 0.7,
        }}
      >
        {chartStyle === 'bezier' ? 'Smooth' : 'Linear'}
      </Box>

      {/* 数据统计指示器（左下角） */}
      <Box
        sx={{
          position: 'absolute',
          bottom: 6,
          left: 8,
          fontSize: '9px',
          color: 'text.disabled',
          opacity: 0.6,
          lineHeight: 1.2,
        }}
      >
        Points: {displayDataLength} | Compressed: {compressedBufferSize} | FPS:{' '}
        {currentFPS}
      </Box>
    </Box>
  )
}
