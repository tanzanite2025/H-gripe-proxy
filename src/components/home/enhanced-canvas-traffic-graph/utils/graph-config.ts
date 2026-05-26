/**
 * 图表配置常量
 */

export const MAX_POINTS = 300
export const TARGET_FPS = 15 // 降低帧率减少闪烁
export const STALE_DATA_THRESHOLD = 2500 // ms without fresh data => drop FPS

export const LINE_WIDTH = {
  UP: 2.5,
  DOWN: 2.5,
  GRID: 0.5,
} as const

export const ALPHA = {
  GRADIENT: 0.15, // 降低渐变透明度
  LINE: 0.9,
} as const

export const PADDING = {
  TOP: 16,
  RIGHT: 16, // 增加右边距确保时间戳完整显示
  BOTTOM: 32, // 进一步增加底部空间给时间轴和统计信息
  LEFT: 35, // 增加左边距为Y轴标签留出空间
} as const

export const GRAPH_CONFIG = {
  maxPoints: MAX_POINTS,
  targetFPS: TARGET_FPS,
  lineWidth: LINE_WIDTH,
  alpha: ALPHA,
  padding: PADDING,
} as const

export type TimeRange = 1 | 5 | 10 // 分钟
export type ChartStyle = 'bezier' | 'line'
