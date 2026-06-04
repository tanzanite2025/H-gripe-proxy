/**
 * DNS 高级功能面板
 * 包含 DNS 统计、DNS 智能分流、DNS 零泄漏防护
 */

import { Settings2 } from 'lucide-react'
import { useState } from 'react'

import { Switch } from '@/components/base'
import { DnsLeakProtectionCard } from '@/components/setting/dns-leak-protection-card'
import { DnsRoutingCard } from '@/components/setting/dns-routing-card'
import { buildDnsRuntimeViewModel } from '@/components/setting/dns-runtime-view-model'
import { DnsStatsCard } from '@/components/setting/dns-stats-card'
import { Tab, Tabs } from '@/components/tailwind'
import { Alert } from '@/components/tailwind/Alert'
import { Chip } from '@/components/tailwind/Chip'
import type { DnsRuntimeStatus } from '@/services/cmds'
import type { AdvancedDnsConfig } from '@/services/coordinator'

const DNS_TAB_OVERVIEW = 'overview'
const DNS_TAB_STATS = 'stats'
const DNS_TAB_ROUTING = 'routing'
const DNS_TAB_LEAK = 'leak'

interface Props {
  config: AdvancedDnsConfig
  savedConfig: AdvancedDnsConfig
  hasUnsavedChanges: boolean
  runtimeStatus?: DnsRuntimeStatus
  runtimeStatusPending: boolean
  onRuntimeStatusRefresh: () => void
  onChange: (config: AdvancedDnsConfig) => void
  runtimeEnabled: boolean
  runtimePending: boolean
  onRuntimeToggle: (enabled: boolean) => void | Promise<void>
}

