import type {
  BufferPoolStats,
  EngineStats,
  HotReloadStatus,
  PerfStats,
  RuleTrafficSnapshot,
  TLSFingerprintStats,
  XDPStatus,
} from 'clash-dtos'
import { Activity, RefreshCw } from 'lucide-react'
import { useCallback, useEffect, useMemo, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import {
  forceRuntimeTlsRotation,
  getRuntimeBufferPoolStats,
  getRuntimeEngineStats,
  getRuntimeHotReloadStatus,
  getRuntimePerfStats,
  getRuntimeRuleTraffic,
  getRuntimeTlsFingerprintStats,
  getRuntimeXdpStatus,
} from '@/services/core-runtime'

function formatBytes(value: number): string {
  if (!Number.isFinite(value) || value <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const exp = Math.min(Math.floor(Math.log(value) / Math.log(1024)), units.length - 1)
  return `${(value / 1024 ** exp).toFixed(exp === 0 ? 0 : 2)} ${units[exp]}`
}

interface TelemetryState {
  engine: EngineStats | null
  perf: PerfStats | null
  buffer: BufferPoolStats | null
  hotReload: HotReloadStatus | null
  xdp: XDPStatus | null
  ruleTraffic: Record<string, RuleTrafficSnapshot> | null
  tls: TLSFingerprintStats | null
}

function StatGrid({ rows }: { rows: [string, string][] }) {
  return (
    <div className="grid gap-x-4 gap-y-1 sm:grid-cols-2 lg:grid-cols-3">
      {rows.map(([label, value]) => (
        <div key={label} className="flex justify-between gap-2">
          <span className="text-muted-foreground">{label}</span>
          <span className="font-medium">{value}</span>
        </div>
      ))}
    </div>
  )
}

function Section({
  title,
  available,
  children,
}: {
  title: string
  available: boolean
  children?: React.ReactNode
}) {
  return (
    <div className="space-y-2 rounded-md bg-muted/40 px-3 py-2 text-xs">
      <div className="flex items-center gap-2 font-semibold">
        {title}
        {!available ? (
          <Chip size="small" color="default" label="不可用" />
        ) : null}
      </div>
      {available ? children : null}
    </div>
  )
}

export function TelemetryDiagnosticsPanel() {
  const [state, setState] = useState<TelemetryState>({
    engine: null,
    perf: null,
    buffer: null,
    hotReload: null,
    xdp: null,
    ruleTraffic: null,
    tls: null,
  })
  const [loading, setLoading] = useState(false)
  const [rotating, setRotating] = useState(false)

  const refresh = useCallback(async () => {
    setLoading(true)
    const [engine, perf, buffer, hotReload, xdp, ruleTraffic, tls] = await Promise.all([
      getRuntimeEngineStats().catch(() => null),
      getRuntimePerfStats().catch(() => null),
      getRuntimeBufferPoolStats().catch(() => null),
      getRuntimeHotReloadStatus().catch(() => null),
      getRuntimeXdpStatus().catch(() => null),
      getRuntimeRuleTraffic().catch(() => null),
      getRuntimeTlsFingerprintStats().catch(() => null),
    ])
    setState({ engine, perf, buffer, hotReload, xdp, ruleTraffic, tls })
    setLoading(false)
  }, [])

  const rotateTls = useCallback(async () => {
    try {
      setRotating(true)
      await forceRuntimeTlsRotation()
    } catch (error) {
      console.warn('force tls rotation failed', error)
    } finally {
      setRotating(false)
      void refresh()
    }
  }, [refresh])

  useEffect(() => {
    void refresh()
  }, [refresh])

  const topRuleTraffic = useMemo(() => {
    if (!state.ruleTraffic) return []
    return Object.values(state.ruleTraffic)
      .sort((a, b) => b.upload + b.download - (a.upload + a.download))
      .slice(0, 8)
  }, [state.ruleTraffic])

  const tlsUsage = useMemo(() => {
    if (!state.tls) return []
    return Object.entries(state.tls.usageSnapshot)
      .map(([name, count]) => [name, String(count ?? 0)] as [string, string])
      .sort((a, b) => Number(b[1]) - Number(a[1]))
  }, [state.tls])

  return (
    <Card>
      <div className="space-y-4 p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div>
            <div className="flex items-center gap-2 text-sm font-semibold">
              <Activity className="h-4 w-4" />
              数据面遥测诊断
            </div>
            <div className="mt-1 text-xs text-muted-foreground">
              通过 Rust 运行时命令读取 Mihomo 数据面遥测（引擎 / 性能 / 缓冲池 /
              热重载 / XDP / 规则流量 / TLS 指纹）。部分指标依赖具体内核构建，不支持时显示“不可用”。
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

        <Section title="引擎" available={state.engine !== null}>
          {state.engine ? (
            <StatGrid
              rows={[
                ['活跃连接', String(state.engine.activeConnections)],
                ['跟踪连接', String(state.engine.trackedConns)],
              ]}
            />
          ) : null}
        </Section>

        <Section title="性能" available={state.perf !== null}>
          {state.perf ? (
            <StatGrid
              rows={[
                ['Goroutines', String(state.perf.goroutines)],
                ['GOGC', String(state.perf.gogc)],
                ['内存上限', formatBytes(state.perf.memLimit)],
                ['Heap Alloc', formatBytes(state.perf.heapAlloc)],
                ['Heap Sys', formatBytes(state.perf.heapSys)],
                ['Heap InUse', formatBytes(state.perf.heapInUse)],
                ['Stack InUse', formatBytes(state.perf.stackInUse)],
                ['GC 次数', String(state.perf.numGc)],
                ['GC 暂停总计', `${state.perf.gcPauseTotal} ns`],
                ['受保护连接', String(state.perf.protectedConns)],
                ['规则版本', state.perf.ruleVersion || '-'],
              ]}
            />
          ) : null}
        </Section>

        <Section title="缓冲池" available={state.buffer !== null}>
          {state.buffer ? (
            <StatGrid
              rows={[
                ['总分配', formatBytes(state.buffer.totalAlloc)],
                ['总归还', formatBytes(state.buffer.totalReturn)],
                ['总浪费', formatBytes(state.buffer.totalWaste)],
                ['分配错误', String(state.buffer.allocErrors)],
                ['Size Class 数', String(state.buffer.sizeClasses.length)],
              ]}
            />
          ) : null}
        </Section>

        <Section title="热重载" available={state.hotReload !== null}>
          {state.hotReload ? (
            <StatGrid
              rows={[
                ['规则版本', state.hotReload.ruleVersion || '-'],
                ['受保护连接', String(state.hotReload.protectedConns)],
                ['XDP 已加载', state.hotReload.xdpLoaded ? '是' : '否'],
              ]}
            />
          ) : null}
        </Section>

        <Section title="XDP" available={state.xdp !== null}>
          {state.xdp ? (
            <StatGrid
              rows={[
                ['已加载', state.xdp.loaded ? '是' : '否'],
                ['已启用', state.xdp.enabled ? '是' : '否'],
              ]}
            />
          ) : null}
        </Section>

        <Section title="规则流量 (Top 8)" available={state.ruleTraffic !== null}>
          {topRuleTraffic.length === 0 ? (
            <div className="text-muted-foreground">暂无规则流量数据</div>
          ) : (
            <div className="space-y-1">
              {topRuleTraffic.map((item, index) => (
                <div
                  key={`${item.ruleType}-${item.rulePayload}-${index}`}
                  className="grid gap-2 lg:grid-cols-[minmax(0,1fr)_auto]"
                >
                  <span className="truncate text-muted-foreground">
                    {item.ruleType} · {item.rulePayload}
                  </span>
                  <span className="font-medium">
                    ↑{formatBytes(item.upload)} ↓{formatBytes(item.download)} ·{' '}
                    {item.connections} 连接
                  </span>
                </div>
              ))}
            </div>
          )}
        </Section>

        <Section title="TLS 指纹" available={state.tls !== null}>
          {state.tls ? (
            <div className="space-y-2">
              <StatGrid
                rows={[
                  ['当前指纹', state.tls.currentFingerprint || '-'],
                  ['轮换次数', String(state.tls.rotationCount)],
                ]}
              />
              {tlsUsage.length > 0 ? (
                <div className="space-y-1">
                  <div className="text-muted-foreground">使用分布</div>
                  {tlsUsage.map(([name, count]) => (
                    <div key={name} className="flex justify-between gap-2">
                      <span className="truncate text-muted-foreground">{name}</span>
                      <span className="font-medium">{count}</span>
                    </div>
                  ))}
                </div>
              ) : null}
              <Button
                size="small"
                variant="outlined"
                disabled={rotating}
                onClick={() => void rotateTls()}
              >
                {rotating ? '轮换中...' : '强制轮换指纹'}
              </Button>
            </div>
          ) : null}
        </Section>
      </div>
    </Card>
  )
}
