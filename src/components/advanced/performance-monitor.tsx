/**
 * 性能监控面板
 */

import { AlertCircle, CheckCircle, RefreshCw } from 'lucide-react'

import { Button } from '@/components/tailwind'
import type { CoordinatorStatus } from '@/services/coordinator'

interface Props {
  status: CoordinatorStatus | null
  onRefresh: () => Promise<CoordinatorStatus | null>
}

export function PerformanceMonitor({ status, onRefresh }: Props) {
  if (!status) {
    return (
      <div>
        <div className="p-3 bg-blue-500 text-white rounded-lg">
          <p className="text-sm">加载中...</p>
        </div>
      </div>
    )
  }

  const domainPatternAssignments =
    status.runtimeState.stableEgressBackwrite.domainPatternAssignments
  const domainRuleBindings = status.runtimeState.stableEgressBackwrite.domainRuleBindings

  return (
    <div>
      {/* 安全状态警告 */}
      {status.securityCompromised && (
        <div className="p-4 bg-red-500 text-white rounded-lg mb-4">
          <p className="font-semibold text-sm mb-1">⚠️ 安全状态已被破坏</p>
          <p className="text-xs opacity-90">
            检测到调试器或恶意扫描。建议立即停止使用并检查系统安全。
          </p>
        </div>
      )}

      {/* 刷新按钮 */}
      <div className="flex justify-end mb-4">
        <Button variant="outline" size="sm" onClick={() => void onRefresh()}>
          <RefreshCw className="w-4 h-4 mr-1" />
          刷新状态
        </Button>
      </div>

      {/* 模块状态 */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {/* 协调器状态 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.initialized ? (
              <CheckCircle className="w-4 h-4 text-green-500" />
            ) : (
              <AlertCircle className="w-4 h-4 text-red-500" />
            )}
            <h3 className="text-sm font-semibold">核心协调器</h3>
          </div>

          <div className="flex justify-between items-center">
            <p className="text-sm">状态</p>
            <span
              className={`px-2 py-1 rounded-full text-xs ${
                status.initialized
                  ? 'bg-green-500 text-white'
                  : 'bg-red-500 text-white'
              }`}
            >
              {status.initialized ? '已初始化' : '未初始化'}
            </span>
          </div>
        </div>

        {/* 安全监控 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.securityEnabled && !status.securityCompromised ? (
              <CheckCircle className="w-4 h-4 text-green-500" />
            ) : status.securityCompromised ? (
              <AlertCircle className="w-4 h-4 text-red-500" />
            ) : (
              <AlertCircle className="w-4 h-4 text-yellow-500" />
            )}
            <h3 className="text-sm font-semibold">安全监控</h3>
          </div>

          <div className="flex justify-between items-center">
            <p className="text-sm">状态</p>
            <span
              className={`px-2 py-1 rounded-full text-xs ${
                status.securityEnabled
                  ? status.securityCompromised
                    ? 'bg-red-500 text-white'
                    : 'bg-green-500 text-white'
                  : 'bg-secondary text-secondary-foreground'
              }`}
            >
              {status.securityEnabled
                ? status.securityCompromised
                  ? '已破坏'
                  : '运行中'
                : '未启用'}
            </span>
          </div>
        </div>

        {/* 反探测 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.antiProbeEnabled ? (
              <CheckCircle className="w-4 h-4 text-green-500" />
            ) : (
              <AlertCircle className="w-4 h-4 text-yellow-500" />
            )}
            <h3 className="text-sm font-semibold">反主动探测</h3>
          </div>

          <div className="flex justify-between items-center">
            <p className="text-sm">状态</p>
            <span
              className={`px-2 py-1 rounded-full text-xs ${
                status.antiProbeEnabled
                  ? 'bg-green-500 text-white'
                  : 'bg-secondary text-secondary-foreground'
              }`}
            >
              {status.antiProbeEnabled ? '已启用' : '未启用'}
            </span>
          </div>
        </div>

        {/* TLS 指纹 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.tlsFingerprint ? (
              <CheckCircle className="w-4 h-4 text-green-500" />
            ) : (
              <AlertCircle className="w-4 h-4 text-yellow-500" />
            )}
            <h3 className="text-sm font-semibold">TLS 指纹伪装</h3>
          </div>

          <div className="flex justify-between items-center">
            <p className="text-sm">当前指纹</p>
            <span
              className={`px-2 py-1 rounded-full text-xs ${
                status.tlsFingerprint
                  ? 'bg-green-500 text-white'
                  : 'bg-secondary text-secondary-foreground'
              }`}
            >
              {status.tlsFingerprint || '未设置'}
            </span>
          </div>
        </div>

        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.egressIdentityEnabled ? (
              <CheckCircle className="w-4 h-4 text-green-500" />
            ) : (
              <AlertCircle className="w-4 h-4 text-yellow-500" />
            )}
            <h3 className="text-sm font-semibold">出口身份管理</h3>
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <p className="text-sm">状态</p>
              <span
                className={`px-2 py-1 rounded-full text-xs ${
                  status.egressIdentityEnabled
                    ? 'bg-green-500 text-white'
                    : 'bg-secondary text-secondary-foreground'
                }`}
              >
                {status.egressIdentityEnabled ? '已启用' : '未启用'}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <p className="text-sm">活跃 assignment</p>
              <span className="px-2 py-1 rounded-full text-xs bg-blue-500 text-white">
                {status.egressIdentityActiveAssignments}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <p className="text-sm">domain-pattern 回写</p>
              <span className="px-2 py-1 rounded-full text-xs bg-purple-500 text-white">
                {domainPatternAssignments.length}
              </span>
            </div>
          </div>
        </div>

        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.sessionAffinityEnabled ? (
              <CheckCircle className="w-4 h-4 text-green-500" />
            ) : (
              <AlertCircle className="w-4 h-4 text-yellow-500" />
            )}
            <h3 className="text-sm font-semibold">会话绑定</h3>
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <p className="text-sm">状态</p>
              <span
                className={`px-2 py-1 rounded-full text-xs ${
                  status.sessionAffinityEnabled
                    ? 'bg-green-500 text-white'
                    : 'bg-secondary text-secondary-foreground'
                }`}
              >
                {status.sessionAffinityEnabled ? '已启用' : '未启用'}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <p className="text-sm">活跃绑定</p>
              <span className="px-2 py-1 rounded-full text-xs bg-blue-500 text-white">
                {status.sessionAffinityActiveBindings}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <p className="text-sm">domain-rule 回写</p>
              <span className="px-2 py-1 rounded-full text-xs bg-purple-500 text-white">
                {domainRuleBindings.length}
              </span>
            </div>
          </div>
        </div>

        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {domainPatternAssignments.length > 0 || domainRuleBindings.length > 0 ? (
              <CheckCircle className="w-4 h-4 text-green-500" />
            ) : (
              <AlertCircle className="w-4 h-4 text-yellow-500" />
            )}
            <h3 className="text-sm font-semibold">稳定出口回写</h3>
          </div>

          <div className="space-y-2">
            <div className="flex justify-between items-center">
              <p className="text-sm">domain-pattern</p>
              <span className="px-2 py-1 rounded-full text-xs bg-purple-500 text-white">
                {domainPatternAssignments.length}
              </span>
            </div>
            <div className="flex justify-between items-center">
              <p className="text-sm">domain-rule</p>
              <span className="px-2 py-1 rounded-full text-xs bg-purple-500 text-white">
                {domainRuleBindings.length}
              </span>
            </div>
            <div className="text-xs text-gray-500 pt-1">
              该卡片汇总稳定组手动选择回写到 `egress_identity` 与 `session_affinity` 的统一运行态结果。
            </div>
          </div>
        </div>

        {/* 多路径路由 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.multipathEnabled ? (
              <CheckCircle className="w-4 h-4 text-green-500" />
            ) : (
              <AlertCircle className="w-4 h-4 text-yellow-500" />
            )}
            <h3 className="text-sm font-semibold">多路径路由</h3>
          </div>

          <div className="flex justify-between items-center">
            <p className="text-sm">状态</p>
            <span
              className={`px-2 py-1 rounded-full text-xs ${
                status.multipathEnabled
                  ? 'bg-green-500 text-white'
                  : 'bg-secondary text-secondary-foreground'
              }`}
            >
              {status.multipathEnabled ? '已启用' : '未启用'}
            </span>
          </div>
        </div>

        {/* XDP 代理（仅 Linux） */}
        {status.xdpEnabled !== undefined && (
          <div className="p-4 bg-card border border-border rounded-lg">
            <div className="flex items-center gap-2 mb-3">
              {status.xdpEnabled && status.xdpRunning ? (
                <CheckCircle className="w-4 h-4 text-green-500" />
              ) : status.xdpEnabled ? (
                <AlertCircle className="w-4 h-4 text-yellow-500" />
              ) : (
                <AlertCircle className="w-4 h-4 text-yellow-500" />
              )}
              <h3 className="text-sm font-semibold">XDP 代理</h3>
            </div>

            <div className="flex justify-between items-center">
              <p className="text-sm">状态</p>
              <span
                className={`px-2 py-1 rounded-full text-xs ${
                  status.xdpEnabled && status.xdpRunning
                    ? 'bg-green-500 text-white'
                    : status.xdpEnabled
                    ? 'bg-yellow-500 text-white'
                    : 'bg-secondary text-secondary-foreground'
                }`}
              >
                {status.xdpEnabled
                  ? status.xdpRunning
                    ? '运行中'
                    : '已启用但未运行'
                  : '未启用'}
              </span>
            </div>
          </div>
        )}
      </div>

      {/* 性能提示 */}
      <div className="p-4 bg-card border border-border rounded-lg mt-4">
        <h3 className="text-sm font-semibold mb-3">性能优化建议</h3>

        <div className="space-y-2">
          {!status.securityEnabled && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              建议启用安全监控以保护您的代理
            </div>
          )}

          {!status.antiProbeEnabled && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              建议启用反探测以防止主动探测
            </div>
          )}

          {!status.egressIdentityEnabled && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              建议启用出口身份管理以统一应用、快捷方式和会话的出口画像
            </div>
          )}

          {!status.sessionAffinityEnabled && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              建议启用会话绑定以把稳定出口选择持续映射到域名、进程和连接会话
            </div>
          )}

          {!status.tlsFingerprint && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              建议设置 TLS 指纹伪装以提高隐蔽性
            </div>
          )}

          {status.xdpEnabled !== undefined && !status.xdpEnabled && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              Linux 系统可以启用 XDP 代理获得 10 倍性能提升
            </div>
          )}

          {status.securityEnabled &&
            status.antiProbeEnabled &&
            status.tlsFingerprint &&
            status.egressIdentityEnabled &&
            status.sessionAffinityEnabled &&
            status.multipathEnabled && (
              <div className="p-2 bg-green-500 text-white rounded text-xs">
                ✅ 所有高级功能已启用，您的代理处于最佳状态
              </div>
            )}
        </div>
      </div>
    </div>
  )
}
