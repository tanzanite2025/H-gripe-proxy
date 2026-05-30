/**
 * DNS 统计卡片组件
 * 显示 DNS 缓存、健康检查、智能分流、Tor 等统计信息
 */

import {
  RefreshCw as CachedRounded,
  CheckCircle as CheckCircleRounded,
  AlertCircle as ErrorRounded,
  RefreshCw as RefreshRounded,
  AlertTriangle as WarningRounded,
  Router as RouterRounded,
  Shield as VpnLockRounded,
} from 'lucide-react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import { LinearProgress } from '@/components/tailwind/LinearProgress'
import type { DnsRuntimeStatus } from '@/services/cmds'

interface Props {
  runtimeStatus?: DnsRuntimeStatus
  runtimeStatusPending: boolean
  onRefresh: () => void
}

const getRoutingModeLabel = (mode: string | null) => {
  switch (mode) {
    case 'speed':
      return '速度优先'
    case 'privacy':
      return '隐私优先'
    case 'balanced':
      return '平衡模式'
    case 'custom':
      return '自定义'
    default:
      return 'N/A'
  }
}

const getLeakProtectionLabel = (level: string | null) => {
  switch (level) {
    case 'none':
      return '无防护'
    case 'basic':
      return '基础'
    case 'strict':
      return '严格'
    case 'paranoid':
      return '偏执'
    case 'custom':
      return '自定义'
    default:
      return 'N/A'
  }
}

const getLeakSecurityLabel = (security: string | null) => {
  switch (security) {
    case 'low':
      return '低'
    case 'medium':
      return '中'
    case 'high':
      return '高'
    case 'very-high':
      return '极高'
    case 'custom':
      return '自定义'
    default:
      return 'N/A'
  }
}

const getLeakSecurityColor = (security: string | null) => {
  switch (security) {
    case 'low':
      return 'error' as const
    case 'medium':
      return 'warning' as const
    case 'high':
      return 'info' as const
    case 'very-high':
      return 'success' as const
    default:
      return 'default' as const
  }
}

const getBoolLabel = (
  value: boolean | null,
  enabledLabel = '已启用',
  disabledLabel = '已关闭',
) => {
  if (value === null) {
    return 'N/A'
  }

  return value ? enabledLabel : disabledLabel
}

