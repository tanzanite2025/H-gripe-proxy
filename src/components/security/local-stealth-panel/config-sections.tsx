import { Eye, Network, RefreshCw, Shield } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Button, Chip, TextField } from '@/components/tailwind'
import type {
  AntiDiscoveryConfig,
  PortStealthConfig,
  ProcessStealthConfig,
} from '@/services/local-stealth'

import { SectionCard, ToggleRow } from './shared'

interface ProcessStealthSectionProps {
  config: ProcessStealthConfig
  loading: boolean
  onChange: (updates: Partial<ProcessStealthConfig>) => void
}

export function ProcessStealthSection({
  config,
  loading,
  onChange,
}: ProcessStealthSectionProps) {
  return (
    <SectionCard
      icon={<Eye className="h-4 w-4" />}
      title="进程隐匿"
      description="通过伪装窗口标题或进程展示名称，降低桌面环境中被直接特征识别的概率。"
    >
      <ToggleRow
        title="启用进程隐匿"
        description="开启后会把当前程序对外展示的标题替换成更低特征值的名称。"
        checked={config.enabled}
        disabled={loading}
        onCheckedChange={(checked) => onChange({ enabled: checked })}
      />

      {config.enabled ? (
        <TextField
          label="伪装标题"
          value={config.disguise_title}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onChange({ disguise_title: event.target.value })
          }
          helperText="建议使用中性、常见、与当前环境相符的标题，避免过度突兀。"
          fullWidth
          size="small"
        />
      ) : null}
    </SectionCard>
  )
}

interface PortStealthSectionProps {
  config: PortStealthConfig
  currentPort: number | null
  loading: boolean
  onChange: (updates: Partial<PortStealthConfig>) => void
  onAllocatePort: () => void
}

export function PortStealthSection({
  config,
  currentPort,
  loading,
  onChange,
  onAllocatePort,
}: PortStealthSectionProps) {
  return (
    <SectionCard
      icon={<Network className="h-4 w-4" />}
      title="端口隐匿"
      description="通过随机分配运行端口，减少长期固定使用常见代理端口带来的可识别性。"
    >
      <ToggleRow
        title="启用端口随机化"
        description="开启后会优先在指定区间内挑选端口，并规避预设的常见代理端口。"
        checked={config.enabled}
        disabled={loading}
        onCheckedChange={(checked) => onChange({ enabled: checked })}
      />

      {config.enabled ? (
        <>
          <div className="flex flex-wrap items-end gap-3">
            <div className="w-full max-w-[180px]">
              <TextField
                label="起始端口"
                type="number"
                value={String(config.port_range[0])}
                onChange={(event: ChangeEvent<HTMLInputElement>) =>
                  onChange({
                    port_range: [
                      Number(event.target.value) || config.port_range[0],
                      config.port_range[1],
                    ],
                  })
                }
                size="small"
                fullWidth
              />
            </div>

            <div className="w-full max-w-[180px]">
              <TextField
                label="结束端口"
                type="number"
                value={String(config.port_range[1])}
                onChange={(event: ChangeEvent<HTMLInputElement>) =>
                  onChange({
                    port_range: [
                      config.port_range[0],
                      Number(event.target.value) || config.port_range[1],
                    ],
                  })
                }
                size="small"
                fullWidth
              />
            </div>

            <Button
              variant="outlined"
              size="small"
              startIcon={<RefreshCw className="h-3.5 w-3.5" />}
              onClick={onAllocatePort}
              disabled={loading}
            >
              分配端口
            </Button>
          </div>

          <div className="flex flex-wrap gap-2">
            <Chip
              size="small"
              color={currentPort ? 'success' : 'default'}
              label={currentPort ? `当前端口 ${currentPort}` : '尚未分配端口'}
            />
            <Chip
              size="small"
              color="info"
              label={`范围 ${config.port_range[0]}-${config.port_range[1]}`}
            />
          </div>

          <div className="text-xs text-text-secondary">
            规避常见端口：{config.avoid_ports.join(', ')}
          </div>
        </>
      ) : null}
    </SectionCard>
  )
}

interface AntiDiscoverySectionProps {
  config: AntiDiscoveryConfig
  loading: boolean
  onChange: (updates: Partial<AntiDiscoveryConfig>) => void
}

const ANTI_DISCOVERY_TOGGLES = [
  {
    key: 'disable_mdns',
    title: '禁用 mDNS',
    description: '关闭 5353 相关的局域网服务发现广播。',
  },
  {
    key: 'disable_upnp',
    title: '禁用 UPnP',
    description: '减少端口映射和设备发现带来的局域网暴露面。',
  },
  {
    key: 'disable_llmnr',
    title: '禁用 LLMNR',
    description: '避免本地名称解析广播被旁路观察或利用。',
  },
  {
    key: 'disable_netbios',
    title: '禁用 NetBIOS',
    description: '减少旧式 Windows 局域网广播与主机暴露信息。',
  },
  {
    key: 'disable_ssdp',
    title: '禁用 SSDP',
    description: '降低 UPnP 相关设备探测与服务发现信号。',
  },
] as const

export function AntiDiscoverySection({
  config,
  loading,
  onChange,
}: AntiDiscoverySectionProps) {
  return (
    <SectionCard
      icon={<Shield className="h-4 w-4" />}
      title="防本地发现"
      description="减少局域网广播、设备发现和本地名称解析泄露，让本机在局域网环境里更低可见。"
    >
      <ToggleRow
        title="启用防本地发现"
        description="开启后可以按协议逐项关闭 mDNS、UPnP、LLMNR、NetBIOS 和 SSDP。"
        checked={config.enabled}
        disabled={loading}
        onCheckedChange={(checked) => onChange({ enabled: checked })}
      />

      {config.enabled ? (
        <div className="space-y-3">
          {ANTI_DISCOVERY_TOGGLES.map((item) => (
            <ToggleRow
              key={item.key}
              title={item.title}
              description={item.description}
              checked={config[item.key]}
              disabled={loading}
              onCheckedChange={(checked) => onChange({ [item.key]: checked })}
            />
          ))}
        </div>
      ) : null}
    </SectionCard>
  )
}
