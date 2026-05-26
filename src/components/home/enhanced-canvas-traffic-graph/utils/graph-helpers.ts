import { PADDING } from './graph-config'

/**
 * 图表工具函数
 */

/**
 * Y轴坐标计算 - 线性映射
 */
export function calculateY(
  value: number,
  height: number,
  topValue: number,
  bottomValue: number,
): number {
  const topY = PADDING.TOP + 10
  const bottomY = height - PADDING.BOTTOM - 5

  if (topValue === bottomValue) return bottomY

  const ratio = (value - bottomValue) / (topValue - bottomValue)
  return bottomY - ratio * (bottomY - topY)
}

/**
 * 计算Y轴刻度范围
 */
export function computeYScale(
  data: ITrafficDataPoint[],
): { topValue: number; bottomValue: number } {
  if (data.length === 0) return { topValue: 1024, bottomValue: 0 }

  let maxValue = 0
  let minValue = Infinity
  for (let i = 0; i < data.length; i++) {
    const up = data[i].up
    const down = data[i].down
    if (up > maxValue) maxValue = up
    if (down > maxValue) maxValue = down
    if (up < minValue) minValue = up
    if (down < minValue) minValue = down
  }
  if (!isFinite(minValue)) minValue = 0

  if (maxValue === 0) return { topValue: 1024, bottomValue: 0 }

  const range = maxValue - minValue
  if (range === 0) return { topValue: maxValue * 1.2, bottomValue: 0 }

  const pct = 0.1
  return {
    topValue: maxValue + range * pct,
    bottomValue: Math.max(0, minValue - range * pct),
  }
}

/**
 * 格式化流量数值
 */
export function formatTrafficValue(bytes: number): string {
  if (bytes === 0) return '0'
  if (bytes < 1024) return `${Math.round(bytes)}B`
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)}KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)}MB`
}

/**
 * 同步 Canvas 尺寸
 */
export function syncCanvasSize(canvas: HTMLCanvasElement): {
  ctx: CanvasRenderingContext2D
  cssWidth: number
  cssHeight: number
} | null {
  const ctx = canvas.getContext('2d')
  if (!ctx) return null

  const rect = canvas.getBoundingClientRect()
  const dpr = window.devicePixelRatio || 1
  const cssWidth = rect.width
  const cssHeight = rect.height
  const pixelWidth = Math.max(1, Math.floor(cssWidth * dpr))
  const pixelHeight = Math.max(1, Math.floor(cssHeight * dpr))

  if (canvas.style.width !== '100%') {
    canvas.style.width = '100%'
  }
  if (canvas.style.height !== '100%') {
    canvas.style.height = '100%'
  }

  if (canvas.width !== pixelWidth || canvas.height !== pixelHeight) {
    canvas.width = pixelWidth
    canvas.height = pixelHeight
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.scale(dpr, dpr)
  }

  return { ctx, cssWidth, cssHeight }
}

/**
 * 清除 Canvas
 */
export function clearCanvas(canvas: HTMLCanvasElement | null): void {
  if (!canvas) return
  const synced = syncCanvasSize(canvas)
  if (!synced) return
  synced.ctx.clearRect(0, 0, synced.cssWidth, synced.cssHeight)
}

/**
 * 检查两个数据数组是否相同
 */
export function isSameTrafficData(
  current: ITrafficDataPoint[],
  next: ITrafficDataPoint[],
): boolean {
  if (current === next) return true
  if (current.length !== next.length) return false

  for (let i = 0; i < current.length; i++) {
    const currentPoint = current[i]
    const nextPoint = next[i]

    if (
      currentPoint.timestamp !== nextPoint.timestamp ||
      currentPoint.up !== nextPoint.up ||
      currentPoint.down !== nextPoint.down
    ) {
      return false
    }
  }

  return true
}
