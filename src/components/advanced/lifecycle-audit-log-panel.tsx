import dayjs from 'dayjs'
import { RefreshCw, ScrollText } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import {
  getRuntimeLifecycleState,
  type RuntimeLifecycleRecord,
} from '@/services/core-runtime'

const KIND_LABELS: Record<string, string> = {
  restart_core: '重启核心',
  reload_config: '重载配置',
  restart_app: '重启应用',
  update_geo: '更新 GEO',
  change_mode: '切换模式',
  toggle_system_proxy: '系统代理',
  toggle_tun: 'TUN 模式',
  apply_dns: 'DNS 配置',
  patch_sensitive_config: '敏感配置变更',
  tls_rotation: 'TLS 指纹轮换',
}

const DETAIL_LABELS: Record<string, string> = {
  on: '开启',
  off: '关闭',
  apply: '应用',
  revoke: '撤销',
  rule: '规则',
  global: '全局',
  direct: '直连',
  script: '脚本',
}

export function LifecycleAuditLogPanel() {
  const [records, setRecords] = useState<RuntimeLifecycleRecord[]>([])
  const [loading, setLoading] = useState(false)

  const refresh = useCallback(async () => {
    try {
      setLoading(true)
      const { records } = await getRuntimeLifecycleState()
      setRecords(records)
    } catch (error) {
      console.warn('failed to load runtime lifecycle state', error)
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const ordered = useMemo(
    () => [...records].sort((a, b) => b.updatedAt - a.updatedAt),
    [records],
  )

  return (
    <Card>
      <div className="space-y-4 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2 text-sm font-semibold">
              <ScrollText className="h-4 w-4" />
              核心生命周期审计日志
            </div>
            <div className="mt-1 text-xs text-muted-foreground">
              读取 Rust runtime 持久化的运行时变更事件（重启核心 / 重载配置 /
              重启应用 / 更新 GEO / 切换模式 / 系统代理 / TUN / DNS 配置 /
              敏感配置），来自 app-runtime/lifecycle-events.yaml，保留最近 100 条。
            </div>
          </div>
          <Button
            size="small"
            variant="outlined"
            onClick={() => void refresh()}
            disabled={loading}
            startIcon={
              <RefreshCw className={loading ? 'h-4 w-4 animate-spin' : 'h-4 w-4'} />
            }
          >
            {loading ? '刷新中...' : '刷新'}
          </Button>
        </div>

        {ordered.length === 0 ? (
          <div className="rounded-md bg-muted/40 px-3 py-6 text-center text-xs text-muted-foreground">
            暂无生命周期事件记录
          </div>
        ) : (
          <div className="space-y-1">
            {ordered.map((record, index) => (
              <div
                key={`${record.updatedAt}-${index}`}
                className="grid gap-2 rounded-md bg-muted/40 px-3 py-2 text-xs lg:grid-cols-[140px_minmax(0,1fr)_auto]"
              >
                <div className="font-medium">
                  {KIND_LABELS[record.kind] ?? record.kind}
                  {record.detail ? (
                    <span className="ml-1 text-muted-foreground">
                      · {DETAIL_LABELS[record.detail] ?? record.detail}
                    </span>
                  ) : null}
                </div>
                <div className="text-muted-foreground">
                  {record.error ? (
                    <span className="text-red-500">{record.error}</span>
                  ) : (
                    dayjs(record.updatedAt).format('YYYY-MM-DD HH:mm:ss')
                  )}
                </div>
                <div className="flex items-start justify-end">
                  <Chip
                    size="small"
                    color={record.success ? 'success' : 'error'}
                    label={record.success ? '成功' : '失败'}
                  />
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </Card>
  )
}
