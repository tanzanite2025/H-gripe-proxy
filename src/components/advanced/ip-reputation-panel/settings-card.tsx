import type { ChangeEvent, MouseEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Collapse } from '@/components/tailwind/Collapse'
import { Select } from '@/components/tailwind/Select'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import type {
  IpMetadataProviderConfig,
  IpMetadataProviderHealthReport,
  IpMetadataProviderRegistration,
  IpReputationConfig,
} from '@/services/ip-reputation/model'

import { formatMetadataProviderAvailability } from './shared'

interface IpReputationSettingsCardProps {
  config: IpReputationConfig
  metadataProviders: IpMetadataProviderRegistration[]
  metadataOptionsDraft: string
  providerOverrideVisible: boolean
  providerProbeLoading: boolean
  providerProbeIp: string
  providerProbeResult: IpMetadataProviderHealthReport | null
  onToggleEnabled: (enabled: boolean) => void
  onTtlChange: (value: string) => void
  onRefreshCache: () => void | Promise<void>
  onClearCache: () => void | Promise<void>
  onProviderPanelToggle: (event: MouseEvent) => void
  onProviderKindChange: (value: string | number) => void
  onDatabasePathChange: (value: string) => void
  onApiEndpointChange: (value: string) => void
  onAccessTokenChange: (value: string) => void
  onMetadataOptionsDraftChange: (value: string) => void
  onMetadataOptionsDraftCommit: () => void
  onProviderProbeIpChange: (value: string) => void
  onProbeProvider: () => void | Promise<void>
}

