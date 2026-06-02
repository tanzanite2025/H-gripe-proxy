/**
 * XDP proxy configuration UI.
 */

import { AlertCircle, CheckCircle, Info, Rocket, Zap } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Button, Switch, Select, TextField } from '@/components/tailwind'
import type {
  XdpConfig,
  XdpMode,
  XdpStatus,
  XdpSupportInfo,
} from '@/services/xdp'

interface XdpConfigUIProps {
  config: XdpConfig
  status: XdpStatus | null
  supportInfo: XdpSupportInfo | null
  interfaces: string[]
  saving: boolean
  loading: boolean
  onConfigChange: (config: XdpConfig) => void
  onSaveConfig: () => void
  onStart: () => void
  onStop: () => void
  formatBytes: (bytes: number) => string
  formatNumber: (num: number) => string
}

const xdpModeLabels: Record<XdpMode, string> = {
  Native: 'Native (best performance, driver support required)',
  Skb: 'SKB (better compatibility)',
  Generic: 'Generic (works with most adapters)',
}

export default function XdpConfigUI({
  config,
  status,
  supportInfo,
  interfaces,
  saving,
  loading,
  onConfigChange,
  onSaveConfig,
  onStart,
  onStop,
  formatBytes,
  formatNumber,
}: XdpConfigUIProps) {
  return (
    <div className="p-6">
      <div className="space-y-6">
        <div className="flex items-center gap-2">
          <Rocket className="w-5 h-5 text-primary" />
          <h2 className="text-xl font-semibold">XDP Proxy</h2>
        </div>

        <div className="p-4 bg-blue-500 text-white rounded-lg">
          <div className="flex items-start gap-2">
            <Info className="w-5 h-5 flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-sm">Kernel data path</p>
              <p className="text-xs opacity-90 mt-1">
                XDP processes packets close to the network driver for lower
                latency and higher throughput on supported Linux systems.
              </p>
            </div>
          </div>
        </div>

        {supportInfo && (
          <div className="p-4 bg-card border border-border rounded-lg">
            <h3 className="text-sm font-semibold mb-4">System Support</h3>
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                {supportInfo.xdp_supported ? (
                  <CheckCircle className="w-4 h-4 text-green-500" />
                ) : (
                  <AlertCircle className="w-4 h-4 text-red-500" />
                )}
                <span className="text-sm">
                  XDP: {supportInfo.xdp_supported ? 'supported' : 'unsupported'}
                </span>
              </div>
              <div className="flex items-center gap-2">
                {supportInfo.native_mode_supported ? (
                  <CheckCircle className="w-4 h-4 text-green-500" />
                ) : (
                  <AlertCircle className="w-4 h-4 text-yellow-500" />
                )}
                <span className="text-sm">
                  Native mode:{' '}
                  {supportInfo.native_mode_supported
                    ? 'supported'
                    : 'unsupported'}
                </span>
              </div>
              <p className="text-xs text-muted-foreground">
                Kernel: {supportInfo.kernel_version}
              </p>
            </div>
          </div>
        )}

        <div className="p-4 bg-card border border-border rounded-lg">
          <h3 className="text-sm font-semibold mb-4">Configuration</h3>
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <label className="text-sm font-medium">Enable XDP proxy</label>
              <Switch
                checked={config.enabled}
                onCheckedChange={(checked) =>
                  onConfigChange({ ...config, enabled: checked })
                }
              />
            </div>

            <Select
              label="Network interface"
              value={config.interface}
              onChange={(e) =>
                onConfigChange({ ...config, interface: e.target.value })
              }
              disabled={!config.enabled}
              fullWidth
            >
              {interfaces.map((iface) => (
                <option key={iface} value={iface}>
                  {iface}
                </option>
              ))}
            </Select>

            <Select
              label="XDP mode"
              value={config.mode}
              onChange={(e) =>
                onConfigChange({
                  ...config,
                  mode: e.target.value as XdpMode,
                })
              }
              disabled={!config.enabled}
              fullWidth
            >
              {Object.entries(xdpModeLabels).map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </Select>

            <TextField
              label="Queue size"
              type="number"
              value={String(config.queue_size)}
              onChange={(e: ChangeEvent<HTMLInputElement>) =>
                onConfigChange({
                  ...config,
                  queue_size: Number.parseInt(e.target.value, 10) || 4096,
                })
              }
              disabled={!config.enabled}
              fullWidth
            />
          </div>
        </div>

        {status && (
          <div className="p-4 bg-card border border-border rounded-lg">
            <h3 className="text-sm font-semibold mb-4">Runtime Status</h3>
            <div className="space-y-4">
              <div className="flex gap-2 flex-wrap">
                <span
                  className={`inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm ${
                    status.running
                      ? 'bg-green-500 text-white'
                      : 'bg-secondary text-secondary-foreground'
                  }`}
                >
                  {status.running ? (
                    <CheckCircle className="w-3 h-3" />
                  ) : (
                    <AlertCircle className="w-3 h-3" />
                  )}
                  {status.running ? 'Running' : 'Stopped'}
                </span>
                {status.running && (
                  <>
                    <span className="px-3 py-1 bg-secondary text-secondary-foreground rounded-full text-sm">
                      Interface: {status.interface}
                    </span>
                    <span className="px-3 py-1 bg-secondary text-secondary-foreground rounded-full text-sm">
                      Mode: {status.mode}
                    </span>
                  </>
                )}
              </div>

              {status.running && (
                <div>
                  <p className="text-xs font-semibold mb-2">Statistics</p>
                  <div className="grid grid-cols-2 gap-2">
                    <Stat label="Total packets" value={formatNumber(status.stats.total_packets)} />
                    <Stat label="Proxied packets" value={formatNumber(status.stats.proxied_packets)} />
                    <Stat label="Direct packets" value={formatNumber(status.stats.direct_packets)} />
                    <Stat label="Rejected packets" value={formatNumber(status.stats.rejected_packets)} />
                    <Stat
                      label="Errors"
                      value={formatNumber(status.stats.errors)}
                      valueClassName="text-red-500"
                    />
                    <Stat
                      label="Bytes processed"
                      value={formatBytes(status.stats.bytes_processed)}
                    />
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        <div className="p-4 bg-green-500 text-white rounded-lg">
          <div className="flex items-center gap-2 mb-4">
            <Zap className="w-5 h-5" />
            <h3 className="text-sm font-semibold">Performance Profile</h3>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <p className="text-3xl font-bold">10x</p>
              <p className="text-xs opacity-90">lower latency</p>
            </div>
            <div>
              <p className="text-3xl font-bold">10x</p>
              <p className="text-xs opacity-90">higher throughput</p>
            </div>
            <div>
              <p className="text-3xl font-bold">80%</p>
              <p className="text-xs opacity-90">less CPU overhead</p>
            </div>
          </div>
        </div>

        <div className="flex gap-4">
          <Button
            variant="default"
            onClick={onSaveConfig}
            disabled={saving || loading}
            className="flex-1"
          >
            {saving ? 'Saving...' : 'Save configuration'}
          </Button>
          {status?.running ? (
            <Button
              variant="destructive"
              onClick={onStop}
              disabled={loading}
              className="flex-1"
            >
              Stop proxy
            </Button>
          ) : (
            <Button
              variant="default"
              onClick={onStart}
              disabled={loading || !config.enabled}
              className="flex-1 bg-green-500 hover:bg-green-600"
            >
              Start proxy
            </Button>
          )}
        </div>
      </div>
    </div>
  )
}

function Stat({
  label,
  value,
  valueClassName = '',
}: {
  label: string
  value: string
  valueClassName?: string
}) {
  return (
    <div>
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className={`text-sm font-medium ${valueClassName}`}>{value}</p>
    </div>
  )
}
