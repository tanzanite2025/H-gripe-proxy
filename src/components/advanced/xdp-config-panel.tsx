/**
 * XDP 代理配置面板（仅 Linux）
 */

import { AlertTriangle, Info } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Switch, TextField, Select } from '@/components/tailwind'
import type { SelectChangeEvent } from '@/components/tailwind/Select'
import type { XdpConfig, XdpMode } from '@/services/coordinator'

interface Props {
  config: XdpConfig
  onChange: (config: XdpConfig) => void
}

const modeLabels: Record<XdpMode, string> = {
  Native: 'Native（原生模式，最快）',
  Skb: 'SKB（兼容模式）',
  Generic: 'Generic（通用模式）',
}

export function XdpConfigPanel({ config, onChange }: Props) {
  return (
    <div>
      <div className="p-4 bg-blue-500 text-white rounded-lg mb-4">
        <div className="flex items-start gap-2">
          <Info className="w-5 h-5 flex-shrink-0 mt-0.5" />
          <p className="text-sm">
            XDP（eXpress Data Path）是 Linux 内核的高性能数据包处理框架。
            启用后可以获得 10 倍以上的性能提升。
          </p>
        </div>
      </div>

      <div className="p-4 bg-yellow-500 text-white rounded-lg mb-4">
        <div className="flex items-start gap-2">
          <AlertTriangle className="w-5 h-5 flex-shrink-0 mt-0.5" />
          <p className="text-sm">
            ⚠️ XDP 需要 root 权限和支持 XDP 的网卡驱动。请确保您的系统满足要求。
          </p>
        </div>
      </div>

      {/* 总开关 */}
      <div className="p-4 bg-card border border-border rounded-lg mb-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="font-semibold">启用 XDP 代理</p>
            <p className="text-xs text-muted-foreground">零内核态切换，极致性能</p>
          </div>
          <Switch
            checked={config.enabled}
            onCheckedChange={(checked) =>
              onChange({ ...config, enabled: checked })
            }
          />
        </div>
      </div>

      {config.enabled && (
        <>
          {/* 网卡接口 */}
          <div className="p-4 bg-card border border-border rounded-lg mb-4">
            <h3 className="text-sm font-semibold mb-4">网卡配置</h3>
            <TextField
              label="网卡接口"
              value={config.interface}
              onChange={(e: ChangeEvent<HTMLInputElement>) =>
                onChange({ ...config, interface: e.target.value })
              }
              helperText="例如：eth0, ens33, wlan0"
              fullWidth
            />
          </div>

          {/* XDP 模式 */}
          <div className="p-4 bg-card border border-border rounded-lg mb-4">
            <h3 className="text-sm font-semibold mb-4">XDP 模式</h3>
            <Select
              label="模式"
              value={config.mode}
              onChange={(e: SelectChangeEvent) =>
                onChange({
                  ...config,
                  mode: e.target.value as XdpMode,
                })
              }
              fullWidth
            >
              {Object.entries(modeLabels).map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </Select>

            <div className="space-y-2 mt-4">
              <div className="p-3 bg-green-500 text-white rounded-lg">
                <p className="font-semibold text-sm">Native 模式</p>
                <p className="text-xs opacity-90">
                  最快，但需要网卡驱动支持。延迟 ~10μs，吞吐量 50+ Gbps
                </p>
              </div>

              <div className="p-3 bg-blue-500 text-white rounded-lg">
                <p className="font-semibold text-sm">SKB 模式</p>
                <p className="text-xs opacity-90">
                  兼容性好，性能略低于 Native
                </p>
              </div>

              <div className="p-3 bg-yellow-500 text-white rounded-lg">
                <p className="font-semibold text-sm">Generic 模式</p>
                <p className="text-xs opacity-90">
                  所有网卡都支持，但性能最低
                </p>
              </div>
            </div>
          </div>

          {/* 队列大小 */}
          <div className="p-4 bg-card border border-border rounded-lg">
            <h3 className="text-sm font-semibold mb-4">高级设置</h3>
            <TextField
              label="队列大小"
              type="number"
              value={config.queue_size.toString()}
              onChange={(e: ChangeEvent<HTMLInputElement>) =>
                onChange({
                  ...config,
                  queue_size: Number.parseInt(e.target.value) || 4096,
                })
              }
              helperText="数据包队列大小，默认 4096"
              fullWidth
            />
          </div>
        </>
      )}
    </div>
  )
}