export const DnsStatsCard = ({
  runtimeStatus,
  runtimeStatusPending,
  onRefresh,
}: Props) => {
  if (runtimeStatusPending && !runtimeStatus) {
    return (
      <Card>
        <div className="p-4 min-h-[200px] flex flex-col">
          <div className="mb-2 flex items-center gap-1 text-sm font-semibold">
            <CachedRounded className="h-4 w-4" />
            DNS 当前运行态统计
          </div>
          <div className="flex-1 flex items-center justify-center">
            <div className="text-xs text-muted-foreground">正在加载 DNS 运行态数据...</div>
          </div>
          <LinearProgress />
        </div>
      </Card>
    )
  }

  if (!runtimeStatus) {
    return (
      <Card>
        <div className="p-4 min-h-[200px] flex flex-col">
          <div className="mb-2 flex items-center gap-1 text-sm font-semibold">
            <CachedRounded className="h-4 w-4" />
            DNS 当前运行态统计
          </div>
          <div className="flex-1 flex items-center justify-center">
            <div className="text-sm text-muted-foreground">
              暂时无法读取后端 DNS 运行态统计。
            </div>
          </div>
        </div>
      </Card>
    )
  }

  const { snapshot, derived } = runtimeStatus
  const domesticDns = derived.domestic_dns.join(', ') || 'N/A'
  const foreignDns = derived.foreign_dns.join(', ') || 'N/A'
  const runtimeSource = runtimeStatus.enable_dns_settings
    ? '来自已保存 dns_config.yaml 派生配置'
    : '来自当前基础 runtime 配置'

  return (
    <Card>
      <div className="p-4">
        <div className="mb-2 flex items-center justify-between">
          <div className="flex items-center gap-1 text-sm font-semibold">
            <CachedRounded className="h-4 w-4" />
            DNS 当前运行态统计
          </div>
          <Button
            size="small"
            startIcon={<RefreshRounded className="h-4 w-4" />}
            onClick={onRefresh}
            disabled={runtimeStatusPending}
          >
            刷新
          </Button>
        </div>

        <div className="mb-2 text-xs text-gray-500 dark:text-gray-400">
          此处展示的是 Rust 后端确认的当前 DNS 运行态统计，不会随未保存的表单编辑即时变化。
        </div>

        {/* DNS 缓存统计 */}
        <div className="mb-2">
          <div className="mb-1 block text-xs text-gray-500 dark:text-gray-400">
            DNS 运行态摘要
          </div>
          <div className="space-y-1">
            <div className="flex justify-between">
              <div className="text-sm">Nameserver 数量</div>
              <div className="text-sm font-bold">
                {snapshot.nameserver_count}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">Fallback 数量</div>
              <div className="text-sm font-bold text-green-600 dark:text-green-400">
                {snapshot.fallback_count}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">Nameserver Policy 数量</div>
              <div className="text-sm font-bold text-yellow-600 dark:text-yellow-400">
                {snapshot.nameserver_policy_count}
              </div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">Default Nameserver 数量</div>
              <div className="text-sm font-bold">
                {derived.default_nameserver_count}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">当前运行态</div>
              <div className="text-sm font-bold">
                {runtimeStatus.runtime_has_dns ? 'DNS 已注入' : 'DNS 未注入'}
              </div>
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 健康检查统计 */}
        <div className="mb-2">
          <div className="mb-1 block text-xs text-gray-500 dark:text-gray-400">
            DNS 运行态选项
          </div>
          <div className="space-y-1">
            <div className="flex justify-between">
              <div className="text-sm">增强模式</div>
              <div className="text-sm font-bold">
                {snapshot.enhanced_mode ?? 'N/A'}
              </div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">IPv6</div>
              <Chip
                icon={<CheckCircleRounded className="h-3 w-3" />}
                label={getBoolLabel(snapshot.ipv6, '已开启', '已关闭')}
                size="small"
                color={snapshot.ipv6 === null ? 'default' : snapshot.ipv6 ? 'success' : 'warning'}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">Prefer H3</div>
              <Chip
                icon={<WarningRounded className="h-3 w-3" />}
                label={getBoolLabel(derived.prefer_h3, '已开启', '已关闭')}
                size="small"
                color={derived.prefer_h3 === null ? 'default' : derived.prefer_h3 ? 'success' : 'warning'}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">Use Hosts</div>
              <Chip
                icon={<ErrorRounded className="h-3 w-3" />}
                label={getBoolLabel(snapshot.use_hosts, '已开启', '已关闭')}
                size="small"
                color={snapshot.use_hosts === null ? 'default' : snapshot.use_hosts ? 'success' : 'warning'}
              />
            </div>
            <div className="flex justify-between">
              <div className="text-sm">Use System Hosts</div>
              <div className="text-sm font-bold text-primary-600 dark:text-primary-400">
                {getBoolLabel(snapshot.use_system_hosts, '已开启', '已关闭')}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">Respect Rules</div>
              <div
                className="max-w-[200px] overflow-hidden text-ellipsis whitespace-nowrap text-sm font-bold"
                title={getBoolLabel(snapshot.respect_rules, '已开启', '已关闭')}
              >
                {getBoolLabel(snapshot.respect_rules, '已开启', '已关闭')}
              </div>
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 预解析统计 */}
        <div className="mb-2">
          <div className="mb-1 block text-xs text-gray-500 dark:text-gray-400">
            运行态对齐状态
          </div>
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="text-sm">dns_config.yaml</div>
              <Chip
                label={
                  runtimeStatus.dns_config_exists
                    ? runtimeStatus.dns_config_valid
                      ? '存在且有效'
                      : '存在但无效'
                    : '不存在'
                }
                size="small"
                color={runtimeStatus.dns_config_valid ? 'success' : 'warning'}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">DNS 段对齐</div>
              <Chip
                label={runtimeStatus.runtime_dns_matches_saved ? '已对齐' : '未对齐'}
                size="small"
                color={runtimeStatus.runtime_dns_matches_saved ? 'success' : 'warning'}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">Hosts 段对齐</div>
              <Chip
                label={runtimeStatus.runtime_hosts_matches_saved ? '已对齐' : '未对齐'}
                size="small"
                color={runtimeStatus.runtime_hosts_matches_saved ? 'success' : 'warning'}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">整体运行态</div>
              <Chip
                label={runtimeStatus.runtime_matches_saved ? '已对齐' : '未对齐'}
                size="small"
                color={runtimeStatus.runtime_matches_saved ? 'success' : 'warning'}
              />
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 智能分流统计 */}
        <div className="mb-2">
          <div className="mb-1 flex items-center gap-0.5 text-xs text-gray-500 dark:text-gray-400">
            <RouterRounded className="h-3.5 w-3.5" />
            DNS 智能分流
          </div>
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="text-sm">分流模式</div>
              <Chip
                label={getRoutingModeLabel(derived.routing_mode)}
                size="small"
                color={
                  derived.routing_mode === 'speed'
                    ? 'success'
                    : derived.routing_mode === 'privacy'
                      ? 'info'
                      : derived.routing_mode === 'balanced'
                        ? 'warning'
                        : 'default'
                }
              />
            </div>
            <div className="flex justify-between">
              <div className="text-sm">国内 DNS</div>
              <div
                className="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
                title={domesticDns}
              >
                {domesticDns}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">国外 DNS</div>
              <div
                className="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
                title={foreignDns}
              >
                {foreignDns}
              </div>
            </div>
            <div className="flex justify-between">
              <div className="text-sm">策略组数量</div>
              <div className="text-sm font-bold">
                {snapshot.nameserver_policy_count}
              </div>
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* Tor 代理统计 */}
        <div className="mb-2">
          <div className="mb-1 flex items-center gap-0.5 text-xs text-gray-500 dark:text-gray-400">
            <VpnLockRounded className="h-3.5 w-3.5" />
            运行态覆盖
          </div>
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="text-sm">覆盖开关</div>
              <Chip
                label={runtimeStatus.enable_dns_settings ? '已启用' : '未启用'}
                size="small"
                color={runtimeStatus.enable_dns_settings ? 'success' : 'warning'}
              />
            </div>
            <div className="flex justify-between">
              <div className="text-sm">当前来源</div>
              <div
                className="max-w-[180px] overflow-hidden text-ellipsis whitespace-nowrap text-xs font-bold"
                title={runtimeSource}
              >
                {runtimeSource}
              </div>
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">当前生效情况</div>
              <Chip
                label={runtimeStatus.runtime_has_dns ? '运行态已携带 DNS' : '运行态未携带 DNS'}
                size="small"
                color={runtimeStatus.runtime_has_dns ? 'success' : 'warning'}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">已保存产物</div>
              <Chip
                label={runtimeStatus.runtime_matches_saved ? '已生效' : '未完全生效'}
                size="small"
                color={runtimeStatus.runtime_matches_saved ? 'success' : 'warning'}
              />
            </div>
          </div>
        </div>

        <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

        {/* DNS 零泄漏防护统计 */}
        <div>
          <div className="mb-1 flex items-center gap-0.5 text-xs text-gray-500 dark:text-gray-400">
            <RouterRounded className="h-3.5 w-3.5" />
            零泄漏防护
          </div>
          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <div className="text-sm">防护级别</div>
              <Chip
                label={getLeakProtectionLabel(derived.leak_protection_level)}
                size="small"
                color={getLeakSecurityColor(derived.leak_protection_security)}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">安全等级</div>
              <Chip
                label={getLeakSecurityLabel(derived.leak_protection_security)}
                size="small"
                color={getLeakSecurityColor(derived.leak_protection_security)}
              />
            </div>
            <div className="flex items-center justify-between">
              <div className="text-sm">安全状态</div>
              {derived.leak_protection_safe === null ? (
                <Chip label="未知" size="small" color="default" />
              ) : derived.leak_protection_safe ? (
                <Chip icon={<CheckCircleRounded className="h-3 w-3" />} label="安全" size="small" color="success" />
              ) : (
                <Chip icon={<WarningRounded className="h-3 w-3" />} label="不安全" size="small" color="error" />
              )}
            </div>
          </div>
        </div>
      </div>
    </Card>
  )
}
