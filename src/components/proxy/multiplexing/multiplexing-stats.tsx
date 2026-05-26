import { Box, Card, CardContent, Chip, Typography } from '@mui/material'

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
        <CardContent>
          <Typography variant="h6" gutterBottom>
            多路复用统计
          </Typography>
          <Typography color="text.secondary">
            多路复用未启用
          </Typography>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <Typography variant="h6" sx={{ flex: 1 }}>
            多路复用统计
          </Typography>
          <Chip label="已启用" color="success" size="small" />
        </Box>

        <Typography variant="body2" color="text.secondary" gutterBottom>
          代理: {proxyName} ({proxyType})
        </Typography>

        <Box
          sx={{
            display: 'grid',
            gridTemplateColumns: 'repeat(2, 1fr)',
            gap: 2,
            mt: 2,
          }}
        >
          <Box>
            <Typography variant="caption" color="text.secondary">
              连接数
            </Typography>
            <Typography variant="h6">
              {stats?.connections ?? '-'}
            </Typography>
          </Box>

          <Box>
            <Typography variant="caption" color="text.secondary">
              流数
            </Typography>
            <Typography variant="h6">
              {stats?.streams ?? '-'}
            </Typography>
          </Box>

          <Box>
            <Typography variant="caption" color="text.secondary">
              复用率
            </Typography>
            <Typography variant="h6">
              {stats?.reuseRate ? `${stats.reuseRate}%` : '-'}
            </Typography>
          </Box>

          <Box>
            <Typography variant="caption" color="text.secondary">
              平均延迟
            </Typography>
            <Typography variant="h6">
              {stats?.avgLatency ? `${stats.avgLatency}ms` : '-'}
            </Typography>
          </Box>
        </Box>

        <Typography
          variant="caption"
          color="text.secondary"
          sx={{ display: 'block', mt: 2 }}
        >
          注: 统计数据需要代理核心支持
        </Typography>
      </CardContent>
    </Card>
  )
}
