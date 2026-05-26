import {
  formatTrafficHourMinute,
  formatTrafficMinuteSecond,
} from '@/utils/network/traffic-sampler'

import type { TimeRange } from '../utils/graph-config'
import { PADDING } from '../utils/graph-config'
import { formatTrafficValue } from '../utils/graph-helpers'

/**
 * Y轴刻度接口
 */
interface YAxisTick {
  value: number
  label: string
  y: number
}

/**
 * 获取Y轴刻度（三刻度系统：最小值、中间值、最大值）
 */
export function getYAxisTicks(
  topValue: number,
  bottomValue: number,
  height: number,
): YAxisTick[] {
  const topY = PADDING.TOP + 10 // 避免与顶部时间范围按钮重叠
  const bottomY = height - PADDING.BOTTOM - 5 // 避免与底部时间轴重叠
  const middleY = (topY + bottomY) / 2
  const middleValue = (bottomValue + topValue) / 2

  // 创建三个固定位置的刻度
  return [
    {
      value: bottomValue,
      label: formatTrafficValue(bottomValue),
      y: bottomY,
    },
    {
      value: middleValue,
      label: formatTrafficValue(middleValue),
      y: middleY,
    },
    {
      value: topValue,
      label: formatTrafficValue(topValue),
      y: topY,
    },
  ]
}

/**
 * 绘制Y轴刻度线和标签
 */
export function drawYAxis(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  topValue: number,
  bottomValue: number,
  colors: { grid: string; text: string; background: string },
): void {
  const ticks = getYAxisTicks(topValue, bottomValue, height)

  if (ticks.length === 0) return

  ctx.save()

  ticks.forEach((tick, index) => {
    const isBottomTick = index === 0 // 最底部的刻度
    const isTopTick = index === ticks.length - 1 // 最顶部的刻度

    // 绘制水平刻度线，只绘制关键刻度线
    if (isBottomTick || isTopTick) {
      ctx.strokeStyle = colors.grid
      ctx.lineWidth = isBottomTick ? 0.8 : 0.4 // 底部刻度线稍粗
      ctx.globalAlpha = isBottomTick ? 0.25 : 0.15

      ctx.beginPath()
      ctx.moveTo(PADDING.LEFT, tick.y)
      ctx.lineTo(width - PADDING.RIGHT, tick.y)
      ctx.stroke()
    }

    // 绘制Y轴标签
    ctx.fillStyle = colors.text
    ctx.font =
      "8px -apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif"
    ctx.globalAlpha = 0.9
    ctx.textAlign = 'right'
    ctx.textBaseline = 'middle'

    // 为标签添加更清晰的背景（仅在必要时）
    if (tick.label !== '0') {
      const labelWidth = ctx.measureText(tick.label).width
      ctx.globalAlpha = 0.15
      ctx.fillStyle = colors.background
      ctx.fillRect(
        PADDING.LEFT - labelWidth - 8,
        tick.y - 5,
        labelWidth + 4,
        10,
      )
    }

    // 绘制标签文字
    ctx.globalAlpha = 0.9
    ctx.fillStyle = colors.text
    ctx.fillText(tick.label, PADDING.LEFT - 4, tick.y)
  })

  ctx.restore()
}

/**
 * 时间显示策略接口
 */
interface TimeDisplayStrategy {
  maxLabels: number
  formatTime: (timestamp: number) => string
  intervalSeconds: number
  minPixelDistance: number
}

/**
 * 获取时间范围对应的最佳时间显示策略
 */
export function getTimeDisplayStrategy(
  timeRangeMinutes: TimeRange,
): TimeDisplayStrategy {
  switch (timeRangeMinutes) {
    case 1: // 1分钟：更密集的时间标签，显示 MM:SS
      return {
        maxLabels: 6,
        formatTime: formatTrafficMinuteSecond,
        intervalSeconds: 10,
        minPixelDistance: 35,
      }
    case 5: // 5分钟：中等密度，显示 HH:MM
      return {
        maxLabels: 6,
        formatTime: formatTrafficHourMinute,
        intervalSeconds: 30,
        minPixelDistance: 38,
      }
    case 10: // 10分钟：标准密度，显示 HH:MM
    default:
      return {
        maxLabels: 8,
        formatTime: formatTrafficHourMinute,
        intervalSeconds: 60,
        minPixelDistance: 40,
      }
  }
}

/**
 * 绘制时间轴
 */
export function drawTimeAxis(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  data: ITrafficDataPoint[],
  timeRange: TimeRange,
  textColor: string,
): void {
  if (data.length === 0) return

  const effectiveWidth = width - PADDING.LEFT - PADDING.RIGHT
  const timeAxisY = height - PADDING.BOTTOM + 14

  const strategy = getTimeDisplayStrategy(timeRange)

  ctx.save()
  ctx.fillStyle = textColor
  ctx.font =
    "10px -apple-system, BlinkMacSystemFont, 'Segoe UI', Arial, sans-serif"
  ctx.globalAlpha = 0.7

  // 根据数据长度和时间范围智能选择显示间隔
  const targetLabels = Math.min(strategy.maxLabels, data.length)
  const step = Math.max(1, Math.floor(data.length / (targetLabels - 1)))

  const minPixelDistance = strategy.minPixelDistance
  const actualStep = Math.max(
    step,
    Math.ceil((data.length * minPixelDistance) / effectiveWidth),
  )

  // 收集要显示的时间点
  const timePoints: Array<{ index: number; x: number; label: string }> = []

  // 添加第一个时间点
  if (data.length > 0 && data[0].timestamp) {
    timePoints.push({
      index: 0,
      x: PADDING.LEFT,
      label: strategy.formatTime(data[0].timestamp),
    })
  }

  // 添加中间的时间点
  for (let i = actualStep; i < data.length - actualStep; i += actualStep) {
    const point = data[i]
    if (!point.timestamp) continue

    const x = PADDING.LEFT + (i / (data.length - 1)) * effectiveWidth
    timePoints.push({
      index: i,
      x,
      label: strategy.formatTime(point.timestamp),
    })
  }

  // 添加最后一个时间点（如果不会与前面的重叠）
  if (data.length > 1 && data[data.length - 1].timestamp) {
    const lastX = width - PADDING.RIGHT
    const lastPoint = timePoints[timePoints.length - 1]

    // 确保最后一个标签与前一个标签有足够间距
    if (!lastPoint || lastX - lastPoint.x >= minPixelDistance) {
      timePoints.push({
        index: data.length - 1,
        x: lastX,
        label: strategy.formatTime(data[data.length - 1].timestamp),
      })
    }
  }

  // 绘制时间标签
  timePoints.forEach((point, index) => {
    if (index === 0) {
      // 第一个标签左对齐
      ctx.textAlign = 'left'
    } else if (index === timePoints.length - 1) {
      // 最后一个标签右对齐
      ctx.textAlign = 'right'
    } else {
      // 中间标签居中对齐
      ctx.textAlign = 'center'
    }

    ctx.fillText(point.label, point.x, timeAxisY)
  })

  ctx.restore()
}
