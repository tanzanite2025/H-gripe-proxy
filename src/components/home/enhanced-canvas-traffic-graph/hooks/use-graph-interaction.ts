import React, { useCallback, useRef, useState } from 'react'

import parseTraffic from '@/utils/format'
import { formatTrafficName } from '@/utils/network/traffic-sampler'

import { PADDING } from '../utils/graph-config'
import { calculateY } from '../utils/graph-helpers'

/**
 * 悬浮提示数据接口
 */
export interface TooltipData {
  x: number
  y: number
  upSpeed: string
  downSpeed: string
  timestamp: string
  visible: boolean
  dataIndex: number
  highlightY: number
}

/**
 * 图表交互处理 Hook
 * 处理鼠标悬浮、点击等交互
 */
export const useGraphInteraction = (
  displayData: ITrafficDataPoint[],
  yScale: { topValue: number; bottomValue: number },
  canvasRef: React.RefObject<HTMLCanvasElement | null>,
) => {
  // 悬浮提示状态
  const [tooltipData, setTooltipData] = useState<TooltipData>({
    x: 0,
    y: 0,
    upSpeed: '',
    downSpeed: '',
    timestamp: '',
    visible: false,
    dataIndex: -1,
    highlightY: 0,
  })
  const tooltipDataRef = useRef<TooltipData>(tooltipData)

  // 鼠标移动帧请求
  const mouseMoveFrameRef = useRef<number | undefined>(undefined)
  const pendingMousePositionRef = useRef<{
    clientX: number
    clientY: number
  } | null>(null)

  // 鼠标悬浮处理 - 计算最近的数据点
  const handleMouseMove = useCallback(
    (event: React.MouseEvent<HTMLElement>) => {
      if (displayData.length === 0) return

      pendingMousePositionRef.current = {
        clientX: event.clientX,
        clientY: event.clientY,
      }

      if (mouseMoveFrameRef.current !== undefined) return

      mouseMoveFrameRef.current = requestAnimationFrame(() => {
        mouseMoveFrameRef.current = undefined

        const pendingMousePosition = pendingMousePositionRef.current
        pendingMousePositionRef.current = null
        if (!pendingMousePosition) return

        const canvas = canvasRef.current
        if (!canvas || displayData.length === 0) return

        const rect = canvas.getBoundingClientRect()
        const mouseX = pendingMousePosition.clientX - rect.left
        const mouseY = pendingMousePosition.clientY - rect.top

        const effectiveWidth = rect.width - PADDING.LEFT - PADDING.RIGHT
        if (effectiveWidth <= 0) return

        // 计算最接近的数据点索引
        const relativeMouseX = mouseX - PADDING.LEFT
        const ratio = Math.max(0, Math.min(1, relativeMouseX / effectiveWidth))
        const dataIndex = Math.round(ratio * (displayData.length - 1))

        if (dataIndex < 0 || dataIndex >= displayData.length) return

        const dataPoint = displayData[dataIndex]

        // 格式化流量数据
        const [upValue, upUnit] = parseTraffic(dataPoint.up)
        const [downValue, downUnit] = parseTraffic(dataPoint.down)

        // 格式化时间戳
        const timeStr = dataPoint.timestamp
          ? formatTrafficName(dataPoint.timestamp)
          : '未知时间'

        // 计算数据点对应的Y坐标位置（用于高亮）
        const { topValue, bottomValue } = yScale
        const upY = calculateY(dataPoint.up, rect.height, topValue, bottomValue)
        const downY = calculateY(
          dataPoint.down,
          rect.height,
          topValue,
          bottomValue,
        )
        const highlightY =
          Math.max(dataPoint.up, dataPoint.down) === dataPoint.up ? upY : downY

        const nextTooltipData = {
          x: mouseX,
          y: mouseY,
          upSpeed: `${upValue}${upUnit}/s`,
          downSpeed: `${downValue}${downUnit}/s`,
          timestamp: timeStr,
          visible: true,
          dataIndex,
          highlightY,
        }

        setTooltipData((prev) => {
          if (
            prev.visible &&
            prev.dataIndex === nextTooltipData.dataIndex &&
            Math.abs(prev.x - nextTooltipData.x) < 1 &&
            Math.abs(prev.y - nextTooltipData.y) < 1 &&
            Math.abs(prev.highlightY - nextTooltipData.highlightY) < 1 &&
            prev.upSpeed === nextTooltipData.upSpeed &&
            prev.downSpeed === nextTooltipData.downSpeed &&
            prev.timestamp === nextTooltipData.timestamp
          ) {
            return prev
          }

          return nextTooltipData
        })
      })
    },
    [displayData, yScale, canvasRef],
  )

  // 鼠标离开处理
  const handleMouseLeave = useCallback(() => {
    pendingMousePositionRef.current = null

    if (mouseMoveFrameRef.current !== undefined) {
      cancelAnimationFrame(mouseMoveFrameRef.current)
      mouseMoveFrameRef.current = undefined
    }

    setTooltipData((prev) => (prev.visible ? { ...prev, visible: false } : prev))
  }, [])

  // 清理函数
  const cleanup = useCallback(() => {
    if (mouseMoveFrameRef.current !== undefined) {
      cancelAnimationFrame(mouseMoveFrameRef.current)
      mouseMoveFrameRef.current = undefined
    }
  }, [])

  return {
    tooltipData,
    tooltipDataRef,
    handleMouseMove,
    handleMouseLeave,
    cleanup,
  }
}
