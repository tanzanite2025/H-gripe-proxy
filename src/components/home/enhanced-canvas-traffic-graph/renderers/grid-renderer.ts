import { PADDING, LINE_WIDTH } from '../utils/graph-config'

/**
 * 网格渲染器
 * 负责绘制背景网格线
 */
export function drawGrid(
  ctx: CanvasRenderingContext2D,
  width: number,
  height: number,
  gridColor: string,
): void {
  const effectiveWidth = width - PADDING.LEFT - PADDING.RIGHT
  const effectiveHeight = height - PADDING.TOP - PADDING.BOTTOM

  ctx.save()
  ctx.strokeStyle = gridColor
  ctx.lineWidth = LINE_WIDTH.GRID
  ctx.globalAlpha = 0.7

  // 水平网格线
  const horizontalLines = 4
  for (let i = 1; i <= horizontalLines; i++) {
    const y = PADDING.TOP + (effectiveHeight / (horizontalLines + 1)) * i
    ctx.beginPath()
    ctx.moveTo(PADDING.LEFT, y)
    ctx.lineTo(width - PADDING.RIGHT, y)
    ctx.stroke()
  }

  // 垂直网格线
  const verticalLines = 6
  for (let i = 1; i <= verticalLines; i++) {
    const x = PADDING.LEFT + (effectiveWidth / (verticalLines + 1)) * i
    ctx.beginPath()
    ctx.moveTo(x, PADDING.TOP)
    ctx.lineTo(x, height - PADDING.BOTTOM)
    ctx.stroke()
  }

  ctx.restore()
}
