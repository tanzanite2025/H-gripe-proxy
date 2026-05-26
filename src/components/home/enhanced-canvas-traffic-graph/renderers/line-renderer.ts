import type { ChartStyle } from '../utils/graph-config'
import { ALPHA, LINE_WIDTH, PADDING } from '../utils/graph-config'
import { calculateY } from '../utils/graph-helpers'

/**
 * 流量线渲染器
 * 负责绘制上传和下载流量线
 */
export function drawTrafficLine(
  ctx: CanvasRenderingContext2D,
  data: ITrafficDataPoint[],
  valueKey: 'up' | 'down',
  width: number,
  height: number,
  color: string,
  chartStyle: ChartStyle,
  withGradient: boolean,
  topValue: number,
  bottomValue: number,
): void {
  if (data.length < 2) return

  const effectiveWidth = width - PADDING.LEFT - PADDING.RIGHT
  const lastIndex = data.length - 1
  const getX = (index: number) =>
    PADDING.LEFT + (index / lastIndex) * effectiveWidth
  const getY = (index: number) =>
    calculateY(data[index][valueKey], height, topValue, bottomValue)

  ctx.save()

  // 绘制渐变填充
  if (withGradient && chartStyle === 'bezier') {
    const gradient = ctx.createLinearGradient(
      0,
      PADDING.TOP,
      0,
      height - PADDING.BOTTOM,
    )
    gradient.addColorStop(
      0,
      `${color}${Math.round(ALPHA.GRADIENT * 255)
        .toString(16)
        .padStart(2, '0')}`,
    )
    gradient.addColorStop(1, `${color}00`)

    ctx.beginPath()
    ctx.moveTo(getX(0), getY(0))

    if (chartStyle === 'bezier') {
      for (let i = 1; i < data.length; i++) {
        const currentX = getX(i)
        const currentY = getY(i)
        const nextIndex = Math.min(i + 1, lastIndex)
        const controlX = (currentX + getX(nextIndex)) / 2
        const controlY = (currentY + getY(nextIndex)) / 2
        ctx.quadraticCurveTo(currentX, currentY, controlX, controlY)
      }
    } else {
      for (let i = 1; i < data.length; i++) {
        ctx.lineTo(getX(i), getY(i))
      }
    }

    ctx.lineTo(getX(lastIndex), height - PADDING.BOTTOM)
    ctx.lineTo(getX(0), height - PADDING.BOTTOM)
    ctx.closePath()
    ctx.fillStyle = gradient
    ctx.fill()
  }

  // 绘制主线条
  ctx.beginPath()
  ctx.strokeStyle = color
  ctx.lineWidth = valueKey === 'up' ? LINE_WIDTH.UP : LINE_WIDTH.DOWN
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'
  ctx.globalAlpha = ALPHA.LINE

  ctx.moveTo(getX(0), getY(0))

  if (chartStyle === 'bezier') {
    for (let i = 1; i < data.length; i++) {
      const currentX = getX(i)
      const currentY = getY(i)
      const nextIndex = Math.min(i + 1, lastIndex)
      const controlX = (currentX + getX(nextIndex)) / 2
      const controlY = (currentY + getY(nextIndex)) / 2
      ctx.quadraticCurveTo(currentX, currentY, controlX, controlY)
    }
  } else {
    for (let i = 1; i < data.length; i++) {
      ctx.lineTo(getX(i), getY(i))
    }
  }

  ctx.stroke()
  ctx.restore()
}

/**
 * 绘制悬浮指示线
 */
export function drawHoverIndicator(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  dataIndex: number,
  dataLength: number,
  highlightY: number,
  textColor: string,
): void {
  const effectiveWidth = width - PADDING.LEFT - PADDING.RIGHT
  const dataX = PADDING.LEFT + (dataIndex / (dataLength - 1)) * effectiveWidth

  ctx.save()
  ctx.strokeStyle = textColor
  ctx.lineWidth = 1
  ctx.globalAlpha = 0.6
  ctx.setLineDash([4, 4]) // 虚线效果

  // 绘制垂直指示线
  ctx.beginPath()
  ctx.moveTo(dataX, PADDING.TOP)
  ctx.lineTo(dataX, height - PADDING.BOTTOM)
  ctx.stroke()

  // 绘制水平指示线（高亮Y轴位置）
  ctx.beginPath()
  ctx.moveTo(PADDING.LEFT, highlightY)
  ctx.lineTo(width - PADDING.RIGHT, highlightY)
  ctx.stroke()

  ctx.restore()
}
