import { CheckCircle, Circle } from 'lucide-react'

import { Chip } from '@/components/tailwind/Chip'
import { List, ListItem, ListItemText } from '@/components/tailwind/List'
import { Paper } from '@/components/tailwind/Paper'
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
    <div>
      <h3 className="text-lg font-semibold mb-4">混淆策略详情</h3>

      <Paper className="p-4 mb-4">
        <div className="flex items-center gap-2 mb-2">
          <h4 className="text-base font-medium">{strategy.name}</h4>
          <Chip label={strategy.level} size="small" color="primary" />
        </div>
        <p className="text-sm text-gray-600 dark:text-gray-400">
          {strategy.description}
        </p>
      </Paper>

      <h4 className="text-sm font-medium mb-2">启用的功能</h4>

      <List>
        {features.map((feature) => {
          const enabled =
            strategy.features[
              feature.key as keyof typeof strategy.features
            ]

          return (
            <ListItem key={feature.key}>
              <div
                className={`mr-4 ${
                  enabled ? 'text-green-500' : 'text-gray-400'
                }`}
              >
                {enabled ? (
                  <CheckCircle className="h-5 w-5" />
                ) : (
                  <Circle className="h-5 w-5" />
                )}
              </div>
              <ListItemText
                primary={feature.label}
                secondary={feature.description}
                className={!enabled ? 'opacity-40' : ''}
              />
            </ListItem>
          )
        })}
      </List>

      <h4 className="text-sm font-medium mb-2 mt-4">配置参数</h4>

      <Paper className="p-4">
        <div className="flex flex-col gap-2">
          <div className="flex justify-between">
            <span className="text-sm text-gray-600 dark:text-gray-400">
              填充大小范围
            </span>
            <span className="text-sm">
              {strategy.config.minPaddingSize} - {strategy.config.maxPaddingSize}{' '}
              字节
            </span>
          </div>

          <div className="flex justify-between">
            <span className="text-sm text-gray-600 dark:text-gray-400">
              时序抖动
            </span>
            <span className="text-sm">
              {strategy.config.timingJitter} ms
            </span>
          </div>

          <div className="flex justify-between">
            <span className="text-sm text-gray-600 dark:text-gray-400">
              包大小变化
            </span>
            <span className="text-sm">
              ±{strategy.config.packetSizeVariation}%
            </span>
          </div>
        </div>
      </Paper>
    </div>
  )
}
