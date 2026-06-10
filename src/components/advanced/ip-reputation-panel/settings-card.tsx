import type { ChangeEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import type {
  IpMetadataProviderHealthReport,
  IpReputationConfig,
} from '@/services/ip-reputation/model'

interface IpReputationSettingsCardProps {
  config: IpReputationConfig
  providerProbeLoading: boolean
  providerProbeIp: string
  providerProbeResult: IpMetadataProviderHealthReport | null
  onToggleEnabled: (enabled: boolean) => void
  onTtlChange: (value: string) => void
  onRefreshCache: () => void | Promise<void>
  onClearCache: () => void | Promise<void>
  onProviderProbeIpChange: (value: string) => void
  onProbeProvider: () => void | Promise<void>
}

export function IpReputationSettingsCard({
  config,
  providerProbeLoading,
  providerProbeIp,
  providerProbeResult,
  onToggleEnabled,
  onTtlChange,
  onRefreshCache,
  onClearCache,
  onProviderProbeIpChange,
  onProbeProvider,
}: IpReputationSettingsCardProps) {
  return (
    <Card>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3 className="select-none text-sm font-semibold">IP 信誉数据源</h3>
            <p className="mt-1 text-xs text-gray-500">
              当前运行链路只读取本地 `GeoLite2-ASN.mmdb` 和
              `GeoLite2-City.mmdb`，不再允许切换外部 provider，也不再做国家码、
              ASN 或机房猜测兜底。
            </p>
          </div>
          <Switch
            checked={config.enabled}
            onCheckedChange={onToggleEnabled}
          />
        </div>

        {config.enabled && (
          <div className="space-y-4">
            <div className="rounded-lg border border-teal-200/70 bg-teal-50/70 px-3 py-3 text-xs text-teal-900 dark:border-teal-900/40 dark:bg-teal-950/15 dark:text-teal-100">
              <div className="grid grid-cols-1 gap-2 md:grid-cols-2">
                <div>
                  <span className="opacity-70">Active Source</span>
                  <p className="font-medium">Local GeoLite2 MMDB</p>
                </div>
                <div>
                  <span className="opacity-70">Files</span>
                  <p className="font-mono">
                    GeoLite2-ASN.mmdb / GeoLite2-City.mmdb
                  </p>
                </div>
              </div>
            </div>

            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              <TextField
                label="缓存 TTL（秒）"
                type="number"
                value={String(config.cacheTtl)}
                onChange={(event: ChangeEvent<HTMLInputElement>) =>
                  onTtlChange(event.target.value)
                }
                helperText="控制 IP 信誉结果在本地缓存多久。"
              />
              <div className="flex items-end gap-2">
                <Button
                  onClick={() => void onRefreshCache()}
                  variant="outlined"
                  size="sm"
                >
                  查看缓存
                </Button>
                <Button
                  onClick={() => void onClearCache()}
                  variant="outlined"
                  size="sm"
                >
                  清空缓存
                </Button>
              </div>
            </div>

            <div className="space-y-3 border-t border-gray-200/70 pt-4 dark:border-gray-800/70">
              <div className="grid grid-cols-1 gap-4 md:grid-cols-[minmax(0,1fr)_auto]">
                <TextField
                  label="本地 MMDB 测试 IP"
                  value={providerProbeIp}
                  onChange={(event: ChangeEvent<HTMLInputElement>) =>
                    onProviderProbeIpChange(event.target.value)
                  }
                  placeholder="1.1.1.1"
                  helperText="使用当前固定的本地 GeoLite2 数据源做一次真实查询。"
                />
                <div className="flex items-end">
                  <Button
                    onClick={() => void onProbeProvider()}
                    variant="outlined"
                    size="sm"
                    disabled={providerProbeLoading}
                  >
                    {providerProbeLoading ? 'Testing...' : 'Test MMDB'}
                  </Button>
                </div>
              </div>

              {providerProbeResult && (
                <div
                  className={`rounded-lg border px-3 py-3 text-xs ${
                    providerProbeResult.healthy
                      ? 'border-green-200 bg-green-50 text-green-800 dark:border-green-900/40 dark:bg-green-950/20 dark:text-green-200'
                      : 'border-red-200 bg-red-50 text-red-800 dark:border-red-900/40 dark:bg-red-950/20 dark:text-red-200'
                  }`}
                >
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="font-semibold">
                      {providerProbeResult.providerLabel}
                    </span>
                    <span className="rounded bg-black/5 px-2 py-0.5 dark:bg-white/10">
                      {providerProbeResult.healthy ? 'Healthy' : 'Failed'}
                    </span>
                    {providerProbeResult.latencyMs !== undefined && (
                      <span className="rounded bg-black/5 px-2 py-0.5 dark:bg-white/10">
                        {providerProbeResult.latencyMs} ms
                      </span>
                    )}
                  </div>
                  <p className="mt-2 break-all">
                    {providerProbeResult.message}
                  </p>
                  <div className="mt-2 grid grid-cols-1 gap-2 text-[11px] md:grid-cols-4">
                    <div>
                      <span className="opacity-70">Target</span>
                      <p className="font-mono">{providerProbeResult.targetIp}</p>
                    </div>
                    <div>
                      <span className="opacity-70">ASN</span>
                      <p>{providerProbeResult.asn ?? '--'}</p>
                    </div>
                    <div>
                      <span className="opacity-70">Org</span>
                      <p>{providerProbeResult.asnOrg ?? '--'}</p>
                    </div>
                    <div>
                      <span className="opacity-70">Timezone</span>
                      <p>{providerProbeResult.timezone ?? '--'}</p>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </Card>
  )
}
