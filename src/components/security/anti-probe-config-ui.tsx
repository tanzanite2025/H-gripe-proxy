/**
 * 反主动探测配置 UI 组件
 */

import { AlertTriangle, Copy, RefreshCw, Shield, Trash2 } from 'lucide-react'
import { Button, Switch, TextField } from '@/components/tailwind'
import type { AntiProbeConfig } from '@/services/anti-probe'

interface AntiProbeConfigUIProps {
  config: AntiProbeConfig
  token: string
  newIp: string
  saving: boolean
  onConfigChange: (config: AntiProbeConfig) => void
  onTokenGenerate: () => void
  onTokenCopy: () => void
  onNewIpChange: (ip: string) => void
  onAddIp: () => void
  onRemoveIp: (ip: string) => void
  onGenerateKey: () => void
  onSave: () => void
  onCleanup: () => void
}

export default function AntiProbeConfigUI({
  config,
  token,
  newIp,
  saving,
  onConfigChange,
  onTokenGenerate,
  onTokenCopy,
  onNewIpChange,
  onAddIp,
  onRemoveIp,
  onGenerateKey,
  onSave,
  onCleanup,
}: AntiProbeConfigUIProps) {
  return (
    <div className="p-6">
      <div className="space-y-6">
        {/* 标题 */}
        <div className="flex items-center gap-2">
          <Shield className="w-5 h-5 text-primary" />
          <h2 className="text-xl font-semibold">反主动探测配置</h2>
        </div>

        {/* 说明 */}
        <div className="p-4 bg-yellow-500 text-white rounded-lg">
          <div className="flex items-start gap-2">
            <AlertTriangle className="w-5 h-5 flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-sm">幻影无响应模式</p>
              <p className="text-xs opacity-90 mt-1">
                对未携带握手暗号的连接直接丢弃，不返回任何响应。在外部探测者看来，服务器就像一个完全不存在的"黑洞 IP"。
              </p>
            </div>
          </div>
        </div>

        {/* 基础配置 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <label className="text-sm font-medium">启用反主动探测</label>
              <Switch
                checked={config.enabled}
                onCheckedChange={(checked) =>
                  onConfigChange({ ...config, enabled: checked })
                }
              />
            </div>

            <div className="flex items-center justify-between">
              <label className="text-sm font-medium">
                严格模式（非白名单直接拒绝）
              </label>
              <Switch
                checked={config.strict_mode}
                onCheckedChange={(checked) =>
                  onConfigChange({ ...config, strict_mode: checked })
                }
                disabled={!config.enabled}
              />
            </div>

            <TextField
              label="时间窗口（秒）"
              type="number"
              value={config.time_window.toString()}
              onChange={(e) =>
                onConfigChange({
                  ...config,
                  time_window: Number.parseInt(e.target.value),
                })
              }
              disabled={!config.enabled}
              helperText="握手暗号的有效时间"
              fullWidth
            />
          </div>
        </div>

        {/* 密钥管理 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="space-y-4">
            <h3 className="text-sm font-semibold">私钥管理</h3>
            <TextField
              label="私钥"
              value={config.secret_key}
              onChange={(e) =>
                onConfigChange({ ...config, secret_key: e.target.value })
              }
              disabled={!config.enabled}
              fullWidth
              readOnly
              className="font-mono text-sm"
            />
            <Button
              variant="outline"
              onClick={onGenerateKey}
              disabled={!config.enabled}
              className="w-full"
            >
              <RefreshCw className="w-4 h-4 mr-2" />
              生成新密钥
            </Button>
          </div>
        </div>

        {/* 握手暗号生成 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="space-y-4">
            <h3 className="text-sm font-semibold">握手暗号生成</h3>
            <Button
              variant="default"
              onClick={onTokenGenerate}
              disabled={!config.enabled}
              className="w-full"
            >
              <RefreshCw className="w-4 h-4 mr-2" />
              生成握手暗号
            </Button>
            {token && (
              <div className="space-y-2">
                <div className="relative">
                  <TextField
                    label="当前暗号"
                    value={token}
                    fullWidth
                    readOnly
                    className="font-mono text-sm"
                  />
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={onTokenCopy}
                    className="absolute right-2 top-8"
                  >
                    <Copy className="w-4 h-4 mr-1" />
                    复制
                  </Button>
                </div>
                <p className="text-xs text-muted-foreground">
                  此暗号在 {config.time_window} 秒内有效
                </p>
              </div>
            )}
          </div>
        </div>

        {/* 白名单管理 */}
        <div className="p-4 bg-card border border-border rounded-lg">
          <div className="space-y-4">
            <h3 className="text-sm font-semibold">IP 白名单</h3>
            <div className="flex gap-2">
              <TextField
                label="添加 IP 地址"
                value={newIp}
                onChange={(e) => onNewIpChange(e.target.value)}
                onKeyPress={(e) => e.key === 'Enter' && onAddIp()}
                disabled={!config.enabled}
                placeholder="192.168.1.1 或 2001:db8::1"
                fullWidth
              />
              <Button
                variant="default"
                onClick={onAddIp}
                disabled={!config.enabled}
                className="shrink-0"
              >
                添加
              </Button>
            </div>
            <div className="flex flex-wrap gap-2">
              {config.whitelist.map((ip) => (
                <div
                  key={ip}
                  className="inline-flex items-center gap-1 px-3 py-1 bg-secondary text-secondary-foreground rounded-full text-sm"
                >
                  <span>{ip}</span>
                  <button
                    onClick={() => onRemoveIp(ip)}
                    disabled={!config.enabled}
                    className="hover:text-destructive disabled:opacity-50"
                  >
                    <Trash2 className="w-3 h-3" />
                  </button>
                </div>
              ))}
              {config.whitelist.length === 0 && (
                <p className="text-sm text-muted-foreground">暂无白名单 IP</p>
              )}
            </div>
          </div>
        </div>

        {/* 操作按钮 */}
        <div className="flex gap-4">
          <Button
            variant="default"
            onClick={onSave}
            disabled={saving}
            className="flex-1"
          >
            保存配置
          </Button>
          <Button
            variant="outline"
            onClick={onCleanup}
            disabled={!config.enabled}
          >
            <Trash2 className="w-4 h-4 mr-2" />
            清理缓存
          </Button>
        </div>
      </div>
    </div>
  )
}
