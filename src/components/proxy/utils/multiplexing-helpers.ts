/**
 * 多路复用辅助函数
 * 用于显示和格式化多路复用信息
 */

/**
 * 获取 SMUX 配置的 Tooltip 文本
 */
export function getSmuxTooltip(proxy: IProxyItem): string {
  // proxy.smux 在类型定义中是 boolean，但实际运行时可能是配置对象
  const smuxConfig = (proxy as any).smux
  if (!smuxConfig || typeof smuxConfig === 'boolean') return ''

  const parts: string[] = []

  // 协议类型
  if (smuxConfig.protocol) {
    parts.push(`协议: ${smuxConfig.protocol}`)
  }

  // 连接配置
  if (smuxConfig['max-connections']) {
    parts.push(`最大连接: ${smuxConfig['max-connections']}`)
  }

  // 流配置
  if (smuxConfig['min-streams']) {
    parts.push(`最小流: ${smuxConfig['min-streams']}`)
  }
  if (smuxConfig['max-streams']) {
    parts.push(`最大流: ${smuxConfig['max-streams']}`)
  }

  // 其他配置
  if (smuxConfig.padding) {
    parts.push('填充: 启用')
  }
  if (smuxConfig.statistic) {
    parts.push('统计: 启用')
  }
  if (smuxConfig['only-tcp']) {
    parts.push('仅TCP')
  }

  // Brutal 优化
  if (smuxConfig['brutal-opts']?.enabled) {
    const brutal = smuxConfig['brutal-opts']
    parts.push(`Brutal: ${brutal.up || '?'}↑ ${brutal.down || '?'}↓`)
  }

  return parts.length > 0 ? parts.join('\n') : 'SMUX 多路复用已启用'
}

/**
 * 获取 SMUX 协议的简短显示文本
 */
export function getSmuxShortText(proxy: IProxyItem): string {
  const smuxConfig = (proxy as any).smux
  if (!smuxConfig || typeof smuxConfig === 'boolean') return 'SMUX'

  const protocol = smuxConfig.protocol
  if (protocol) {
    return `SMUX (${protocol})`
  }

  return 'SMUX'
}

/**
 * 获取 Mieru 多路复用的 Tooltip 文本
 */
export function getMieruMultiplexTooltip(multiplexing: string): string {
  const levelMap: Record<string, string> = {
    MULTIPLEXING_OFF: '关闭 - 不使用多路复用',
    MULTIPLEXING_LOW: '低 - 最小的多路复用',
    MULTIPLEXING_MIDDLE: '中 - 平衡性能和资源',
    MULTIPLEXING_HIGH: '高 - 最大的多路复用',
  }

  return levelMap[multiplexing] || `多路复用级别: ${multiplexing}`
}

/**
 * 获取 Mieru 多路复用的简短显示文本
 */
export function getMieruMultiplexShortText(multiplexing: string): string {
  const shortMap: Record<string, string> = {
    MULTIPLEXING_OFF: 'OFF',
    MULTIPLEXING_LOW: 'LOW',
    MULTIPLEXING_MIDDLE: 'MID',
    MULTIPLEXING_HIGH: 'HIGH',
  }

  return `MUX (${shortMap[multiplexing] || multiplexing})`
}

/**
 * 获取 Sudoku HTTP Mask 多路复用的 Tooltip 文本
 */
export function getSudokuMultiplexTooltip(multiplex: string): string {
  const modeMap: Record<string, string> = {
    off: '关闭 - 不使用 HTTP Mask 多路复用',
    auto: '自动 - 根据情况自动启用',
    on: '开启 - 始终使用 HTTP Mask 多路复用',
  }

  return modeMap[multiplex] || `HTTP Mask 多路复用: ${multiplex}`
}

/**
 * 获取 Sudoku HTTP Mask 多路复用的简短显示文本
 */
export function getSudokuMultiplexShortText(multiplex: string): string {
  return `HTTP-MUX (${multiplex.toUpperCase()})`
}

/**
 * 检查代理是否启用了任何形式的多路复用
 */
export function hasMultiplexing(proxy: IProxyItem): boolean {
  // SMUX
  if (proxy.smux) return true

  // Mieru
  if (proxy.type === 'mieru' && (proxy as any).multiplexing) {
    const multiplexing = (proxy as any).multiplexing
    return multiplexing !== 'MULTIPLEXING_OFF'
  }

  // Sudoku
  if (proxy.type === 'sudoku' && (proxy as any).httpmask?.multiplex) {
    const multiplex = (proxy as any).httpmask.multiplex
    return multiplex !== 'off'
  }

  return false
}

/**
 * 获取多路复用的通用描述
 */
export function getMultiplexingDescription(proxy: IProxyItem): string {
  if (proxy.smux) {
    return getSmuxTooltip(proxy)
  }

  if (proxy.type === 'mieru' && (proxy as any).multiplexing) {
    return getMieruMultiplexTooltip((proxy as any).multiplexing)
  }

  if (proxy.type === 'sudoku' && (proxy as any).httpmask?.multiplex) {
    return getSudokuMultiplexTooltip((proxy as any).httpmask.multiplex)
  }

  return '多路复用已启用'
}
