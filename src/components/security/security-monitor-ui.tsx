/**
 * 安全监控 UI 组件
 */

import { AlertTriangle, Bug, Copy, Shield, Trash2 } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Button, Switch, TextField } from '@/components/tailwind'
import type { SecurityStatus } from '@/services/security'

interface SecurityMonitorUIProps {
  monitorEnabled: boolean
  status: SecurityStatus
  decoyPath: string
  encryptionKey: string
  hasEncryptionKey: boolean
  selfDestructConfirm: string
  onToggleMonitor: () => void
  onDecoyPathChange: (path: string) => void
  onDeployDecoy: () => void
  onCleanupDecoy: () => void
  onCheckDecoyAccess: () => void
  onGenerateKey: () => void
  onCopyKey: () => void
  onSelfDestructConfirmChange: (value: string) => void
  onSelfDestruct: () => void
}

export default function SecurityMonitorUI({
  monitorEnabled,
  status,
  decoyPath,
  encryptionKey,
  hasEncryptionKey,
  selfDestructConfirm,
  onToggleMonitor,
  onDecoyPathChange,
  onDeployDecoy,
  onCleanupDecoy,
  onCheckDecoyAccess,
  onGenerateKey,
  onCopyKey,
  onSelfDestructConfirmChange,
  onSelfDestruct,
}: SecurityMonitorUIProps) {
  return (
    <div className="p-6">
      <div className="space-y-6">
        {/* 标题 */}
        <div className="flex items-center gap-2">
          <Shield className="w-5 h-5 text-primary" />
          <h2 className="text-xl font-semibold">
            内生欺骗陷阱（Canary Honeytoken）
          </h2>
        </div>

        {/* 说明 */}
        <div className="p-4 bg-red-500 text-white rounded-lg">
          <div className="flex items-start gap-2">
            <AlertTriangle className="w-5 h-5 shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-sm">防御究极体</p>
              <p className="text-xs opacity-90 mt-1">
                反调试、内存蜜罐、配置欺骗、自毁机制 -
                全方位防范本地流氓软件扫描和物理攻破
              </p>
            </div>
          </div>
        </div>

        <div className="grid grid-cols-2 gap-4">
          {/* 安全状态 */}
          <div className="p-4 bg-card border border-border rounded-lg">
            <h3 className="text-sm font-semibold mb-4">安全状态监控</h3>
            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">启用安全监控</label>
                <Switch checked={monitorEnabled} onCheckedChange={onToggleMonitor} />
              </div>

              <div className="flex gap-2 flex-wrap">
                <div
                  className={`inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm ${
                    status.compromised
                      ? 'bg-red-500 text-white'
                      : 'bg-green-500 text-white'
                  }`}
                >
                  <Shield className="w-3 h-3" />
                  <span>{status.compromised ? '已破坏' : '安全'}</span>
                </div>
                <div
                  className={`inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm ${
                    status.debugger_present
                      ? 'bg-red-500 text-white'
                      : 'bg-secondary text-secondary-foreground'
                  }`}
                >
                  <Bug className="w-3 h-3" />
                  <span>
                    {status.debugger_present ? '检测到调试器' : '无调试器'}
                  </span>
                </div>
                <div
                  className={`inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm ${
                    status.memory_scanning
                      ? 'bg-red-500 text-white'
                      : 'bg-secondary text-secondary-foreground'
                  }`}
                >
                  <span>
                    {status.memory_scanning ? '检测到内存扫描' : '无内存扫描'}
                  </span>
                </div>
                <div
                  className={`inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm ${
                    status.leak_detected
                      ? 'bg-red-500 text-white'
                      : 'bg-secondary text-secondary-foreground'
                  }`}
                >
                  <span>
                    {status.leak_detected
                      ? `泄漏: ${status.leak_type ?? '未知'}`
                      : '无泄漏'}
                  </span>
                </div>
              </div>
            </div>
          </div>

          {/* 配置文件欺骗 */}
          <div className="p-4 bg-card border border-border rounded-lg">
            <h3 className="text-sm font-semibold mb-4">配置文件欺骗</h3>
            <div className="space-y-4">
              <TextField
                label="假配置文件路径"
                value={decoyPath}
                onChange={(event: ChangeEvent<HTMLInputElement>) => onDecoyPathChange(event.target.value)}
                fullWidth
                helperText="放置假配置文件来误导扫描软件"
              />
              <div className="flex gap-2">
                <Button variant="default" onClick={onDeployDecoy}>
                  部署假配置
                </Button>
                <Button variant="outline" onClick={onCheckDecoyAccess}>
                  检查访问
                </Button>
                <Button variant="outline" onClick={onCleanupDecoy}>
                  清除假配置
                </Button>
              </div>
            </div>
          </div>

          {/* 加密密钥管理 */}
          <div className="p-4 bg-card border border-border rounded-lg">
            <h3 className="text-sm font-semibold mb-4">加密密钥管理</h3>
            <div className="space-y-4">
              <div className="flex items-center gap-2">
                <div
                  className={`inline-flex items-center gap-1 px-3 py-1 rounded-full text-sm ${
                    hasEncryptionKey
                      ? 'bg-green-500 text-white'
                      : 'bg-yellow-500 text-white'
                  }`}
                >
                  <span>{hasEncryptionKey ? '密钥已设置' : '密钥未设置'}</span>
                </div>
                <p className="text-xs text-muted-foreground">
                  真实配置只在内存中加密存储
                </p>
              </div>

              <Button variant="default" onClick={onGenerateKey} className="w-full">
                生成新密钥
              </Button>

              {encryptionKey && (
                <div className="space-y-2">
                  <div className="relative">
                    <TextField
                      label="加密密钥（请保存到环境变量）"
                      value={encryptionKey}
                      fullWidth
                      readOnly
                      className="font-mono text-xs"
                    />
                    <Button
                      size="sm"
                      variant="ghost"
                      onClick={onCopyKey}
                      className="absolute right-2 top-8"
                    >
                      <Copy className="w-4 h-4 mr-1" />
                      复制
                    </Button>
                  </div>
                  <p className="text-xs text-yellow-600 dark:text-yellow-400">
                    请将此密钥设置为环境变量 CLASH_VERGE_SECURE_KEY
                  </p>
                </div>
              )}
            </div>
          </div>

          {/* 自毁机制 */}
          <div className="p-4 bg-card border-2 border-red-500 rounded-lg">
            <h3 className="text-sm font-semibold mb-4 text-red-500">
              🚨 紧急自毁机制
            </h3>
            <div className="space-y-4">
              <p className="text-sm text-muted-foreground">
                检测到安全威胁时，自动清除内存中的密钥、擦除本地缓存并退出程序
              </p>

              <TextField
                label="确认码"
                value={selfDestructConfirm}
                onChange={(event: ChangeEvent<HTMLInputElement>) => onSelfDestructConfirmChange(event.target.value)}
                placeholder="输入 CONFIRM_SELF_DESTRUCT"
                fullWidth
                helperText="手动触发自毁需要输入确认码"
              />

              <Button
                variant="destructive"
                onClick={onSelfDestruct}
                disabled={selfDestructConfirm !== 'CONFIRM_SELF_DESTRUCT'}
                className="w-full"
              >
                <Trash2 className="w-4 h-4 mr-2" />
                触发自毁
              </Button>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}
