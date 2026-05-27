/**
 * 多路径路由配置面板
 */

import { AlertTriangle } from 'lucide-react'
import { Switch, Select, Button } from '@/components/tailwind'
import type { MultipathConfig, SlicingStrategy, PoolType } from '@/services/coordinator'

interface Props {
  config: MultipathConfig
  onChange: (config: MultipathConfig) => void
}

const strategyLabels: Record<SlicingStrategy, string> = {
  RoundRobin: '轮询（均匀分配）',
  Random: '随机',
  Weighted: '加权（推荐）',
  LeastConnections: '最少连接',
  LatencyBased: '延迟优先',
}

const poolTypeLabels: Record<PoolType, string> = {
  General: '通用池',
  Streaming: '流媒体专用',
  Gaming: '游戏专用',
  Download: '下载专用',
  Social: '社交媒体',
}

export function MultipathConfigPanel({ config, onChange }: Props) {
  return (
    <div>
      <div className="p-4 bg-yellow-500 text-white rounded-lg mb-4">
        <div className="flex items-start gap-2">
          <AlertTriangle className="w-5 h-5 flex-shrink-0 mt-0.5" />
          <p className="text-sm">
            ⚠️ 多路径路由会将流量分片到多个节点。流媒体和游戏服务会自动使用单节点模式，避免 IP 变化导致封号。
          </p>
        </div>
      </div>

      {/* 总开关 */}
      <div className="p-4 bg-card border border-border rounded-lg mb-4">
        <div className="flex items-center justify-between">
          <div>
            <p className="font-bold">启用多路径路由</p>
            <p className="text-xs text-muted-foreground">
              将流量分片到多个节点，降维打击行为分析
            </p>
          </div>
          <Switch
            checked={config.enabled}
            onCheckedChange={(checked) =>
              onChange({ ...config, enabled: checked })
            }
          />
        </div>
      </div>

      {config.enabled && (
        <>
          {/* 分片策略 */}
          <div className="p-4 bg-card border border-border rounded-lg mb-4">
            <h3 className="text-lg font-semibold mb-4">分片策略</h3>

            <Select
              label="策略"
              value={config.strategy}
              onChange={(e) =>
                onChange({
                  ...config,
                  strategy: e.target.value as SlicingStrategy,
                })
              }
              fullWidth
            >
              {Object.entries(strategyLabels).map(([value, label]) => (
                <option key={value} value={value}>
                  {label}
                </option>
              ))}
            </Select>

            <p className="text-xs text-muted-foreground mt-2">
              {config.strategy === 'Weighted' && '推荐：根据节点权重分配流量'}
              {config.strategy === 'RoundRobin' && '轮询：均匀分配到所有节点'}
              {config.strategy === 'Random' && '随机：完全随机选择节点'}
              {config.strategy === 'LeastConnections' && '最少连接：选择连接数最少的节点'}
              {config.strategy === 'LatencyBased' && '延迟优先：选择延迟最低的节点'}
            </p>
          </div>

          {/* 会话保持 */}
          <div className="p-4 bg-card border border-border rounded-lg mb-4">
            <div className="flex items-center justify-between">
              <div>
                <p className="font-bold">会话保持</p>
                <p className="text-xs text-muted-foreground">
                  同一会话使用相同节点（推荐开启）
                </p>
              </div>
              <Switch
                checked={config.session_persistence}
                onCheckedChange={(checked) =>
                  onChange({
                    ...config,
                    session_persistence: checked,
                  })
                }
              />
            </div>
          </div>

          {/* 节点池 */}
          <div className="p-4 bg-card border border-border rounded-lg mb-4">
            <div className="flex justify-between items-center mb-4">
              <h3 className="text-lg font-semibold">节点池</h3>
              <Button variant="outline" size="sm">
                添加节点池
              </Button>
            </div>

            {config.node_pools.length === 0 ? (
              <div className="p-3 bg-blue-500 text-white rounded-lg">
                <p className="text-sm">
                  还没有节点池。点击"添加节点池"开始配置。
                </p>
              </div>
            ) : (
              <div className="space-y-4">
                {config.node_pools.map((pool, index) => (
                  <div key={index} className="p-4 bg-card border border-border rounded-lg">
                    <div className="flex justify-between items-center">
                      <div>
                        <p className="font-bold">{pool.name}</p>
                        <div className="flex gap-2 mt-2">
                          <span className="px-2 py-1 bg-primary text-primary-foreground rounded-full text-xs">
                            {poolTypeLabels[pool.pool_type]}
                          </span>
                          <span className="px-2 py-1 bg-secondary text-secondary-foreground rounded-full text-xs">
                            {pool.nodes.length} 个节点
                          </span>
                          <span
                            className={`px-2 py-1 rounded-full text-xs ${
                              pool.enabled
                                ? 'bg-green-500 text-white'
                                : 'bg-secondary text-secondary-foreground'
                            }`}
                          >
                            {pool.enabled ? '已启用' : '已禁用'}
                          </span>
                        </div>
                      </div>
                      <div className="flex gap-2">
                        <Button size="sm" variant="ghost">
                          编辑
                        </Button>
                        <Button size="sm" variant="ghost">
                          删除
                        </Button>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* 预定义规则 */}
          <div className="p-4 bg-card border border-border rounded-lg">
            <h3 className="text-lg font-semibold mb-2">会话绑定规则</h3>
            <p className="text-sm text-muted-foreground mb-4">
              以下服务会自动使用单节点模式，避免 IP 变化
            </p>

            <div className="space-y-2">
              <div className="p-3 bg-green-500 text-white rounded-lg">
                <p className="font-semibold text-sm">流媒体服务（强制单节点）</p>
                <p className="text-xs opacity-90">
                  Netflix, YouTube, Hulu, Disney+, Prime Video
                </p>
              </div>

              <div className="p-3 bg-green-500 text-white rounded-lg">
                <p className="font-semibold text-sm">游戏服务（强制单节点）</p>
                <p className="text-xs opacity-90">
                  Steam, Epic Games, Riot Games, Blizzard
                </p>
              </div>

              <div className="p-3 bg-blue-500 text-white rounded-lg">
                <p className="font-semibold text-sm">社交媒体（建议单节点）</p>
                <p className="text-xs opacity-90">
                  Twitter, Facebook, Instagram
                </p>
              </div>

              <div className="p-3 bg-blue-500 text-white rounded-lg">
                <p className="font-semibold text-sm">下载服务（可多路径）</p>
                <p className="text-xs opacity-90">GitHub, CDN 等</p>
              </div>
            </div>
          </div>
        </>
      )}
    </div>
  )
}
