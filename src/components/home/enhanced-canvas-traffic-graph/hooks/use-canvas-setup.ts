import { useCallback, useRef } from 'react'

import { clearCanvas, syncCanvasSize } from '../utils/graph-helpers'

/**
 * Canvas 设置和管理 Hook
 * 处理 Canvas 尺寸同步、清除等操作
 */
export const useCanvasSetup = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const hoverCanvasRef = useRef<HTMLCanvasElement>(null)

  /**
   * 同步 Canvas 尺寸
   */
  const syncCanvas = useCallback((canvas: HTMLCanvasElement) => {
    return syncCanvasSize(canvas)
  }, [])

  /**
   * 清除 Canvas
   */
  const clearCanvasContent = useCallback((canvas: HTMLCanvasElement | null) => {
    clearCanvas(canvas)
  }, [])

  /**
   * 清除所有 Canvas
   */
  const clearAllCanvas = useCallback(() => {
    clearCanvas(canvasRef.current)
    clearCanvas(hoverCanvasRef.current)
  }, [])

  return {
    canvasRef,
    hoverCanvasRef,
    syncCanvas,
    clearCanvasContent,
    clearAllCanvas,
  }
}
