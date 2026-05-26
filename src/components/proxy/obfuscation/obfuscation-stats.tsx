import { Box, Card, CardContent, Chip, Typography } from '@mui/material'

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
        <CardContent>
          <Typography variant="h6" gutterBottom>
            混淆统计
          </Typography>
          <Typography color="text.secondary">混淆未启用</Typography>
        </CardContent>
      </Card>
    )
  }

  return (
    <Card>
      <CardContent>
        <Box sx={{ display: 'flex', alignItems: 'center', mb: 2 }}>
          <Typography variant="h6" sx={{ flex: 1 }}>
            混淆统计
          </Typography>
          <Chip label="已启用" color="success" size="small" />
        </Box>

        {stats && (
          <>
            <Box
              sx={{
                display: 'grid',
                gridTemplateColumns: 'repeat(2, 1fr)',
                gap: 2,
                mb: 2,
              }}
            >
              <Box>
                <Typography variant="caption" color="text.secondary">
                  混淆级别
                </Typography>
                <Typography variant="h6" sx={{ textTransform: 'uppercase' }}>
                  {stats.level}
                </Typography>
              </Box>

              <Box>
                <Typography variant="caption" color="text.secondary">
                  平均填充大小
                </Typography>
                <Typography variant="h6">
                  {Math.round(stats.avgPaddingSize)} 字节
                </Typography>
              </Box>

              <Box>
                <Typography variant="caption" color="text.secondary">
                  平均时序抖动
                </Typography>
                <Typography variant="h6">
                  {Math.round(stats.avgTimingJitter)} ms
                </Typography>
              </Box>

              <Box>
                <Typography variant="caption" color="text.secondary">
                  包大小变化
                </Typography>
                <Typography variant="h6">
                  ±{stats.packetSizeVariation}%
                </Typography>
              </Box>
            </Box>

            <Box sx={{ display: 'flex', gap: 1, flexWrap: 'wrap' }}>
              {stats.httpHeaderObfuscation && (
                <Chip label="HTTP 头混淆" size="small" color="primary" />
              )}
              {stats.tlsFingerprintRandomization && (
                <Chip label="TLS 指纹随机化" size="small" color="primary" />
              )}
            </Box>

            <Typography
              variant="caption"
              color="text.secondary"
              sx={{ display: 'block', mt: 2 }}
            >
              注: 混淆会增加少量延迟和流量开销
            </Typography>
          </>
        )}
      </CardContent>
    </Card>
  )
}