export function DnsAdvancedPanel({
  config,
  savedConfig,
  hasUnsavedChanges,
  runtimeStatus,
  runtimeStatusPending,
  onRuntimeStatusRefresh,
  onChange,
  runtimeEnabled,
  runtimePending,
  onRuntimeToggle,
}: Props) {
  const [activeTab, setActiveTab] = useState(DNS_TAB_OVERVIEW)

  const previewModeLabel =
    config.routing_mode === 'speed'
      ? '速度优先'
      : config.routing_mode === 'privacy'
        ? '隐私优先'
        : config.routing_mode === 'balanced'
          ? '平衡模式'
          : '自定义'

  const savedModeLabel =
    savedConfig.routing_mode === 'speed'
      ? '速度优先'
      : savedConfig.routing_mode === 'privacy'
        ? '隐私优先'
        : savedConfig.routing_mode === 'balanced'
          ? '平衡模式'
          : '自定义'

  const previewLeakLabel =
    config.leak_protection_level === 'none'
      ? '无防护'
      : config.leak_protection_level === 'basic'
        ? '基础'
        : config.leak_protection_level === 'strict'
          ? '严格'
          : '偏执'

  const savedLeakLabel =
    savedConfig.leak_protection_level === 'none'
      ? '无防护'
      : savedConfig.leak_protection_level === 'basic'
        ? '基础'
        : savedConfig.leak_protection_level === 'strict'
          ? '严格'
          : '偏执'

  const runtimeView = runtimeStatus
    ? buildDnsRuntimeViewModel(runtimeStatus)
    : null

  const renderTabPanel = (
    value: string,
    children: React.ReactNode,
    className = 'mt-4',
  ) => (
    <div
      role="tabpanel"
      hidden={activeTab !== value}
      className={activeTab === value ? className : undefined}
    >
      {activeTab === value && children}
    </div>
  )

  return (
    <div className="space-y-4">
      <div className="bg-card border border-border rounded-lg p-4">
        <div className="mb-3 flex items-start justify-between gap-3">
          <div>
            <div className="flex items-center gap-2 text-lg font-bold">
              <Settings2 className="h-4 w-4" />
              DNS 运行时应用
            </div>
            <div className="mt-1 text-sm text-muted-foreground">
              控制统一 DNS 配置派生出的 `dns_config.yaml` 是否应用到当前 core 运行时。
            </div>
          </div>

          <Switch
            checked={runtimeEnabled}
            disabled={runtimePending}
            onChange={(_event, checked) => onRuntimeToggle(checked)}
          />
        </div>

        <div className="mb-3 flex flex-wrap items-center gap-2">
          <Chip
            size="small"
            color={runtimeEnabled ? 'success' : 'warning'}
            label={runtimeEnabled ? '已应用到运行时' : '未应用到运行时'}
          />
          {runtimePending && <Chip size="small" color="info" label="切换中..." />}
          {runtimeStatusPending && <Chip size="small" color="info" label="同步后端状态中..." />}
          {runtimeView?.runtimeAlignment.color === 'success' && (
            <Chip size="small" color="success" label="后端确认已与已保存配置一致" />
          )}
        </div>

        <Alert severity={runtimeEnabled ? 'success' : 'warning'} className="text-sm">
          {runtimeEnabled
            ? '当前已启用 DNS 运行时覆盖，core 会使用从 AdvancedConfig.dns 派生的 DNS 配置。'
            : '当前仅保存统一 DNS 配置，尚未将派生的 DNS 配置应用到 core 运行时。'}
        </Alert>

        {runtimeView && (
          <div className="mt-3 grid grid-cols-1 md:grid-cols-2 gap-3 text-sm">
            <div className="rounded-lg border border-border px-3 py-2 space-y-1">
              <div className="text-xs text-muted-foreground">后端确认的当前运行态</div>
              <div className="flex items-center justify-between gap-3">
                <span>DNS 段</span>
                <Chip
                  size="small"
                  color={runtimeView.runtimeDnsPresence.color}
                  label={runtimeView.runtimeDnsPresence.label}
                />
              </div>
              <div className="flex items-center justify-between gap-3">
                <span>Hosts 段</span>
                <Chip
                  size="small"
                  color={runtimeView.runtimeHostsPresence.color}
                  label={runtimeView.runtimeHostsPresence.label}
                />
              </div>
              {runtimeView.summary && (
                <div className="text-xs text-muted-foreground">{runtimeView.summary}</div>
              )}
            </div>

            <div className="rounded-lg border border-border px-3 py-2 space-y-1">
              <div className="text-xs text-muted-foreground">后端对已保存产物的校验</div>
              <div className="flex items-center justify-between gap-3">
                <span>dns_config.yaml</span>
                <Chip
                  size="small"
                  color={runtimeView.dnsConfig.color}
                  label={runtimeView.dnsConfig.label}
                />
              </div>
              <div className="flex items-center justify-between gap-3">
                <span>运行态对齐</span>
                <Chip
                  size="small"
                  color={runtimeView.runtimeAlignment.color}
                  label={runtimeView.runtimeAlignment.label}
                />
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="bg-card border border-border rounded-lg p-4">
        <Tabs
          value={activeTab}
          onChange={(_event, value) => setActiveTab(String(value))}
          variant="scrollable"
          className="border-b border-border"
        >
          <Tab label="概览" value={DNS_TAB_OVERVIEW} />
          <Tab label="统计" value={DNS_TAB_STATS} />
          <Tab label="分流" value={DNS_TAB_ROUTING} />
          <Tab label="防泄漏" value={DNS_TAB_LEAK} />
        </Tabs>

        {renderTabPanel(
          DNS_TAB_OVERVIEW,
          <div className="space-y-3">
            <div className="flex items-center justify-between gap-3">
              <div className="text-sm font-semibold">DNS 状态视角</div>
              <div className="flex flex-wrap items-center gap-2">
                <Chip size="small" color="info" label="当前运行态" />
                <Chip size="small" color="default" label="已保存配置" />
                <Chip size="small" color="warning" label="表单预览" />
              </div>
            </div>

            <Alert severity={hasUnsavedChanges ? 'warning' : 'info'} className="text-sm">
              {hasUnsavedChanges
                ? '统计卡显示的是当前运行态；配置卡正在编辑的是表单预览，未保存变更尚未进入运行态。后端运行态状态以顶部“后端确认的当前运行态”为准。'
                : '当前表单预览与已保存配置一致；统计卡显示的是当前运行态镜像。后端运行态状态以顶部“后端确认的当前运行态”为准。'}
            </Alert>

            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div className="rounded-lg border border-border px-3 py-2 space-y-2">
                <div className="text-xs text-muted-foreground">已保存配置</div>
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span>分流模式</span>
                  <Chip size="small" label={savedModeLabel} />
                </div>
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span>零泄漏防护</span>
                  <Chip size="small" label={savedLeakLabel} />
                </div>
              </div>

              <div className="rounded-lg border border-border px-3 py-2 space-y-2">
                <div className="text-xs text-muted-foreground">表单预览</div>
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span>分流模式</span>
                  <Chip
                    size="small"
                    color={
                      config.routing_mode === savedConfig.routing_mode
                        ? 'default'
                        : 'warning'
                    }
                    label={previewModeLabel}
                  />
                </div>
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span>零泄漏防护</span>
                  <Chip
                    size="small"
                    color={
                      config.leak_protection_level === savedConfig.leak_protection_level
                        ? 'default'
                        : 'warning'
                    }
                    label={previewLeakLabel}
                  />
                </div>
              </div>
            </div>
          </div>,
        )}

        {renderTabPanel(
          DNS_TAB_STATS,
          <DnsStatsCard
            runtimeStatus={runtimeStatus}
            runtimeStatusPending={runtimeStatusPending}
            onRefresh={onRuntimeStatusRefresh}
          />,
        )}

        {renderTabPanel(
          DNS_TAB_ROUTING,
          <div className="bg-card border border-border rounded-lg p-4">
            <DnsRoutingCard
              mode={config.routing_mode}
              runtimeStatus={runtimeStatus}
              onChange={(routing_mode) => onChange({ ...config, routing_mode })}
            />
          </div>,
        )}

        {renderTabPanel(
          DNS_TAB_LEAK,
          <div className="bg-card border border-border rounded-lg p-4">
            <DnsLeakProtectionCard
              level={config.leak_protection_level}
              runtimeStatus={runtimeStatus}
              onChange={(leak_protection_level) =>
                onChange({ ...config, leak_protection_level })
              }
            />
          </div>,
        )}
      </div>
    </div>
  )
}
