import { Card, Chip } from '@/components/tailwind'

interface MultiplexingStatsProps {
  proxyName: string
  proxyType: string
  multiplexingEnabled: boolean
  stats?: {
    connections?: number
    streams?: number
    reuseRate?: number
    avgLatency?: number
  }
}

export function MultiplexingStats({
  proxyName,
  proxyType,
  multiplexingEnabled,
  stats,
}: MultiplexingStatsProps) {
  if (!multiplexingEnabled) {
    return (
      <Card>
        <div className="p-4">
          <h6 className="mb-2 text-lg font-semibold">多路复用统计</h6>
          <p className="text-gray-500">多路复用未启用</p>
        </div>
      </Card>
    )
  }

  return (
    <Card>
      <div className="p-4">
        <div className="mb-4 flex items-center">
          <h6 className="flex-1 text-lg font-semibold">多路复用统计</h6>
          <Chip label="已启用" color="success" size="small" />
        </div>

        <p className="mb-2 text-sm text-gray-500">
          代理: {proxyName} ({proxyType})
        </p>

        <div className="mt-4 grid grid-cols-2 gap-4">
          <div>
            <p className="text-xs text-gray-500">连接数</p>
            <p className="text-xl font-semibold">{stats?.connections ?? '-'}</p>
          </div>

          <div>
            <p className="text-xs text-gray-500">流数</p>
            <p className="text-xl font-semibold">{stats?.streams ?? '-'}</p>
          </div>

          <div>
            <p className="text-xs text-gray-500">复用率</p>
            <p className="text-xl font-semibold">
              {stats?.reuseRate ? `${stats.reuseRate}%` : '-'}
            </p>
          </div>

          <div>
            <p className="text-xs text-gray-500">平均延迟</p>
            <p className="text-xl font-semibold">
              {stats?.avgLatency ? `${stats.avgLatency}ms` : '-'}
            </p>
          </div>
        </div>

        <p className="mt-4 block text-xs text-gray-500">
          注: 统计数据需要代理核心支持
        </p>
      </div>
    </Card>
  )
}
