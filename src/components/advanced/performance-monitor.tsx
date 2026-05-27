/**
 * 性能监控面板
 */

import { AlertCircle, CheckCircle, RefreshCw } from 'lucide-react'
import { Button } from '@/components/tailwind'
import type { CoordinatorStatus } from '@/services/coordinator'

interface Props {
  status: CoordinatorStatus | null
  onRefresh: () => void
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

  return (
    <div>
      {/* 安全状态警告 */}
      {status.security_compromised && (
        <div className="p-4 bg-red-500 text-white rounded-lg mb-4">
          <p className="font-semibold text-sm mb-1">⚠️ 安全状态已被破坏</p>
          <p className="text-xs opacity-90">
            检测到调试器或恶意扫描。建议立即停止使用并检查系统安全。
          </p>
        </div>
      )}

      {/* 刷新按钮 */}
      <div className="flex justify-end mb-4">
        <Button variant="outline" size="sm" onClick={onRefresh}>
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
            {status.security_enabled && !status.security_compromised ? (
              <CheckCircle className="w-4 h-4 text-green-500" />
            ) : status.security_compromised ? (
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
                status.security_enabled
                  ? status.security_compromised
                    ? 'bg-red-500 text-white'
                    : 'bg-green-500 text-white'
                  : 'bg-secondary text-secondary-foreground'
              }`}
            >
              {status.security_enabled
                ? status.security_compromised
                  ? '已破坏'
                  : '运行中'
                : '未启用'}
            </span>
          </div>
        </div>

        {/* 反探测 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.anti_probe_enabled ? (
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
                status.anti_probe_enabled
                  ? 'bg-green-500 text-white'
                  : 'bg-secondary text-secondary-foreground'
              }`}
            >
              {status.anti_probe_enabled ? '已启用' : '未启用'}
            </span>
          </div>
        </div>

        {/* TLS 指纹 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.tls_fingerprint ? (
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
                status.tls_fingerprint
                  ? 'bg-green-500 text-white'
                  : 'bg-secondary text-secondary-foreground'
              }`}
            >
              {status.tls_fingerprint || '未设置'}
            </span>
          </div>
        </div>

        {/* 多路径路由 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="flex items-center gap-2 mb-3">
            {status.multipath_enabled ? (
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
                status.multipath_enabled
                  ? 'bg-green-500 text-white'
                  : 'bg-secondary text-secondary-foreground'
              }`}
            >
              {status.multipath_enabled ? '已启用' : '未启用'}
            </span>
          </div>
        </div>

        {/* XDP 代理（仅 Linux） */}
        {status.xdp_enabled !== undefined && (
          <div className="p-4 bg-card border border-border rounded-lg">
            <div className="flex items-center gap-2 mb-3">
              {status.xdp_enabled && status.xdp_running ? (
                <CheckCircle className="w-4 h-4 text-green-500" />
              ) : status.xdp_enabled ? (
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
                  status.xdp_enabled && status.xdp_running
                    ? 'bg-green-500 text-white'
                    : status.xdp_enabled
                    ? 'bg-yellow-500 text-white'
                    : 'bg-secondary text-secondary-foreground'
                }`}
              >
                {status.xdp_enabled
                  ? status.xdp_running
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
          {!status.security_enabled && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              建议启用安全监控以保护您的代理
            </div>
          )}

          {!status.anti_probe_enabled && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              建议启用反探测以防止主动探测
            </div>
          )}

          {!status.tls_fingerprint && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              建议设置 TLS 指纹伪装以提高隐蔽性
            </div>
          )}

          {status.xdp_enabled !== undefined && !status.xdp_enabled && (
            <div className="p-2 bg-blue-500 text-white rounded text-xs">
              Linux 系统可以启用 XDP 代理获得 10 倍性能提升
            </div>
          )}

          {status.security_enabled &&
            status.anti_probe_enabled &&
            status.tls_fingerprint &&
            status.multipath_enabled && (
              <div className="p-2 bg-green-500 text-white rounded text-xs">
                ✅ 所有高级功能已启用，您的代理处于最佳状态
              </div>
            )}
        </div>
      </div>
    </div>
  )
}
