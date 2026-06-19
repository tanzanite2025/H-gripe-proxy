import dayjs from 'dayjs'
import { ArrowUpCircle, RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'
import type { CoreUpdaterChannel } from 'tauri-plugin-mihomo-api'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Checkbox } from '@/components/tailwind/Checkbox'
import { Chip } from '@/components/tailwind/Chip'
import { Select } from '@/components/tailwind/Select'
import {
  getRuntimeUpgradeHistory,
  getRuntimeVersion,
  type RuntimeLifecycleRecord,
  upgradeRuntimeCore,
  upgradeRuntimeGeo,
  upgradeRuntimeUi,
} from '@/services/core-runtime'

const KIND_LABELS: Record<string, string> = {
  core: '内核',
  ui: '控制面板 UI',
  geo: 'Geo 数据库',
}

const CHANNEL_OPTIONS = [
  { value: 'auto', label: '自动 (auto)' },
  { value: 'release', label: '正式版 (release)' },
  { value: 'alpha', label: '测试版 (alpha)' },
]

export function CoreUpgradePanel() {
  const [records, setRecords] = useState<RuntimeLifecycleRecord[]>([])
  const [version, setVersion] = useState<string>('')
  const [channel, setChannel] = useState<CoreUpdaterChannel>('auto')
  const [force, setForce] = useState(false)
  const [pending, setPending] = useState<string | null>(null)

  const refresh = useCallback(async () => {
    try {
      const [{ records }, ver] = await Promise.all([
        getRuntimeUpgradeHistory(),
        getRuntimeVersion().catch(() => null),
      ])
      setRecords(records)
      if (ver) setVersion(ver.version)
    } catch (error) {
      console.warn('failed to load upgrade history', error)
    }
  }, [])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const runUpgrade = useCallback(
    async (kind: string, fn: () => Promise<void>) => {
      try {
        setPending(kind)
        await fn()
      } catch (error) {
        console.warn(`upgrade ${kind} failed`, error)
      } finally {
        setPending(null)
        void refresh()
      }
    },
    [refresh],
  )

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
              <ArrowUpCircle className="h-4 w-4" />
              内核升级（Rust 门禁）
            </div>
            <div className="mt-1 text-xs text-muted-foreground">
              通过 Rust app-owned 命令触发 Mihomo 内核 / UI / Geo 升级，结果记录到
              app-runtime/core-upgrade-history.yaml，保留最近 50 条。当前内核版本：
              {version || '-'}
            </div>
          </div>
          <Button
            size="small"
            variant="outlined"
            onClick={() => void refresh()}
            startIcon={<RefreshCw className="h-4 w-4" />}
          >
            刷新
          </Button>
        </div>

        <div className="flex flex-wrap items-center gap-3 rounded-md bg-muted/40 px-3 py-3">
          <Select
            size="small"
            options={CHANNEL_OPTIONS}
            value={channel}
            onChange={(value) => setChannel(value as CoreUpdaterChannel)}
          />
          <label className="flex items-center gap-1 text-xs text-muted-foreground">
            <Checkbox
              size="small"
              checked={force}
              onChange={(_, checked) => setForce(checked)}
            />
            强制覆盖
          </label>
          <Button
            size="small"
            variant="contained"
            disabled={pending !== null}
            onClick={() =>
              void runUpgrade('core', () => upgradeRuntimeCore(channel, force))
            }
          >
            {pending === 'core' ? '升级内核中...' : '升级内核'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            disabled={pending !== null}
            onClick={() => void runUpgrade('ui', upgradeRuntimeUi)}
          >
            {pending === 'ui' ? '升级 UI 中...' : '升级 UI'}
          </Button>
          <Button
            size="small"
            variant="outlined"
            disabled={pending !== null}
            onClick={() => void runUpgrade('geo', upgradeRuntimeGeo)}
          >
            {pending === 'geo' ? '升级 Geo 中...' : '升级 Geo'}
          </Button>
        </div>

        {ordered.length === 0 ? (
          <div className="rounded-md bg-muted/40 px-3 py-6 text-center text-xs text-muted-foreground">
            暂无升级记录
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
                      · {record.detail}
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
