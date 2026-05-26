import {
  Box,
  Chip,
  List,
  ListItem,
  ListItemText,
  Paper,
  Typography,
} from '@mui/material'
import {
  CheckCircleOutlined,
  RadioButtonUnchecked,
} from '@mui/icons-material'

import type { ObfuscationStrategy } from '@/services/obfuscation'

interface ObfuscationStrategyConfigProps {
  strategy: ObfuscationStrategy
}

export function ObfuscationStrategyConfig({
  strategy,
}: ObfuscationStrategyConfigProps) {
  const features = [
    {
      key: 'trafficObfuscation',
      label: '流量混淆',
      description: '随机化包大小和时序',
    },
    {
      key: 'protocolObfuscation',
      label: '协议混淆',
      description: 'HTTP/HTTPS 伪装',
    },
    {
      key: 'timingObfuscation',
      label: '时序混淆',
      description: '添加随机延迟',
    },
    {
      key: 'packetSizeObfuscation',
      label: '包大小混淆',
      description: '随机化数据包大小',
    },
    {
      key: 'tlsFingerprintRandomization',
      label: 'TLS 指纹随机化',
      description: '模拟不同浏览器',
    },
    {
      key: 'httpHeaderObfuscation',
      label: 'HTTP 头混淆',
      description: '随机化 HTTP 请求头',
    },
  ]

  return (
    <Box>
      <Typography variant="h6" gutterBottom>
        混淆策略详情
      </Typography>

      <Paper variant="outlined" sx={{ p: 2, mb: 2 }}>
        <Box sx={{ display: 'flex', alignItems: 'center', gap: 1, mb: 1 }}>
          <Typography variant="subtitle1">{strategy.name}</Typography>
          <Chip label={strategy.level} size="small" color="primary" />
        </Box>
        <Typography variant="body2" color="text.secondary">
          {strategy.description}
        </Typography>
      </Paper>

      <Typography variant="subtitle2" gutterBottom>
        启用的功能
      </Typography>

      <List>
        {features.map((feature) => {
          const enabled =
            strategy.features[
              feature.key as keyof typeof strategy.features
            ]

          return (
            <ListItem key={feature.key}>
              <Box
                sx={{
                  mr: 2,
                  color: enabled ? 'success.main' : 'text.disabled',
                }}
              >
                {enabled ? <CheckCircleOutlined /> : <RadioButtonUnchecked />}
              </Box>
              <ListItemText
                primary={feature.label}
                secondary={feature.description}
                slotProps={{
                  primary: {
                    style: {
                      color: enabled ? undefined : 'rgba(0, 0, 0, 0.38)',
                    },
                  },
                  secondary: {
                    style: {
                      color: enabled ? undefined : 'rgba(0, 0, 0, 0.38)',
                    },
                  },
                }}
              />
            </ListItem>
          )
        })}
      </List>

      <Typography variant="subtitle2" gutterBottom sx={{ mt: 2 }}>
        配置参数
      </Typography>

      <Paper variant="outlined" sx={{ p: 2 }}>
        <Box sx={{ display: 'flex', flexDirection: 'column', gap: 1 }}>
          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">
              填充大小范围
            </Typography>
            <Typography variant="body2">
              {strategy.config.minPaddingSize} - {strategy.config.maxPaddingSize}{' '}
              字节
            </Typography>
          </Box>

          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">
              时序抖动
            </Typography>
            <Typography variant="body2">
              {strategy.config.timingJitter} ms
            </Typography>
          </Box>

          <Box sx={{ display: 'flex', justifyContent: 'space-between' }}>
            <Typography variant="body2" color="text.secondary">
              包大小变化
            </Typography>
            <Typography variant="body2">
              ±{strategy.config.packetSizeVariation}%
            </Typography>
          </Box>
        </Box>
      </Paper>
    </Box>
  )
}
