import { Bug, Shield } from 'lucide-react'

import { Switch } from '@/components/tailwind'
import type { SecurityStatus } from '@/services/security'

interface SecurityStatusPanelProps {
  monitorEnabled: boolean
  status: SecurityStatus
  onToggleMonitor: () => void
}

export default function SecurityStatusPanel({
  monitorEnabled,
  status,
  onToggleMonitor,
}: SecurityStatusPanelProps) {
  return (
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
              status.compromised ? 'bg-red-500 text-white' : 'bg-green-500 text-white'
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
  )
}