export function IpReputationSettingsCard({
  config,
  metadataProviders,
  metadataOptionsDraft,
  providerOverrideVisible,
  providerProbeLoading,
  providerProbeIp,
  providerProbeResult,
  onToggleEnabled,
  onTtlChange,
  onRefreshCache,
  onClearCache,
  onProviderPanelToggle,
  onProviderKindChange,
  onDatabasePathChange,
  onApiEndpointChange,
  onAccessTokenChange,
  onMetadataOptionsDraftChange,
  onMetadataOptionsDraftCommit,
  onProviderProbeIpChange,
  onProbeProvider,
}: IpReputationSettingsCardProps) {
  const selectableMetadataProviders = metadataProviders.filter(
    (provider) => provider.availability !== 'placeholder',
  )

  const activeMetadataProvider =
    metadataProviders.find(
      (provider) => provider.kind === config.metadataProvider.kind,
    ) ?? null

  return (
    <Card>
      <div className="space-y-4">
        <div className="flex items-center justify-between">
          <div>
            <h3
              className="select-none text-sm font-semibold"
              onClick={onProviderPanelToggle}
            >
              IP 信誉数据源
            </h3>
            <p className="mt-1 text-xs text-gray-500">
              为当前节点和当前出口身份识别提供底层证据，手动 IP 查询仅用于调试。
            </p>
          </div>
          <Switch
            checked={config.enabled}
            onCheckedChange={onToggleEnabled}
          />
        </div>

        {config.enabled && (
          <div className="space-y-4">
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
              <TextField
                label="缓存 TTL（秒）"
                type="number"
                value={String(config.cacheTtl)}
                onChange={(event: ChangeEvent<HTMLInputElement>) =>
                  onTtlChange(event.target.value)
                }
                helperText="IP 信誉结果的缓存时长。"
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
                  清除缓存
                </Button>
              </div>
            </div>

            <Collapse in={providerOverrideVisible}>
              <div className="space-y-4 rounded-lg border border-amber-200/70 bg-amber-50/80 p-4 dark:border-amber-900/40 dark:bg-amber-950/10">
                <div className="space-y-1">
                  <p className="text-xs font-semibold uppercase tracking-widest text-amber-700 dark:text-amber-300">
                    Hidden Provider Override
                  </p>
                  <p className="text-xs text-amber-700/90 dark:text-amber-200/80">
                    这是隐藏的高级数据源切换入口，仅用于高级调试。它只影响 IP
                    信誉分析链路，不会改变顶部出口卡片只认 Mihomo 的单源逻辑。
                  </p>
                </div>

                <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                  <Select
                    label="Metadata Provider"
                    value={config.metadataProvider.kind}
                    onChange={onProviderKindChange}
                    options={selectableMetadataProviders.map((provider) => ({
                      value: provider.kind,
                      label: `${provider.label} (${formatMetadataProviderAvailability(provider.availability)})`,
                    }))}
                    fullWidth
                  />

                  <div className="rounded-lg border border-gray-200/80 bg-white/70 px-3 py-2 text-xs text-gray-600 dark:border-gray-800 dark:bg-gray-900/30 dark:text-gray-300">
                    <p className="font-medium text-gray-800 dark:text-gray-100">
                      {activeMetadataProvider?.label ?? config.metadataProvider.kind}
                    </p>
                    <p className="mt-1">
                      {activeMetadataProvider?.description ??
                        '当前数据源描述信息尚未加载。'}
                    </p>
                  </div>
                </div>

                {activeMetadataProvider?.availability === 'experimental' && (
                  <div className="rounded-lg border border-yellow-200 bg-yellow-50 px-3 py-2 text-xs text-yellow-800 dark:border-yellow-900/40 dark:bg-yellow-950/20 dark:text-yellow-200">
                    当前 provider 为 experimental：代码已经可切换，但默认不启用，建议只在明确测试或定向验证时使用。
                  </div>
                )}

                {config.metadataProvider.kind === 'geoLite2AsnMmdb' && (
                  <TextField
                    label="Database Path Override"
                    value={config.metadataProvider.databasePath ?? ''}
                    onChange={(event: ChangeEvent<HTMLInputElement>) =>
                      onDatabasePathChange(event.target.value)
                    }
                    placeholder="留空时自动搜索 GeoLite2-ASN.mmdb / ASN.mmdb"
                    helperText="可选。留空时使用应用标准目录搜索。"
                  />
                )}

                {config.metadataProvider.kind === 'ipinfoHttpApi' && (
                  <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                    <TextField
                      label="API Endpoint"
                      value={config.metadataProvider.apiEndpoint ?? ''}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        onApiEndpointChange(event.target.value)
                      }
                      placeholder="https://api.ipinfo.io/lite"
                      helperText="留空时使用内置 IPinfo Lite endpoint"
                    />
                    <TextField
                      label="Access Token"
                      type="password"
                      value={config.metadataProvider.accessToken ?? ''}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        onAccessTokenChange(event.target.value)
                      }
                      placeholder="IPinfo token"
                      helperText="切换到 IPinfo 时为必填"
                    />
                    <div className="md:col-span-2">
                      <TextField
                        label="Options"
                        multiline
                        rows={4}
                        value={metadataOptionsDraft}
                        onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                          onMetadataOptionsDraftChange(event.target.value)
                        }
                        onBlur={onMetadataOptionsDraftCommit}
                        placeholder="timeoutSeconds=10"
                        helperText="每行一个 key=value，当前已支持 timeoutSeconds。"
                      />
                    </div>
                  </div>
                )}

                <div className="space-y-3 border-t border-amber-200/70 pt-4 dark:border-amber-900/30">
                  <div className="grid grid-cols-1 gap-4 md:grid-cols-[minmax(0,1fr)_auto]">
                    <TextField
                      label="Provider Test IP"
                      value={providerProbeIp}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        onProviderProbeIpChange(event.target.value)
                      }
                      placeholder="1.1.1.1"
                      helperText="使用当前未保存的 provider 配置做一次真实 lookup。"
                    />
                    <div className="flex items-end">
                      <Button
                        onClick={() => void onProbeProvider()}
                        variant="outlined"
                        size="sm"
                        disabled={providerProbeLoading}
                      >
                        {providerProbeLoading ? 'Testing...' : 'Test Provider'}
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
                        <span className="rounded bg-black/5 px-2 py-0.5 dark:bg-white/10">
                          {formatMetadataProviderAvailability(
                            providerProbeResult.availability,
                          )}
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
                          <p className="font-mono">
                            {providerProbeResult.targetIp}
                          </p>
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
                          <span className="opacity-70">Country</span>
                          <p>{providerProbeResult.countryCode ?? '--'}</p>
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              </div>
            </Collapse>
          </div>
        )}
      </div>
    </Card>
  )
}
