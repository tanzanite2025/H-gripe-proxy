import { Card, Chip } from '@/components/tailwind'

interface ObfuscationStatsProps {
  enabled: boolean
  stats?: {
    level: string
    avgPaddingSize: number
    avgTimingJitter: number
    packetSizeVariation: number
    httpHeaderObfuscation: boolean
    tlsFingerprintRandomization: boolean
  }
}

export function ObfuscationStats({ enabled, stats }: ObfuscationStatsProps) {
  if (!enabled) {
    return (
      <Card>
        <div className="p-4">
          <h6 className="mb-2 text-lg font-semibold">混淆统计</h6>
          <p className="text-gray-500">混淆未启用</p>
        </div>
      </Card>
    )
  }

  return (
    <Card>
      <div className="p-4">
        <div className="mb-4 flex items-center">
          <h6 className="flex-1 text-lg font-semibold">混淆统计</h6>
          <Chip label="已启用" color="success" size="small" />
        </div>

        {stats && (
          <>
            <div className="mb-4 grid grid-cols-2 gap-4">
              <div>
                <p className="text-xs text-gray-500">混淆级别</p>
                <p className="text-xl font-semibold uppercase">{stats.level}</p>
              </div>

              <div>
                <p className="text-xs text-gray-500">平均填充大小</p>
                <p className="text-xl font-semibold">
                  {Math.round(stats.avgPaddingSize)} 字节
                </p>
              </div>

              <div>
                <p className="text-xs text-gray-500">平均时序抖动</p>
                <p className="text-xl font-semibold">
                  {Math.round(stats.avgTimingJitter)} ms
                </p>
              </div>

              <div>
                <p className="text-xs text-gray-500">包大小变化</p>
                <p className="text-xl font-semibold">
                  ±{stats.packetSizeVariation}%
                </p>
              </div>
            </div>

            <div className="flex flex-wrap gap-2">
              {stats.httpHeaderObfuscation && (
                <Chip label="HTTP 头混淆" size="small" color="primary" />
              )}
              {stats.tlsFingerprintRandomization && (
                <Chip label="TLS 指纹随机化" size="small" color="primary" />
              )}
            </div>

            <p className="mt-4 block text-xs text-gray-500">
              注: 混淆会增加少量延迟和流量开销
            </p>
          </>
        )}
      </div>
    </Card>
  )
}
