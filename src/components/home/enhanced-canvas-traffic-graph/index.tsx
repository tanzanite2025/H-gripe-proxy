import { Box } from '@mui/material'
import type { Ref } from 'react'
import {
  memo,
  useCallback,
  useEffect,
  useImperativeHandle,
  useRef,
  useState,
} from 'react'

import { useVerge } from '@/hooks/system'
import { debugLog } from '@/utils/misc'

import { GraphOverlay } from './components/graph-overlay'
import { GraphTooltip } from './components/graph-tooltip'
import { useCanvasSetup } from './hooks/use-canvas-setup'
import { useGraphData } from './hooks/use-graph-data'
import { useGraphInteraction } from './hooks/use-graph-interaction'
import { useGraphRenderer } from './hooks/use-graph-renderer'
import type { ChartStyle, TimeRange } from './utils/graph-config'
import { TARGET_FPS } from './utils/graph-config'

// 流量数据项接口
interface ITrafficItem {
  up: number
  down: number
  timestamp?: number
}

// 对外暴露的接口
export interface EnhancedCanvasTrafficGraphRef {
  appendData: (data: ITrafficItem) => void
  toggleStyle: () => void
}

interface EnhancedCanvasTrafficGraphProps {
  ref?: Ref<EnhancedCanvasTrafficGraphRef>
}

/**
 * 增强版 Canvas 流量图表组件
 * 修复闪烁问题，添加时间轴显示
 */
export const EnhancedCanvasTrafficGraph = memo(
  function EnhancedCanvasTrafficGraph({
    ref,
  }: EnhancedCanvasTrafficGraphProps) {
    const verge = useVerge()
    const pauseRenderOnBlur =
      verge.verge?.pause_render_traffic_stats_on_blur ?? true

    // 基础状态
    const [timeRange, setTimeRange] = useState<TimeRange>(10)
    const [chartStyle, setChartStyle] = useState<ChartStyle>('bezier')

    // 窗口焦点状态
    const initialFocusState =
      typeof document !== 'undefined' ? !document.hidden : true
    const [isWindowFocused, setIsWindowFocused] = useState(initialFocusState)
    const [isDocumentVisible, setIsDocumentVisible] =
      useState(initialFocusState)
    const isWindowFocusedRef = useRef<boolean>(initialFocusState)
    const isDocumentVisibleRef = useRef(initialFocusState)

    // Canvas 设置
    const { canvasRef, hoverCanvasRef } = useCanvasSetup()

    // 数据管理
    const {
      displayData,
      yScale,
      samplerStats,
      currentFPS,
      setCurrentFPS,
      lastDataTimestampRef,
      dataStaleRef,
    } = useGraphData(timeRange)

    // 交互处理
    const {
      tooltipData,
      tooltipDataRef,
      handleMouseMove,
      handleMouseLeave,
      cleanup: cleanupInteraction,
    } = useGraphInteraction(displayData, yScale, canvasRef)

    // 渲染调度
    const { scheduleDrawGraph, colors } = useGraphRenderer({
      displayData,
      yScale,
      chartStyle,
      timeRange,
      tooltipData,
      tooltipDataRef,
      canvasRef,
      hoverCanvasRef,
      isWindowFocused,
      isDocumentVisible,
      pauseRenderOnBlur,
      lastDataTimestampRef,
      dataStaleRef,
    })

    // 处理焦点状态变化
    const handleFocusStateChange = useCallback(
      (focused: boolean) => {
        isWindowFocusedRef.current = focused
        setIsWindowFocused(focused)

        if (focused || !pauseRenderOnBlur) {
          setCurrentFPS(TARGET_FPS)
        }
      },
      [pauseRenderOnBlur, setCurrentFPS],
    )

    // 监听窗口焦点变化
    useEffect(() => {
      if (typeof window === 'undefined' || typeof document === 'undefined') {
        return
      }

      const handleFocus = () => handleFocusStateChange(true)
      const handleBlur = () => handleFocusStateChange(false)
      const handleVisibilityChange = () => {
        const visible = !document.hidden
        isDocumentVisibleRef.current = visible
        setIsDocumentVisible(visible)
        handleFocusStateChange(visible)
      }

      window.addEventListener('focus', handleFocus)
      window.addEventListener('blur', handleBlur)
      document.addEventListener('visibilitychange', handleVisibilityChange)

      return () => {
        window.removeEventListener('focus', handleFocus)
        window.removeEventListener('blur', handleBlur)
        document.removeEventListener('visibilitychange', handleVisibilityChange)
      }
    }, [handleFocusStateChange])

    // 清理交互
    useEffect(() => {
      return () => {
        cleanupInteraction()
      }
    }, [cleanupInteraction])

    // 切换时间范围
    const handleTimeRangeClick = useCallback((event: React.MouseEvent) => {
      event.stopPropagation()
      setTimeRange((prev) => {
        return prev === 1 ? 5 : prev === 5 ? 10 : 1
      })
    }, [])

    // 切换图表样式
    const toggleStyle = useCallback(() => {
      setChartStyle((prev) => (prev === 'bezier' ? 'line' : 'bezier'))
    }, [])

    // 兼容性方法
    const appendData = useCallback((data: ITrafficItem) => {
      debugLog(
        '[EnhancedCanvasTrafficGraph] appendData called (using global data):',
        data,
      )
    }, [])

    // 暴露方法给父组件
    useImperativeHandle(
      ref,
      () => ({
        appendData,
        toggleStyle,
      }),
      [appendData, toggleStyle],
    )

    return (
      <Box
        sx={{
          width: '100%',
          height: '100%',
          position: 'relative',
          bgcolor: 'action.hover',
          borderRadius: 1,
          cursor: 'pointer',
          overflow: 'hidden',
        }}
        onClick={toggleStyle}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
      >
        {/* 主 Canvas */}
        <canvas
          ref={canvasRef}
          style={{
            width: '100%',
            height: '100%',
            display: 'block',
          }}
          onClick={toggleStyle}
        />

        {/* 悬浮层 Canvas */}
        {tooltipData.visible && (
          <canvas
            ref={hoverCanvasRef}
            style={{
              position: 'absolute',
              inset: 0,
              width: '100%',
              height: '100%',
              display: 'block',
              pointerEvents: 'none',
            }}
          />
        )}

        {/* 覆盖层 */}
        <GraphOverlay
          timeRange={timeRange}
          chartStyle={chartStyle}
          displayDataLength={displayData.length}
          compressedBufferSize={samplerStats.compressedBufferSize}
          currentFPS={currentFPS}
          colors={{ up: colors.up, down: colors.down }}
          onTimeRangeClick={handleTimeRangeClick}
        />

        {/* 悬浮提示框 */}
        <GraphTooltip tooltipData={tooltipData} />
      </Box>
    )
  },
)

EnhancedCanvasTrafficGraph.displayName = 'EnhancedCanvasTrafficGraph'
