import { Box, useTheme } from '@mui/material'

import type { TooltipData } from '../hooks/use-graph-interaction'

interface GraphTooltipProps {
  tooltipData: TooltipData
}

/**
 * 图表悬浮提示框组件
 */
export const GraphTooltip = ({ tooltipData }: GraphTooltipProps) => {
  const theme = useTheme()

  if (!tooltipData.visible) return null

  return (
    <Box
      sx={{
        position: 'absolute',
        left: tooltipData.x + 8,
        top: tooltipData.y - 8,
        bgcolor: theme.palette.background.paper,
        border: 1,
        borderColor: 'divider',
        borderRadius: 0.5,
        px: 1,
        py: 0.5,
        fontSize: '10px',
        lineHeight: 1.2,
        zIndex: 1000,
        pointerEvents: 'none',
        transform:
          tooltipData.x > 200 ? 'translateX(-100%)' : 'translateX(0)',
        boxShadow: '0 4px 12px rgba(0,0,0,0.15)',
        backdropFilter: 'none',
        opacity: 1,
        whiteSpace: 'nowrap',
      }}
    >
      <Box sx={{ color: 'text.secondary', mb: 0.2 }}>
        {tooltipData.timestamp}
      </Box>
      <Box sx={{ color: 'secondary.main', fontWeight: 500 }}>
        ↑ {tooltipData.upSpeed}
      </Box>
      <Box sx={{ color: 'primary.main', fontWeight: 500 }}>
        ↓ {tooltipData.downSpeed}
      </Box>
    </Box>
  )
}
