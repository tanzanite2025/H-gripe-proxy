/**
 * 安全策略规则管理面板
 *
 * 管理安全策略的 CRUD、启用/禁用、应用/撤销。
 * 策略定义持久化在 advanced.yaml，运行时通过 Mihomo 规则 API 应用。
 */

import { useLockFn } from 'ahooks'
import { Plus, Shield, Trash2, Play, Square, RefreshCw } from 'lucide-react'
import { useEffect, useState } from 'react'
import type { ChangeEvent } from 'react'

import { Switch, TextField, Button, Dialog, DialogTitle, DialogContent, DialogActions, IconButton } from '@/components/tailwind'
import {
  securityPolicyGetStates,
  securityPolicyApply,
  securityPolicyRevoke,
  securityPolicyApplyAll,
  securityPolicyRevokeAll,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

interface Props {
  policies: ISecurityPolicy[]
  hasUnsavedChanges?: boolean
  onChange: (policies: ISecurityPolicy[]) => void
}

/** 空策略模板 */
function emptyPolicy(): ISecurityPolicy {
  return { name: '', enabled: true, description: '', rules: [] }
}

/** 空规则模板 */
function emptyRule(): IPolicyRule {
  return { ruleType: 'DOMAIN', payload: '', proxy: 'REJECT' }
}

/** 可选规则类型 */
const RULE_TYPES = [
  'DOMAIN',
  'DOMAIN-SUFFIX',
  'DOMAIN-KEYWORD',
  'IP-CIDR',
  'SRC-IP-CIDR',
  'GEOIP',
  'PROCESS-NAME',
  'PROCESS-PATH',
  'IN-TYPE',
  'IN-USER',
  'IN-NAME',
  'NETWORK',
  'UID',
  'AND',
  'OR',
  'NOT',
  'SUB-RULE',
]

export function SecurityPolicyPanel({ policies, hasUnsavedChanges = false, onChange }: Props) {
  const [editingPolicy, setEditingPolicy] = useState<ISecurityPolicy | null>(null)
  const [editingIndex, setEditingIndex] = useState<number>(-1) // -1 = 新建
  const [dialogOpen, setDialogOpen] = useState(false)
  const [appliedStates, setAppliedStates] = useState<IAppliedPolicyState[]>([])
  const [loading, setLoading] = useState(false)

  // 刷新运行态状态
  const refreshStates = useLockFn(async () => {
    try {
      const states = await securityPolicyGetStates()
      setAppliedStates(states)
    } catch (e: any) {
      showNotice('error', `获取策略状态失败: ${e.message || e}`)
    }
  })

  useEffect(() => {
    void refreshStates()
  }, [refreshStates])

  // 打开新建对话框
  const handleCreate = () => {
    setEditingPolicy(emptyPolicy())
    setEditingIndex(-1)
    setDialogOpen(true)
  }

  // 打开编辑对话框
  const handleEdit = (index: number) => {
    setEditingPolicy({ ...policies[index], rules: [...policies[index].rules] })
    setEditingIndex(index)
    setDialogOpen(true)
  }

  // 保存对话框中的策略
  const handleSaveDialog = () => {
    if (!editingPolicy) return
    if (!editingPolicy.name.trim()) {
      showNotice('error', '策略名称不能为空')
      return
    }
    if (editingPolicy.rules.length === 0) {
      showNotice('error', '至少需要一条规则')
      return
    }
    for (const rule of editingPolicy.rules) {
      if (!rule.payload.trim()) {
        showNotice('error', '规则载荷不能为空')
        return
      }
      if (!rule.proxy.trim()) {
        showNotice('error', '规则目标代理不能为空')
        return
      }
    }

    const newPolicies = [...policies]
    if (editingIndex === -1) {
      // 检查重名
      if (newPolicies.some((p) => p.name === editingPolicy.name)) {
        showNotice('error', `策略 "${editingPolicy.name}" 已存在`)
        return
      }
      newPolicies.push(editingPolicy)
    } else {
      newPolicies[editingIndex] = editingPolicy
    }
    onChange(newPolicies)
    setDialogOpen(false)
    setEditingPolicy(null)
  }

  // 删除策略
  const handleDelete = (index: number) => {
    const name = policies[index].name
    const newPolicies = policies.filter((_, i) => i !== index)
    onChange(newPolicies)
    showNotice('info', `策略 "${name}" 已从配置中移除（保存后生效）`)
  }

  // 切换启用/禁用
  const handleToggleEnabled = (index: number, enabled: boolean) => {
    const newPolicies = [...policies]
    newPolicies[index] = { ...newPolicies[index], enabled }
    onChange(newPolicies)
  }

  // 应用单个策略到 Mihomo
  const handleApply = useLockFn(async (name: string) => {
    if (hasUnsavedChanges) {
      showNotice('info', 'Save the configuration first to apply or revoke policies.')
      return
    }
    setLoading(true)
    try {
      await securityPolicyApply(name)
      showNotice('success', `策略 "${name}" 已应用到 Mihomo`)
      await refreshStates()
    } catch (e: any) {
      showNotice('error', `应用策略失败: ${e.message || e}`)
    } finally {
      setLoading(false)
    }
  })

  // 撤销单个策略
  const handleRevoke = useLockFn(async (name: string) => {
    if (hasUnsavedChanges) {
      showNotice('info', 'Save the configuration first to apply or revoke policies.')
      return
    }
    setLoading(true)
    try {
      await securityPolicyRevoke(name)
      showNotice('success', `策略 "${name}" 已从 Mihomo 撤销`)
      await refreshStates()
    } catch (e: any) {
      showNotice('error', `撤销策略失败: ${e.message || e}`)
    } finally {
      setLoading(false)
    }
  })

  // 应用所有已启用策略
  const handleApplyAll = useLockFn(async () => {
    if (hasUnsavedChanges) {
      showNotice('info', 'Save the configuration first to apply or revoke policies.')
      return
    }
    setLoading(true)
    try {
      const applied = await securityPolicyApplyAll()
      showNotice('success', `已应用 ${applied.length} 个策略`)
      await refreshStates()
    } catch (e: any) {
      showNotice('error', `批量应用失败: ${e.message || e}`)
    } finally {
      setLoading(false)
    }
  })

  // 撤销所有策略
  const handleRevokeAll = useLockFn(async () => {
    if (hasUnsavedChanges) {
      showNotice('info', 'Save the configuration first to apply or revoke policies.')
      return
    }
    setLoading(true)
    try {
      const revoked = await securityPolicyRevokeAll()
      showNotice('success', `已撤销 ${revoked.length} 个策略`)
      await refreshStates()
    } catch (e: any) {
      showNotice('error', `批量撤销失败: ${e.message || e}`)
    } finally {
      setLoading(false)
    }
  })

  // 获取策略的运行态
  const getState = (name: string) => appliedStates.find((s) => s.name === name)

  return (
    <div>
      {/* 说明 */}
      <div className="p-4 bg-blue-500 text-white rounded-lg mb-4">
        <div className="flex items-start gap-2">
          <Shield className="w-5 h-5 shrink-0 mt-0.5" />
          <p className="text-sm">
            安全策略通过 Mihomo 规则引擎实现进程/入站/域名的访问控制。
            支持 AND/OR/NOT 逻辑组合，规则标记为 source=security:&lt;策略名&gt;。
            策略定义在保存配置后持久化，需手动应用才能生效。
          </p>
        </div>
      </div>

      {/* 操作栏 */}
      <div className="flex items-center gap-2 mb-4">
        <Button variant="outlined" size="small" onClick={handleCreate}>
          <Plus className="w-4 h-4 mr-1" /> 新建策略
        </Button>
        <Button variant="outlined" size="small" onClick={handleApplyAll} disabled={loading || hasUnsavedChanges}>
          <Play className="w-4 h-4 mr-1" /> 应用全部
        </Button>
        <Button variant="outlined" size="small" onClick={handleRevokeAll} disabled={loading || hasUnsavedChanges}>
          <Square className="w-4 h-4 mr-1" /> 撤销全部
        </Button>
        <Button variant="outlined" size="small" onClick={refreshStates}>
          <RefreshCw className="w-4 h-4 mr-1" /> 刷新状态
        </Button>
      </div>

      {/* 策略列表 */}
      {policies.length === 0 ? (
        <div className="p-4 bg-card border border-border rounded-lg text-center text-muted-foreground">
          暂无安全策略，点击"新建策略"创建
        </div>
      ) : (
        <div className="space-y-3">
          {policies.map((policy, index) => {
            const state = getState(policy.name)
            const isApplied = state?.applied ?? false

            return (
              <div
                key={policy.name}
                className={`p-4 bg-card border rounded-lg ${
                  isApplied ? 'border-green-500' : 'border-border'
                }`}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold">{policy.name}</span>
                    {isApplied && (
                      <span className="px-2 py-0.5 text-xs bg-green-500 text-white rounded-full">
                        已应用
                      </span>
                    )}
                    {!policy.enabled && (
                      <span className="px-2 py-0.5 text-xs bg-gray-500 text-white rounded-full">
                        已禁用
                      </span>
                    )}
                  </div>
                  <div className="flex items-center gap-1">
                    <Switch
                      checked={policy.enabled}
                      onCheckedChange={(checked) => handleToggleEnabled(index, checked)}
                    />
                    {isApplied ? (
                      <IconButton size="small" onClick={() => handleRevoke(policy.name)} disabled={loading || hasUnsavedChanges}>
                        <Square className="w-4 h-4" />
                      </IconButton>
                    ) : (
                      <IconButton size="small" onClick={() => handleApply(policy.name)} disabled={loading || hasUnsavedChanges || !policy.enabled}>
                        <Play className="w-4 h-4" />
                      </IconButton>
                    )}
                    <IconButton size="small" onClick={() => handleEdit(index)}>
                      <Shield className="w-4 h-4" />
                    </IconButton>
                    <IconButton size="small" onClick={() => handleDelete(index)}>
                      <Trash2 className="w-4 h-4 text-red-500" />
                    </IconButton>
                  </div>
                </div>

                {policy.description && (
                  <p className="text-sm text-muted-foreground mb-2">{policy.description}</p>
                )}

                {/* 规则预览 */}
                <div className="space-y-1">
                  {policy.rules.map((rule, ri) => (
                    <div key={ri} className="text-xs font-mono bg-muted/50 px-2 py-1 rounded">
                      <span className="text-blue-500">{rule.ruleType}</span>
                      {', '}
                      <span>{rule.payload}</span>
                      {', '}
                      <span className="text-green-600">{rule.proxy}</span>
                    </div>
                  ))}
                </div>

                {/* 运行态信息 */}
                {state && state.applied && (
                  <div className="mt-2 text-xs text-muted-foreground">
                    规则索引: [{state.ruleIndices.join(', ')}]
                  </div>
                )}
              </div>
            )
          })}
        </div>
      )}

      {/* 编辑/新建对话框 */}
      <Dialog open={dialogOpen} onClose={() => setDialogOpen(false)}>
        <DialogTitle>{editingIndex === -1 ? '新建安全策略' : '编辑安全策略'}</DialogTitle>
        <DialogContent>
          {editingPolicy && (
            <div className="space-y-4 min-w-[400px]">
              <TextField
                label="策略名称"
                value={editingPolicy.name}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  setEditingPolicy({ ...editingPolicy, name: e.target.value })
                }
                disabled={editingIndex !== -1}
                fullWidth
              />

              <TextField
                label="描述"
                value={editingPolicy.description}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  setEditingPolicy({ ...editingPolicy, description: e.target.value })
                }
                fullWidth
              />

              <div className="flex items-center justify-between">
                <label className="text-sm font-medium">启用</label>
                <Switch
                  checked={editingPolicy.enabled}
                  onCheckedChange={(checked) =>
                    setEditingPolicy({ ...editingPolicy, enabled: checked })
                  }
                />
              </div>

              {/* 规则列表 */}
              <div>
                <div className="flex items-center justify-between mb-2">
                  <label className="text-sm font-medium">规则列表</label>
                  <Button
                    variant="outlined"
                    size="small"
                    onClick={() =>
                      setEditingPolicy({
                        ...editingPolicy,
                        rules: [...editingPolicy.rules, emptyRule()],
                      })
                    }
                  >
                    <Plus className="w-3 h-3 mr-1" /> 添加规则
                  </Button>
                </div>

                <div className="space-y-2">
                  {editingPolicy.rules.map((rule, ri) => (
                    <div key={ri} className="flex items-start gap-2 p-2 bg-muted/30 rounded">
                      <div className="flex-1 space-y-2">
                        <select
                          className="w-full px-2 py-1 text-sm border border-border rounded bg-card"
                          value={rule.ruleType}
                          onChange={(e) => {
                            const newRules = [...editingPolicy.rules]
                            newRules[ri] = { ...newRules[ri], ruleType: e.target.value }
                            setEditingPolicy({ ...editingPolicy, rules: newRules })
                          }}
                        >
                          {RULE_TYPES.map((t) => (
                            <option key={t} value={t}>
                              {t}
                            </option>
                          ))}
                        </select>

                        <TextField
                          label="载荷"
                          value={rule.payload}
                          onChange={(e: ChangeEvent<HTMLInputElement>) => {
                            const newRules = [...editingPolicy.rules]
                            newRules[ri] = { ...newRules[ri], payload: e.target.value }
                            setEditingPolicy({ ...editingPolicy, rules: newRules })
                          }}
                          fullWidth
                          size="small"
                          helperText={
                            rule.ruleType === 'AND' || rule.ruleType === 'OR' || rule.ruleType === 'NOT'
                              ? '逻辑规则载荷格式: ((TYPE,payload),(TYPE,payload))'
                              : undefined
                          }
                        />

                        <TextField
                          label="目标代理/策略组"
                          value={rule.proxy}
                          onChange={(e: ChangeEvent<HTMLInputElement>) => {
                            const newRules = [...editingPolicy.rules]
                            newRules[ri] = { ...newRules[ri], proxy: e.target.value }
                            setEditingPolicy({ ...editingPolicy, rules: newRules })
                          }}
                          fullWidth
                          size="small"
                          helperText="如 DIRECT, REJECT, 或策略组名"
                        />
                      </div>

                      <IconButton size="small" onClick={() => {
                        const newRules = editingPolicy.rules.filter((_, i) => i !== ri)
                        setEditingPolicy({ ...editingPolicy, rules: newRules })
                      }}>
                        <Trash2 className="w-4 h-4 text-red-500" />
                      </IconButton>
                    </div>
                  ))}
                </div>
              </div>
            </div>
          )}
        </DialogContent>
        <DialogActions>
          <Button variant="outlined" onClick={() => setDialogOpen(false)}>
            取消
          </Button>
          <Button variant="primary" onClick={handleSaveDialog}>
            保存
          </Button>
        </DialogActions>
      </Dialog>
    </div>
  )
}
