import { Settings2 } from 'lucide-react'
import { useState, type ReactNode } from 'react'

import { Switch } from '@/components/base'
import { DnsLeakProtectionCard } from '@/components/setting/dns-leak-protection-card'
import { DnsRoutingCard } from '@/components/setting/dns-routing-card'
import {
  buildDnsRuntimeViewModel,
  formatDnsLeakProtectionLabel,
  formatDnsRoutingModeLabel,
} from '@/components/setting/dns-runtime-view-model'
import { DnsStatsCard } from '@/components/setting/dns-stats-card'
import { Tab, Tabs } from '@/components/tailwind'
import { Alert } from '@/components/tailwind/Alert'
import { Chip } from '@/components/tailwind/Chip'
import type { DnsRuntimeStatus } from '@/services/cmds'
import type { AdvancedDnsConfig } from '@/services/coordinator'
import { showNotice } from '@/services/notice-service'

import { ProviderCatalogTab } from './provider-catalog-tab'

const PROVIDER_CATALOG_UNLOCK_STEPS = 5
const DNS_TAB_OVERVIEW = 'overview'
const DNS_TAB_STATS = 'stats'
const DNS_TAB_ROUTING = 'routing'
const DNS_TAB_LEAK = 'leak'
const DNS_TAB_PROVIDERS = 'providers'

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
  const [providerCatalogUnlocked, setProviderCatalogUnlocked] = useState(false)
  const [providerCatalogUnlockCount, setProviderCatalogUnlockCount] = useState(0)

  const previewModeLabel = formatDnsRoutingModeLabel(config.routing_mode)
  const savedModeLabel = formatDnsRoutingModeLabel(savedConfig.routing_mode)
  const previewLeakLabel = formatDnsLeakProtectionLabel(
    config.leak_protection_level,
  )
  const savedLeakLabel = formatDnsLeakProtectionLabel(
    savedConfig.leak_protection_level,
  )

  const runtimeView = runtimeStatus
    ? buildDnsRuntimeViewModel(runtimeStatus)
    : null

  const handleProviderCatalogUnlock = () => {
    if (providerCatalogUnlocked) {
      setActiveTab(DNS_TAB_PROVIDERS)
      return
    }

    const nextCount = providerCatalogUnlockCount + 1
    if (nextCount >= PROVIDER_CATALOG_UNLOCK_STEPS) {
      setProviderCatalogUnlocked(true)
      setProviderCatalogUnlockCount(PROVIDER_CATALOG_UNLOCK_STEPS)
      setActiveTab(DNS_TAB_PROVIDERS)
      showNotice.success('Provider Catalog unlocked')
      return
    }

    setProviderCatalogUnlockCount(nextCount)
  }

  const renderTabPanel = (
    value: string,
    children: ReactNode,
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
      <div className="rounded-lg border border-border bg-card p-4">
        <div className="mb-3 flex items-start justify-between gap-3">
          <button
            type="button"
            onClick={handleProviderCatalogUnlock}
            className="text-left"
          >
            <div className="flex items-center gap-2 text-lg font-bold">
              <Settings2 className="h-4 w-4" />
              DNS 运行态应用
            </div>
            <div className="mt-1 text-sm text-muted-foreground">
              控制统一 DNS 配置派生出的 `dns_config.yaml` 是否应用到当前
              `core` 运行时。
            </div>
            {providerCatalogUnlocked ? (
              <div className="mt-2">
                <Chip size="small" color="info" label="Provider Catalog unlocked" />
              </div>
            ) : null}
          </button>

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
          {runtimePending && (
            <Chip size="small" color="info" label="切换中..." />
          )}
          {runtimeStatusPending && (
            <Chip size="small" color="info" label="同步后端状态中..." />
          )}
          {runtimeView?.runtimeAlignment.color === 'success' && (
            <Chip
              size="small"
              color="success"
              label="后端确认已与已保存配置一致"
            />
          )}
        </div>

        <Alert
          severity={runtimeEnabled ? 'success' : 'warning'}
          className="text-sm"
        >
          {runtimeEnabled
            ? '当前已启用 DNS 运行时覆盖，core 会使用从 AdvancedConfig.dns 派生出的 DNS 配置。'
            : '当前仅保存统一 DNS 配置，尚未将派生出的 DNS 配置应用到 core 运行时。'}
        </Alert>

        {runtimeView && (
          <div className="mt-3 grid grid-cols-1 gap-3 text-sm md:grid-cols-2">
            <div className="space-y-1 rounded-lg border border-border px-3 py-2">
              <div className="text-xs text-muted-foreground">
                后端确认的当前运行态
              </div>
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
                <div className="text-xs text-muted-foreground">
                  {runtimeView.summary}
                </div>
              )}
            </div>

            <div className="space-y-1 rounded-lg border border-border px-3 py-2">
              <div className="text-xs text-muted-foreground">
                后端对已保存产物的校验
              </div>
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

      <div className="rounded-lg border border-border bg-card p-4">
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
          {providerCatalogUnlocked ? (
            <Tab label="Providers" value={DNS_TAB_PROVIDERS} />
          ) : null}
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

            <Alert
              severity={hasUnsavedChanges ? 'warning' : 'info'}
              className="text-sm"
            >
              {hasUnsavedChanges
                ? '统计卡展示的是当前运行态；配置卡正在编辑的是表单预览，未保存变更尚未进入运行态。后端运行态以后端确认结果为准。'
                : '当前表单预览与已保存配置一致；统计卡展示的是当前运行态镜像。后端运行态以后端确认结果为准。'}
            </Alert>

            <div className="grid grid-cols-1 gap-3 md:grid-cols-2">
              <div className="space-y-2 rounded-lg border border-border px-3 py-2">
                <div className="text-xs text-muted-foreground">已保存配置</div>
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span>分流模式</span>
                  <Chip size="small" label={savedModeLabel} />
                </div>
                <div className="flex items-center justify-between gap-3 text-sm">
                  <span>泄漏防护</span>
                  <Chip size="small" label={savedLeakLabel} />
                </div>
              </div>

              <div className="space-y-2 rounded-lg border border-border px-3 py-2">
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
                  <span>泄漏防护</span>
                  <Chip
                    size="small"
                    color={
                      config.leak_protection_level ===
                      savedConfig.leak_protection_level
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
          <div className="rounded-lg border border-border bg-card p-4">
            <DnsRoutingCard
              mode={config.routing_mode}
              runtimeStatus={runtimeStatus}
              onChange={(routing_mode) => onChange({ ...config, routing_mode })}
            />
          </div>,
        )}

        {renderTabPanel(
          DNS_TAB_LEAK,
          <div className="rounded-lg border border-border bg-card p-4">
            <DnsLeakProtectionCard
              level={config.leak_protection_level}
              runtimeStatus={runtimeStatus}
              onChange={(leak_protection_level) =>
                onChange({ ...config, leak_protection_level })
              }
            />
          </div>,
        )}
        {providerCatalogUnlocked
          ? renderTabPanel(
              DNS_TAB_PROVIDERS,
              <ProviderCatalogTab runtimeStatus={runtimeStatus} />,
            )
          : null}
      </div>
    </div>
  )
}
