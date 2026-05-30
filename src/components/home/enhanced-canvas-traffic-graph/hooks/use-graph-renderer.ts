import { useMemo, useCallback, useEffect, useRef } from 'react'

import { defaultDarkTheme } from '@/pages/_core/theme'

import { drawYAxis, drawTimeAxis } from '../renderers/axis-renderer'
import { drawGrid } from '../renderers/grid-renderer'
import { drawTrafficLine, drawHoverIndicator } from '../renderers/line-renderer'
import type { ChartStyle, TimeRange } from '../utils/graph-config'
import { STALE_DATA_THRESHOLD } from '../utils/graph-config'
import { syncCanvasSize, clearCanvas } from '../utils/graph-helpers'

import type { TooltipData, TooltipDataRef } from './use-graph-interaction'

interface UseGraphRendererProps {
  displayData: ITrafficDataPoint[]
  yScale: { topValue: number; bottomValue: number }
  chartStyle: ChartStyle
  timeRange: TimeRange
  tooltipData: TooltipData
  tooltipDataRef: TooltipDataRef
  canvasRef: React.RefObject<HTMLCanvasElement | null>
  hoverCanvasRef: React.RefObject<HTMLCanvasElement | null>
  isWindowFocused: boolean
  isDocumentVisible: boolean
  pauseRenderOnBlur: boolean
  lastDataTimestampRef: React.MutableRefObject<number>
  dataStaleRef: React.MutableRefObject<boolean>
}

/**
 * 图表渲染调度 Hook
 * 处理主图表和悬浮层的渲染
 */
export const useGraphRenderer = ({
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
}: UseGraphRendererProps) => {
  const theme = defaultDarkTheme

  // 帧请求引用
  const drawFrameRef = useRef<number | undefined>(undefined)
  const hoverFrameRef = useRef<number | undefined>(undefined)
  const scheduleDrawGraphRef = useRef<() => void>(() => {})

  // 主题颜色配置
  const colors = useMemo(
    () => ({
      up: theme.secondary_color,
      down: theme.primary_color,
      grid: 'rgba(255, 255, 255, 0.08)',
      text: theme.secondary_text,
      background: theme.background_color,
    }),
    [theme.background_color, theme.primary_color, theme.secondary_color, theme.secondary_text],
  )

  // 主绘制函数
  const drawGraph = useCallback(() => {
    const canvas = canvasRef.current
    if (!canvas || displayData.length === 0) {
      clearCanvas(canvasRef.current)
      clearCanvas(hoverCanvasRef.current)
      return
    }

    const synced = syncCanvasSize(canvas)
    if (!synced) return
    const { ctx, cssWidth, cssHeight } = synced

    ctx.clearRect(0, 0, cssWidth, cssHeight)

    const { topValue, bottomValue } = yScale

    // 绘制Y轴刻度线（背景层）
    drawYAxis(ctx, cssWidth, cssHeight, topValue, bottomValue, {
      grid: colors.grid,
      text: colors.text,
      background: colors.background,
    })

    // 绘制网格
    drawGrid(ctx, cssWidth, cssHeight, colors.grid)

    // 绘制时间轴
    drawTimeAxis(ctx, cssWidth, cssHeight, displayData, timeRange, colors.text)

    // 绘制下载线（背景层）
    drawTrafficLine(
      ctx,
      displayData,
      'down',
      cssWidth,
      cssHeight,
      colors.down,
      chartStyle,
      true,
      topValue,
      bottomValue,
    )

    // 绘制上传线（前景层）
    drawTrafficLine(
      ctx,
      displayData,
      'up',
      cssWidth,
      cssHeight,
      colors.up,
      chartStyle,
      true,
      topValue,
      bottomValue,
    )

    clearCanvas(hoverCanvasRef.current)
  }, [
    displayData,
    yScale,
    chartStyle,
    timeRange,
    colors,
    canvasRef,
    hoverCanvasRef,
  ])

  // 绘制悬浮覆盖层
  const drawHoverOverlay = useCallback(() => {
    const canvas = hoverCanvasRef.current
    if (!canvas || displayData.length < 2) {
      clearCanvas(canvas)
      return
    }

    const synced = syncCanvasSize(canvas)
    if (!synced) return
    const { ctx, cssWidth, cssHeight } = synced

    ctx.clearRect(0, 0, cssWidth, cssHeight)

    const currentTooltip = tooltipData
    if (currentTooltip.visible && currentTooltip.dataIndex >= 0) {
      drawHoverIndicator(
        ctx,
        cssWidth,
        cssHeight,
        currentTooltip.dataIndex,
        displayData.length,
        currentTooltip.highlightY,
        colors.text,
      )
    }
  }, [displayData, colors.text, tooltipData, hoverCanvasRef])

  // 检查是否应该跳过绘制
  const shouldSkipGraphDraw = useCallback(() => {
    if (!isDocumentVisible) return true
 
    if (!isWindowFocused && pauseRenderOnBlur) {
      return true
    }
 
    const lastDataTimestamp = lastDataTimestampRef.current
    if (
      lastDataTimestamp > 0 &&
      Date.now() - lastDataTimestamp > STALE_DATA_THRESHOLD
    ) {
      return true
    }
 
    return dataStaleRef.current
  }, [
    isDocumentVisible,
    isWindowFocused,
    pauseRenderOnBlur,
    lastDataTimestampRef,
    dataStaleRef,
  ])

  // 调度悬浮层绘制
  const scheduleHoverDraw = useCallback(() => {
    if (hoverFrameRef.current !== undefined) return

    hoverFrameRef.current = requestAnimationFrame(() => {
      hoverFrameRef.current = undefined
      drawHoverOverlay()
    })
  }, [drawHoverOverlay])

  // 调度主图表绘制
  const scheduleDrawGraph = useCallback(() => {
    if (drawFrameRef.current !== undefined) return

    drawFrameRef.current = requestAnimationFrame(() => {
      drawFrameRef.current = undefined

      if (shouldSkipGraphDraw()) return

      drawGraph()
      drawHoverOverlay()
    })
  }, [drawGraph, drawHoverOverlay, shouldSkipGraphDraw])

  // 更新 tooltip 时重绘悬浮层
  useEffect(() => {
    scheduleHoverDraw()
  }, [tooltipData, scheduleHoverDraw])

  // 窗口状态变化时重绘
  useEffect(() => {
    scheduleDrawGraph()
  }, [scheduleDrawGraph, isDocumentVisible, isWindowFocused])

  // 保存调度函数引用
  useEffect(() => {
    scheduleDrawGraphRef.current = scheduleDrawGraph
  }, [scheduleDrawGraph])

  // 监听窗口大小变化
  useEffect(() => {
    const canvas = canvasRef.current
    if (!canvas || typeof window === 'undefined') return

    if (typeof ResizeObserver === 'undefined') {
      const handleResize = () => scheduleDrawGraphRef.current()
      window.addEventListener('resize', handleResize)
      return () => {
        window.removeEventListener('resize', handleResize)
      }
    }

    const resizeObserver = new ResizeObserver(() =>
      scheduleDrawGraphRef.current(),
    )
    resizeObserver.observe(canvas)

    return () => {
      resizeObserver.disconnect()
    }
  }, [canvasRef])

  // 清理函数
  useEffect(() => {
    return () => {
      const drawId = drawFrameRef.current
      if (drawId !== undefined) {
        cancelAnimationFrame(drawId)
      }
      const hoverId = hoverFrameRef.current
      if (hoverId !== undefined) {
        cancelAnimationFrame(hoverId)
      }
    }
  }, [])

  return {
    colors,
  }
}
