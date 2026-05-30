/**
 * 出口 IP 监控配置面板
 */

import { useLockFn } from 'ahooks'
import { useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Select } from '@/components/tailwind/Select'
import type { SelectPrimitiveValue } from '@/components/tailwind/Select'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import type {
  EgressMonitorConfig,
  EgressMonitorStats,
  RebindStrategyType,
} from '@/services/coordinator'
import {
  egressMonitorGetStats,
  egressMonitorIsRunning,
  egressMonitorProbeNow,
  egressMonitorResetStats,
  egressMonitorStart,
  egressMonitorStop,
} from '@/services/egress-monitor'
import { showNotice } from '@/services/notice-service'

interface Props {
  config: EgressMonitorConfig
  onChange: (config: EgressMonitorConfig) => void
}

function formatUptime(secs: number): string {
  const h = Math.floor(secs / 3600)
  const m = Math.floor((secs % 3600) / 60)
  const s = secs % 60
  if (h > 0) return `${h}h ${m}m ${s}s`
  if (m > 0) return `${m}m ${s}s`
  return `${s}s`
}

function formatTimestamp(ms: number): string {
  return new Date(ms).toLocaleTimeString()
}

export function EgressMonitorPanel({ config, onChange }: Props) {
  const [stats, setStats] = useState<EgressMonitorStats | null>(null)
  const [running, setRunning] = useState<boolean | null>(null)
  const [probing, setProbing] = useState(false)

  const refreshStats = useLockFn(async () => {
    try {
      const [s, r] = await Promise.all([
        egressMonitorGetStats(),
        egressMonitorIsRunning(),
      ])
      setStats(s)
      setRunning(r)
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      showNotice('error', msg)
    }
  })

  const handleProbeNow = useLockFn(async () => {
    setProbing(true)
    try {
      const result = await egressMonitorProbeNow()
      showNotice('success', `探测成功: ${result.ip} (${result.countryCode ?? '??'}) - ${result.latencyMs}ms`)
      void refreshStats()
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      showNotice('error', `探测失败: ${msg}`)
    } finally {
      setProbing(false)
    }
  })

  const handleToggleRunning = useLockFn(async () => {
    try {
      if (running) {
        await egressMonitorStop()
        setRunning(false)
        showNotice('success', '出口监控已停止')
      } else {
        await egressMonitorStart()
        setRunning(true)
        showNotice('success', '出口监控已启动')
      }
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      showNotice('error', msg)
    }
  })

  const handleResetStats = useLockFn(async () => {
    try {
      await egressMonitorResetStats()
      void refreshStats()
      showNotice('success', '统计已重置')
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err)
      showNotice('error', msg)
    }
  })

  const update = (patch: Partial<EgressMonitorConfig>) => {
    onChange({ ...config, ...patch })
  }

  const strategyOptions: { value: SelectPrimitiveValue; label: string }[] = [
    { value: 'smart', label: 'Smart (同画像优先)' },
    { value: 'round-robin', label: 'Round Robin (简单轮转)' },
  ]

  return (
    <div className="space-y-4">
      {/* 基本配置 */}
      <Card variant="outlined">
        <div className="p-4 space-y-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <h3 className="text-sm font-medium">出口 IP 监控</h3>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                定期探测出口 IP，IP 变化时自动重绑定到同画像节点
              </p>
            </div>
            <Switch
              label="启用"
              checked={config.enabled}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => update({ enabled: e.target.checked })}
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <TextField
              label="探测间隔 (秒)"
              type="number"
              value={String(config.probeIntervalSecs)}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => update({ probeIntervalSecs: Number(e.target.value) || 120 })}
              disabled={!config.enabled}
            />
            <TextField
              label="探测超时 (秒)"
              type="number"
              value={String(config.probeTimeoutSecs)}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => update({ probeTimeoutSecs: Number(e.target.value) || 10 })}
              disabled={!config.enabled}
            />
          </div>

          <div className="grid grid-cols-2 gap-3">
            <TextField
              label="代理组轮询间隔 (秒)"
              type="number"
              value={String(config.watchPollIntervalSecs)}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => update({ watchPollIntervalSecs: Math.max(5, Number(e.target.value) || 30) })}
              disabled={!config.enabled}
              helperText="最小 5 秒"
            />
            <TextField
              label="回写防抖窗口 (秒)"
              type="number"
              value={String(config.watchDebounceSecs)}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => update({ watchDebounceSecs: Math.max(5, Number(e.target.value) || 10) })}
              disabled={!config.enabled}
              helperText="两次回写最小间隔，防止频繁切换时回写风暴"
            />
            <div>
              <label className="mb-1 block text-sm text-gray-600 dark:text-gray-400">
                重绑定策略
              </label>
              <Select
                value={config.rebindStrategy}
                onChange={(v) => update({ rebindStrategy: v as RebindStrategyType })}
                options={strategyOptions}
                disabled={!config.enabled}
              />
            </div>
          </div>

          <Switch
            label="IP 变化时自动重绑定"
            checked={config.autoRebindOnChange}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => update({ autoRebindOnChange: e.target.checked })}
            disabled={!config.enabled}
          />

          <Switch
            label="IP 变化时通知前端"
            checked={config.notifyOnChange}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => update({ notifyOnChange: e.target.checked })}
            disabled={!config.enabled}
          />
        </div>
      </Card>

      {/* 运行状态 */}
      <Card variant="outlined">
        <div className="p-4 space-y-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <h3 className="text-sm font-medium">运行状态</h3>
              <p className="text-xs text-gray-500 dark:text-gray-400">
                监控运行状态与统计信息
              </p>
            </div>
          </div>

          <div className="flex items-center gap-3">
            <Button
              variant={running ? 'outlined' : 'primary'}
              size="small"
              onClick={handleToggleRunning}
            >
              {running ? '停止监控' : '启动监控'}
            </Button>
            <Button
              variant="outlined"
              size="small"
              onClick={handleProbeNow}
              disabled={probing}
            >
              {probing ? '探测中...' : '手动探测'}
            </Button>
            <Button
              variant="outlined"
              size="small"
              onClick={refreshStats}
            >
              刷新状态
            </Button>
            <Button
              variant="outlined"
              size="small"
              onClick={handleResetStats}
            >
              重置统计
            </Button>
          </div>

          {stats && (
            <div className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm">
              <div>
                <span className="text-gray-500 dark:text-gray-400">运行状态</span>
                <span className="ml-2 font-medium">
                  {running ? (
                    <span className="text-green-600 dark:text-green-400">运行中</span>
                  ) : (
                    <span className="text-gray-400">已停止</span>
                  )}
                </span>
              </div>
              <div>
                <span className="text-gray-500 dark:text-gray-400">运行时长</span>
                <span className="ml-2 font-medium">{formatUptime(stats.uptimeSecs)}</span>
              </div>
              <div>
                <span className="text-gray-500 dark:text-gray-400">总探测</span>
                <span className="ml-2 font-medium">{stats.totalProbes}</span>
              </div>
              <div>
                <span className="text-gray-500 dark:text-gray-400">成功/失败</span>
                <span className="ml-2 font-medium">
                  {stats.successfulProbes} / {stats.failedProbes}
                </span>
              </div>
              <div>
                <span className="text-gray-500 dark:text-gray-400">IP 变化次数</span>
                <span className="ml-2 font-medium">{stats.ipChangeCount}</span>
              </div>
              <div>
                <span className="text-gray-500 dark:text-gray-400">自动重绑定次数</span>
                <span className="ml-2 font-medium">{stats.autoRebindCount}</span>
              </div>
            </div>
          )}

          {stats?.lastProbe && (
            <div className="rounded-md bg-gray-50 p-3 dark:bg-gray-800">
              <div className="mb-1 text-xs font-medium text-gray-500 dark:text-gray-400">
                最近探测
              </div>
              <div className="text-sm">
                <span className="font-mono">{stats.lastProbe.ip}</span>
                <span className="ml-2 text-gray-500">
                  {stats.lastProbe.countryCode ?? '??'}
                </span>
                <span className="ml-2 text-gray-500">
                  {stats.lastProbe.latencyMs}ms
                </span>
                <span className="ml-2 text-gray-400">
                  {formatTimestamp(stats.lastProbe.probedAtMs)}
                </span>
              </div>
            </div>
          )}

          {stats?.lastChange && (
            <div className="rounded-md border border-amber-200 bg-amber-50 p-3 dark:border-amber-800 dark:bg-amber-950">
              <div className="mb-1 text-xs font-medium text-amber-600 dark:text-amber-400">
                最近 IP 变化
              </div>
              <div className="text-sm">
                <span className="font-mono">{stats.lastChange.previousIp}</span>
                <span className="mx-1">&rarr;</span>
                <span className="font-mono">{stats.lastChange.currentIp}</span>
                <span className="ml-2 text-gray-500">
                  ({stats.lastChange.previousCountry ?? '??'} &rarr;{' '}
                  {stats.lastChange.currentCountry ?? '??'})
                </span>
                {stats.lastChange.autoRebindApplied && (
                  <span className="ml-2 text-green-600 dark:text-green-400">
                    已自动重绑定
                  </span>
                )}
                <span className="ml-2 text-gray-400">
                  {formatTimestamp(stats.lastChange.timestampMs)}
                </span>
              </div>
            </div>
          )}

          {!stats && (
            <div className="text-sm text-gray-400">
              点击「刷新状态」查看运行状态
            </div>
          )}
        </div>
      </Card>
    </div>
  )
}
