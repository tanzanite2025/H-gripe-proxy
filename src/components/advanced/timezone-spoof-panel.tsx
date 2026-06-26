import { useLockFn } from 'ahooks'
import { useState } from 'react'

import { Button, Stack } from '@/components/tailwind'
import { Card } from '@/components/tailwind/Card'
import { Switch } from '@/components/tailwind/Switch'
import {
  type TimezoneSpoofConfig,
  timezoneSpoofGetNtpServer,
  timezoneSpoofGetTimezone,
  timezoneSpoofGetLocale,
} from '@/services/timezone-spoof'

interface Props {
  config: TimezoneSpoofConfig
  onChange: (config: TimezoneSpoofConfig) => void
}

const NTP_STRATEGY_OPTIONS = [
  { value: 'Auto' as const, label: '自动（根据出口区域）', desc: '根据出口节点所在国家自动选择 NTP 服务器' },
  { value: 'Manual' as const, label: '手动指定', desc: '手动输入 NTP 服务器地址' },
  { value: 'Disabled' as const, label: '仅伪装 HTTP 头', desc: '不启用 NTP 同步，仅伪装 Accept-Language 等头部' },
]

const COMMON_COUNTRIES = [
  { code: 'JP', name: '日本' },
  { code: 'US', name: '美国' },
  { code: 'SG', name: '新加坡' },
  { code: 'HK', name: '香港' },
  { code: 'KR', name: '韩国' },
  { code: 'TW', name: '台湾' },
  { code: 'DE', name: '德国' },
  { code: 'GB', name: '英国' },
  { code: 'AU', name: '澳大利亚' },
  { code: 'IN', name: '印度' },
]

export function TimezoneSpoofPanel({ config, onChange }: Props) {
  const [previewCountry, setPreviewCountry] = useState('JP')
  const [previewResult, setPreviewResult] = useState<{
    ntp: string
    timezone: string
    locale: string
  } | null>(null)
  const [loading, setLoading] = useState(false)

  const handleToggleEnabled = (enabled: boolean) => {
    onChange({ ...config, enabled })
  }

  const handleStrategyChange = (ntp_strategy: TimezoneSpoofConfig['ntp_strategy']) => {
    onChange({ ...config, ntp_strategy })
  }

  const handlePreview = useLockFn(async () => {
    setLoading(true)
    try {
      const ntp = await timezoneSpoofGetNtpServer(previewCountry)
      const tz = await timezoneSpoofGetTimezone(previewCountry)
      const locale = await timezoneSpoofGetLocale(tz)
      setPreviewResult({ ntp, timezone: tz, locale })
    } finally {
      setLoading(false)
    }
  })

  return (
    <Card className="p-4">
      <Stack direction="column" spacing={2}>
        {/* 标题 + 开关 */}
        <div className="flex items-center justify-between">
          <div>
            <div className="text-base font-medium">时区/NTP 伪装</div>
            <div className="text-xs text-gray-500 dark:text-gray-400 mt-0.5">
              当出口节点在日本但系统时钟显示 UTC+8（中国），目标服务器可通过时间偏移推断用户不在日本。
              启用后，内核将通过代理同步出口区域附近的 NTP 服务器时间，减少时钟偏差泄露。
            </div>
          </div>
          <Switch checked={config.enabled} onCheckedChange={handleToggleEnabled} />
        </div>

        {config.enabled && (
          <>
            {/* NTP 策略 */}
            <div className="mt-2">
              <div className="text-sm font-medium mb-1.5">NTP 策略</div>
              <div className="space-y-1.5">
                {NTP_STRATEGY_OPTIONS.map((opt) => (
                  <label
                    key={opt.value}
                    className="flex items-start gap-2 p-2 rounded-lg cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-800 transition-colors"
                  >
                    <input
                      type="radio"
                      name="ntp_strategy"
                      value={opt.value}
                      checked={config.ntp_strategy === opt.value}
                      onChange={() => handleStrategyChange(opt.value)}
                      className="mt-0.5"
                    />
                    <div>
                      <div className="text-sm">{opt.label}</div>
                      <div className="text-xs text-gray-500 dark:text-gray-400">{opt.desc}</div>
                    </div>
                  </label>
                ))}
              </div>
            </div>

            {/* 手动 NTP 服务器 */}
            {config.ntp_strategy === 'Manual' && (
              <div className="mt-2">
                <div className="text-sm font-medium mb-1">NTP 服务器</div>
                <input
                  type="text"
                  value={config.manual_ntp_server || ''}
                  onChange={(e) =>
                    onChange({ ...config, manual_ntp_server: e.target.value || undefined })
                  }
                  placeholder="pool.ntp.org"
                  className="w-full px-3 py-1.5 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            )}

            {/* 同步间隔 */}
            {config.ntp_strategy !== 'Disabled' && (
              <div className="mt-2">
                <div className="text-sm font-medium mb-1">同步间隔（分钟）</div>
                <input
                  type="number"
                  min={5}
                  max={1440}
                  value={config.ntp_interval_min}
                  onChange={(e) =>
                    onChange({ ...config, ntp_interval_min: parseInt(e.target.value) || 30 })
                  }
                  className="w-32 px-3 py-1.5 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            )}

            {/* 写入系统时间 */}
            {config.ntp_strategy !== 'Disabled' && (
              <label className="flex items-center gap-2 mt-2">
                <Switch
                  checked={config.write_to_system}
                  onCheckedChange={(v: boolean) => onChange({ ...config, write_to_system: v })}
                />
                <span className="text-sm">写入系统时间</span>
                <span className="text-xs text-gray-500 dark:text-gray-400">（需要管理员权限）</span>
              </label>
            )}

            {/* NTP 代理 */}
            {config.ntp_strategy !== 'Disabled' && (
              <div className="mt-2">
                <div className="text-sm font-medium mb-1">NTP 代理组</div>
                <input
                  type="text"
                  value={config.dialer_proxy || ''}
                  onChange={(e) =>
                    onChange({ ...config, dialer_proxy: e.target.value || undefined })
                  }
                  placeholder="留空则走 DIRECT"
                  className="w-full px-3 py-1.5 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                />
              </div>
            )}

            {/* 预览 */}
            <div className="mt-3 p-3 rounded-lg bg-gray-50 dark:bg-gray-800/50">
              <div className="text-sm font-medium mb-2">区域预览</div>
              <div className="flex items-center gap-2 mb-2">
                <select
                  value={previewCountry}
                  onChange={(e) => setPreviewCountry(e.target.value)}
                  className="px-3 py-1.5 text-sm rounded-lg border border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 focus:outline-none focus:ring-2 focus:ring-blue-500"
                >
                  {COMMON_COUNTRIES.map((c) => (
                    <option key={c.code} value={c.code}>
                      {c.name} ({c.code})
                    </option>
                  ))}
                </select>
                <Button size="sm" onClick={handlePreview} loading={loading}>
                  预览
                </Button>
              </div>
              {previewResult && (
                <div className="space-y-1 text-xs">
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">NTP 服务器: </span>
                    <span className="font-mono">{previewResult.ntp}</span>
                  </div>
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">时区: </span>
                    <span className="font-mono">{previewResult.timezone}</span>
                  </div>
                  <div>
                    <span className="text-gray-500 dark:text-gray-400">Accept-Language: </span>
                    <span className="font-mono">{previewResult.locale}</span>
                  </div>
                </div>
              )}
            </div>
          </>
        )}
      </Stack>
    </Card>
  )
}
