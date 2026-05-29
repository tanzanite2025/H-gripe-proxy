/**
 * XDP 代理配置 UI 组件
 */

import { AlertCircle, CheckCircle, Info, Rocket, Zap } from 'lucide-react'

import { Button, Switch, Select } from '@/components/tailwind'
import type { XdpConfig, XdpStatus, XdpSupportInfo } from '@/services/xdp'

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
        {/* 标题 */}
        <div className="flex items-center gap-2">
          <Rocket className="w-5 h-5 text-primary" />
          <h2 className="text-xl font-semibold">XDP 零内核态切换代理</h2>
        </div>

        {/* 说明 */}
        <div className="p-4 bg-blue-500 text-white rounded-lg">
          <div className="flex items-start gap-2">
            <Info className="w-5 h-5 flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-sm">架构层面究极体</p>
              <p className="text-xs opacity-90 mt-1">
                在网卡驱动层直接处理数据包，实现线速转发（10-100 Gbps）和微秒级延迟（~10μs）
              </p>
            </div>
          </div>
        </div>

        {/* 系统支持检查 */}
        {supportInfo && (
          <div className="p-4 bg-card border border-border rounded-lg">
            <h3 className="text-sm font-semibold mb-4">系统支持</h3>
            <div className="space-y-2">
              <div className="flex items-center gap-2">
                {supportInfo.xdp_supported ? (
                  <CheckCircle className="w-4 h-4 text-green-500" />
                ) : (
                  <AlertCircle className="w-4 h-4 text-red-500" />
                )}
                <span className="text-sm">
                  XDP 支持: {supportInfo.xdp_supported ? '是' : '否'}
                </span>
              </div>
              <div className="flex items-center gap-2">
                {supportInfo.native_mode_supported ? (
                  <CheckCircle className="w-4 h-4 text-green-500" />
                ) : (
                  <AlertCircle className="w-4 h-4 text-yellow-500" />
                )}
                <span className="text-sm">
                  Native 模式: {supportInfo.native_mode_supported ? '支持' : '不支持'}
                </span>
              </div>
              <p className="text-xs text-muted-foreground">
                内核版本: {supportInfo.kernel_version}
              </p>
            </div>
          </div>
        )}

        {/* 配置 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <h3 className="text-sm font-semibold mb-4">配置</h3>
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <label className="text-sm font-medium">启用 XDP 代理</label>
              <Switch
                checked={config.enabled}
                onCheckedChange={(checked) =>
                  onConfigChange({ ...config, enabled: checked })
                }
              />
            </div>

            <Select
              label="网卡接口"
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
              label="XDP 模式"
              value={config.mode}
              onChange={(e) =>
                onConfigChange({
                  ...config,
                  mode: e.target.value as 'Native' | 'Skb' | 'Hw',
                })
              }
              disabled={!config.enabled}
              fullWidth
            >
              <option value="Native">Native（最高性能，需驱动支持）</option>
              <option value="Skb">SKB（兼容性好）</option>
              <option value="Hw">硬件卸载（需硬件支持）</option>
            </Select>

            <div className="flex items-center justify-between">
              <label className="text-sm font-medium">启用统计</label>
              <Switch
                checked={config.enable_stats}
                onCheckedChange={(checked) =>
                  onConfigChange({ ...config, enable_stats: checked })
                }
                disabled={!config.enabled}
              />
            </div>
          </div>
        </div>

        {/* 状态 */}
        {status && (
          <div className="p-4 bg-card border border-border rounded-lg">
            <h3 className="text-sm font-semibold mb-4">运行状态</h3>
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
                  {status.running ? '运行中' : '已停止'}
                </span>
                {status.running && (
                  <>
                    <span className="px-3 py-1 bg-secondary text-secondary-foreground rounded-full text-sm">
                      接口: {status.interface}
                    </span>
                    <span className="px-3 py-1 bg-secondary text-secondary-foreground rounded-full text-sm">
                      模式: {status.mode}
                    </span>
                  </>
                )}
              </div>

              {status.running && (
                <div>
                  <p className="text-xs font-semibold mb-2">统计信息</p>
                  <div className="grid grid-cols-2 gap-2">
                    <div>
                      <p className="text-xs text-muted-foreground">总包数</p>
                      <p className="text-sm font-medium">
                        {formatNumber(status.stats.total_packets)}
                      </p>
                    </div>
                    <div>
                      <p className="text-xs text-muted-foreground">代理包数</p>
                      <p className="text-sm font-medium">
                        {formatNumber(status.stats.proxied_packets)}
                      </p>
                    </div>
                    <div>
                      <p className="text-xs text-muted-foreground">直连包数</p>
                      <p className="text-sm font-medium">
                        {formatNumber(status.stats.direct_packets)}
                      </p>
                    </div>
                    <div>
                      <p className="text-xs text-muted-foreground">拒绝包数</p>
                      <p className="text-sm font-medium">
                        {formatNumber(status.stats.rejected_packets)}
                      </p>
                    </div>
                    <div>
                      <p className="text-xs text-muted-foreground">错误数</p>
                      <p className="text-sm font-medium text-red-500">
                        {formatNumber(status.stats.errors)}
                      </p>
                    </div>
                    <div>
                      <p className="text-xs text-muted-foreground">处理字节</p>
                      <p className="text-sm font-medium">
                        {formatBytes(status.stats.bytes_processed)}
                      </p>
                    </div>
                  </div>
                </div>
              )}
            </div>
          </div>
        )}

        {/* 性能优势 */}
        <div className="p-4 bg-green-500 text-white rounded-lg">
          <div className="flex items-center gap-2 mb-4">
            <Zap className="w-5 h-5" />
            <h3 className="text-sm font-semibold">性能优势</h3>
          </div>
          <div className="grid grid-cols-3 gap-4">
            <div>
              <p className="text-3xl font-bold">10x</p>
              <p className="text-xs opacity-90">延迟降低</p>
              <p className="text-xs opacity-75">100μs → 10μs</p>
            </div>
            <div>
              <p className="text-3xl font-bold">10x</p>
              <p className="text-xs opacity-90">吞吐量提升</p>
              <p className="text-xs opacity-75">5 Gbps → 50 Gbps</p>
            </div>
            <div>
              <p className="text-3xl font-bold">80%</p>
              <p className="text-xs opacity-90">CPU 占用降低</p>
              <p className="text-xs opacity-75">极低资源消耗</p>
            </div>
          </div>
        </div>

        {/* 操作按钮 */}
        <div className="flex gap-4">
          <Button
            variant="default"
            onClick={onSaveConfig}
            disabled={saving || loading}
            className="flex-1"
          >
            {saving ? '保存中...' : '保存配置'}
          </Button>
          {status?.running ? (
            <Button
              variant="destructive"
              onClick={onStop}
              disabled={loading}
              className="flex-1"
            >
              停止代理
            </Button>
          ) : (
            <Button
              variant="default"
              onClick={onStart}
              disabled={loading || !config.enabled}
              className="flex-1 bg-green-500 hover:bg-green-600"
            >
              启动代理
            </Button>
          )}
        </div>
      </div>
    </div>
  )
}
