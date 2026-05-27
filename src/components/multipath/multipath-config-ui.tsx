/**
 * 多路径路由配置 UI 组件
 */

import { AlertTriangle, Download, Info, Plus, Route, Trash2 } from 'lucide-react'
import { Button, Switch, TextField, Select, Tabs, Tab } from '@/components/tailwind'
import type { MultipathConfig, NodePool, PoolType, SessionBinding, SlicingStrategy } from '@/services/multipath'

interface MultipathConfigUIProps {
  config: MultipathConfig
  bindings: SessionBinding[]
  predefinedBindings: SessionBinding[]
  tabValue: number
  saving: boolean
  loading: boolean
  onConfigChange: (config: MultipathConfig) => void
  onTabChange: (value: number) => void
  onSaveConfig: () => void
  onLoadRecommended: () => void
  onAddPool: () => void
  onRemovePool: (poolName: string) => void
  onExportNodes: (poolName: string) => void
  onAddNode: (poolName: string) => void
  onRemoveNode: (poolName: string, nodeName: string) => void
  getPoolTypeLabel: (type: PoolType) => string
  getStrategyLabel: (strategy: SlicingStrategy) => string
}

export default function MultipathConfigUI({
  config,
  bindings,
  predefinedBindings,
  tabValue,
  saving,
  loading,
  onConfigChange,
  onTabChange,
  onSaveConfig,
  onLoadRecommended,
  onAddPool,
  onRemovePool,
  onExportNodes,
  onAddNode,
  onRemoveNode,
  getPoolTypeLabel,
  getStrategyLabel,
}: MultipathConfigUIProps) {
  return (
    <div className="p-6">
      <div className="space-y-6">
        {/* 标题 */}
        <div className="flex items-center gap-2">
          <Route className="w-5 h-5 text-primary" />
          <h2 className="text-xl font-semibold">多路径阴影路由</h2>
        </div>

        {/* 警告说明 */}
        <div className="p-4 bg-yellow-500 text-white rounded-lg">
          <div className="flex items-start gap-2">
            <AlertTriangle className="w-5 h-5 flex-shrink-0 mt-0.5" />
            <div>
              <p className="font-semibold text-sm">重要：避免 IP 乱跳导致封号</p>
              <p className="text-xs opacity-90 mt-1">
                流媒体（Netflix、YouTube）、游戏、社交媒体等服务必须使用单节点模式，否则会因
                IP 变化被封号。系统已预配置安全规则。
              </p>
            </div>
          </div>
        </div>

        {/* 标签页 */}
        <div className="border-b border-divider">
          <Tabs value={tabValue} onChange={(_, v) => onTabChange(v)}>
            <Tab label="基础配置" value={0} />
            <Tab label="节点池管理" value={1} />
            <Tab label="会话绑定规则" value={2} />
          </Tabs>
        </div>

        {/* 基础配置 */}
        {tabValue === 0 && (
          <div className="space-y-4">
            <div className="p-4 bg-card border border-border rounded-lg">
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <label className="text-sm font-medium">启用多路径路由</label>
                  <Switch
                    checked={config.enabled}
                    onCheckedChange={(checked) =>
                      onConfigChange({ ...config, enabled: checked })
                    }
                  />
                </div>

                <Select
                  label="分片策略"
                  value={config.strategy}
                  onChange={(e) =>
                    onConfigChange({
                      ...config,
                      strategy: e.target.value as SlicingStrategy,
                    })
                  }
                  disabled={!config.enabled}
                  fullWidth
                >
                  <option value="RoundRobin">轮询（均匀分配）</option>
                  <option value="Random">随机</option>
                  <option value="Weighted">加权（推荐）</option>
                  <option value="LeastConnections">最少连接</option>
                  <option value="LatencyBased">延迟优先</option>
                </Select>

                <TextField
                  label="最小分片大小（字节）"
                  type="number"
                  value={config.min_fragment_size.toString()}
                  onChange={(e) =>
                    onConfigChange({
                      ...config,
                      min_fragment_size: Number.parseInt(e.target.value),
                    })
                  }
                  disabled={!config.enabled}
                  fullWidth
                />

                <TextField
                  label="最大分片大小（字节）"
                  type="number"
                  value={config.max_fragment_size.toString()}
                  onChange={(e) =>
                    onConfigChange({
                      ...config,
                      max_fragment_size: Number.parseInt(e.target.value),
                    })
                  }
                  disabled={!config.enabled}
                  fullWidth
                />

                <TextField
                  label="重组超时（毫秒）"
                  type="number"
                  value={config.reassembly_timeout.toString()}
                  onChange={(e) =>
                    onConfigChange({
                      ...config,
                      reassembly_timeout: Number.parseInt(e.target.value),
                    })
                  }
                  disabled={!config.enabled}
                  fullWidth
                />

                <div className="flex items-center justify-between">
                  <label className="text-sm font-medium">启用会话保持</label>
                  <Switch
                    checked={config.session_persistence}
                    onCheckedChange={(checked) =>
                      onConfigChange({
                        ...config,
                        session_persistence: checked,
                      })
                    }
                    disabled={!config.enabled}
                  />
                </div>
              </div>
            </div>

            <div className="p-4 bg-blue-500 text-white rounded-lg">
              <div className="flex items-start gap-2">
                <Info className="w-5 h-5 flex-shrink-0 mt-0.5" />
                <div>
                  <p className="font-semibold text-sm">工作原理</p>
                  <p className="text-xs opacity-90 mt-1">
                    将数据流切分成小片段，通过不同节点传输，在本地重组。单一路径上的审查设备只能看到残缺的加密碎片，无法进行行为分析。
                  </p>
                </div>
              </div>
            </div>

            <div className="flex gap-4">
              <Button
                variant="default"
                onClick={onSaveConfig}
                disabled={saving || loading}
                className="flex-1"
              >
                保存配置
              </Button>
              <Button
                variant="outline"
                onClick={onLoadRecommended}
                disabled={saving || loading}
              >
                加载推荐配置
              </Button>
            </div>
          </div>
        )}

        {/* 节点池管理 */}
        {tabValue === 1 && (
          <div className="space-y-4">
            <div className="flex justify-between items-center">
              <h3 className="text-sm font-semibold">节点池列表</h3>
              <Button size="sm" onClick={onAddPool}>
                <Plus className="w-4 h-4 mr-1" />
                添加节点池
              </Button>
            </div>

            {config.node_pools.map((pool) => (
              <div key={pool.name} className="p-4 bg-card border border-border rounded-lg">
                <div className="flex justify-between mb-4">
                  <div>
                    <h4 className="font-semibold">{pool.name}</h4>
                    <div className="flex gap-2 mt-2">
                      <span className="px-2 py-1 bg-secondary text-secondary-foreground rounded-full text-xs">
                        {getPoolTypeLabel(pool.pool_type)}
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
                      <span className="px-2 py-1 bg-secondary text-secondary-foreground rounded-full text-xs">
                        {pool.nodes.length} 个节点
                      </span>
                    </div>
                  </div>
                  <div className="flex gap-2">
                    <button
                      onClick={() => onExportNodes(pool.name)}
                      className="p-2 hover:bg-secondary rounded"
                      title="导出节点"
                    >
                      <Download className="w-4 h-4" />
                    </button>
                    <button
                      onClick={() => onAddNode(pool.name)}
                      className="p-2 hover:bg-secondary rounded"
                      title="添加节点"
                    >
                      <Plus className="w-4 h-4" />
                    </button>
                    <button
                      onClick={() => onRemovePool(pool.name)}
                      className="p-2 hover:bg-red-500/10 text-red-500 rounded"
                      title="删除节点池"
                    >
                      <Trash2 className="w-4 h-4" />
                    </button>
                  </div>
                </div>

                {pool.nodes.length > 0 && (
                  <div>
                    <p className="text-xs text-muted-foreground mb-2">节点列表</p>
                    <div className="space-y-2">
                      {pool.nodes.map((node) => (
                        <div
                          key={node.name}
                          className="flex justify-between items-center p-2 bg-background rounded"
                        >
                          <div>
                            <p className="text-sm">{node.name}</p>
                            <p className="text-xs text-muted-foreground">
                              {node.server}:{node.port} | 权重: {node.weight}
                            </p>
                          </div>
                          <button
                            onClick={() => onRemoveNode(pool.name, node.name)}
                            className="p-1 hover:bg-red-500/10 text-red-500 rounded"
                          >
                            <Trash2 className="w-3 h-3" />
                          </button>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* 会话绑定规则 */}
        {tabValue === 2 && (
          <div className="space-y-4">
            <div className="p-4 bg-red-500 text-white rounded-lg">
              <p className="font-semibold text-sm">
                ⚠️ 以下服务必须使用单节点，否则会被封号
              </p>
            </div>

            <h3 className="text-sm font-semibold">预定义规则（推荐）</h3>
            {predefinedBindings.map((binding) => (
              <div key={binding.domain_pattern} className="p-4 bg-card border border-border rounded-lg">
                <div className="flex justify-between">
                  <div>
                    <p className="text-sm font-medium">{binding.domain_pattern}</p>
                    <p className="text-xs text-muted-foreground mt-1">
                      {binding.description}
                    </p>
                    <div className="flex gap-2 mt-2">
                      <span className="px-2 py-1 bg-secondary text-secondary-foreground rounded-full text-xs">
                        {getPoolTypeLabel(binding.pool_type)}
                      </span>
                      {binding.force_single_node && (
                        <span className="px-2 py-1 bg-red-500 text-white rounded-full text-xs">
                          强制单节点
                        </span>
                      )}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  )
}
