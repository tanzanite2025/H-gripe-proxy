import { Plus, Trash2 } from 'lucide-react'
import type { ChangeEvent } from 'react'

import {
  Alert,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  IconButton,
  Switch,
  TextField,
} from '@/components/tailwind'

import { createEmptyRule, RULE_TYPES } from './constants'
import { getRulePayloadHelperText } from './helpers'

interface PolicyEditorDialogProps {
  open: boolean
  policy: ISecurityPolicy | null
  editingIndex: number
  onClose: () => void
  onChange: (policy: ISecurityPolicy) => void
  onSave: () => void
}

export function PolicyEditorDialog({
  open,
  policy,
  editingIndex,
  onClose,
  onChange,
  onSave,
}: PolicyEditorDialogProps) {
  const updateRule = (index: number, patch: Partial<IPolicyRule>) => {
    if (!policy) return

    const nextRules = policy.rules.map((rule, ruleIndex) =>
      ruleIndex === index ? { ...rule, ...patch } : rule,
    )

    onChange({ ...policy, rules: nextRules })
  }

  const handleAddRule = () => {
    if (!policy) return
    onChange({ ...policy, rules: [...policy.rules, createEmptyRule()] })
  }

  const handleDeleteRule = (index: number) => {
    if (!policy) return
    onChange({
      ...policy,
      rules: policy.rules.filter((_, ruleIndex) => ruleIndex !== index),
    })
  }

  return (
    <Dialog
      open={open}
      onClose={onClose}
      maxWidth="lg"
      fullWidth
      showCloseButton
    >
      <DialogTitle>
        <div className="text-lg font-semibold">
          {editingIndex === -1 ? '新建安全策略' : '编辑安全策略'}
        </div>
        <div className="mt-1 text-sm text-muted-foreground">
          策略定义保存在高级配置中，保存配置后再手动应用到 Mihomo 运行态。
        </div>
      </DialogTitle>

      <DialogContent>
        {policy ? (
          <div className="min-w-[min(720px,calc(100vw-4rem))] space-y-4">
            <TextField
              label="策略名称"
              value={policy.name}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                onChange({ ...policy, name: event.target.value })
              }
              disabled={editingIndex !== -1}
              helperText="名称用于运行态 apply / revoke，创建后不建议频繁改动。"
              fullWidth
            />

            <TextField
              label="描述"
              value={policy.description}
              onChange={(event: ChangeEvent<HTMLInputElement>) =>
                onChange({ ...policy, description: event.target.value })
              }
              fullWidth
            />

            <div className="rounded-lg border border-border px-4 py-3">
              <div className="flex items-start justify-between gap-3">
                <div>
                  <p className="text-sm font-medium">启用策略</p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    关闭后策略仍会保留在配置里，但不会参与“应用全部”操作。
                  </p>
                </div>
                <Switch
                  checked={policy.enabled}
                  onCheckedChange={(checked) =>
                    onChange({ ...policy, enabled: checked })
                  }
                />
              </div>
            </div>

            <div className="space-y-3">
              <div className="flex items-center justify-between gap-2">
                <div>
                  <div className="text-sm font-medium">规则列表</div>
                  <div className="mt-1 text-xs text-muted-foreground">
                    支持域名、IP、进程及 AND / OR / NOT 等逻辑组合。
                  </div>
                </div>
                <Button
                  variant="outlined"
                  size="small"
                  onClick={handleAddRule}
                  startIcon={<Plus className="h-4 w-4" />}
                >
                  添加规则
                </Button>
              </div>

              {policy.rules.length === 0 ? (
                <Alert severity="info" className="text-sm">
                  还没有规则。先添加至少一条规则，再保存策略。
                </Alert>
              ) : null}

              <div className="space-y-2">
                {policy.rules.map((rule, index) => (
                  <div
                    key={`${rule.ruleType}-${index}`}
                    className="rounded-lg border border-border bg-muted/20 p-3"
                  >
                    <div className="flex items-start gap-3">
                      <div className="flex-1 space-y-3">
                        <div>
                          <label className="mb-2 block text-xs font-semibold uppercase tracking-widest text-text-secondary">
                            规则类型
                          </label>
                          <select
                            className="h-12 w-full rounded-input border border-border bg-card px-4 text-sm font-semibold text-text-primary transition-all duration-300 ease-smooth focus:border-primary focus:outline-none focus:ring-2 focus:ring-primary"
                            value={rule.ruleType}
                            onChange={(event) =>
                              updateRule(index, {
                                ruleType: event.target.value,
                              })
                            }
                          >
                            {RULE_TYPES.map((ruleType) => (
                              <option key={ruleType} value={ruleType}>
                                {ruleType}
                              </option>
                            ))}
                          </select>
                        </div>

                        <TextField
                          label="Payload"
                          value={rule.payload}
                          onChange={(event: ChangeEvent<HTMLInputElement>) =>
                            updateRule(index, { payload: event.target.value })
                          }
                          helperText={getRulePayloadHelperText(rule.ruleType)}
                          fullWidth
                          size="small"
                        />

                        <TextField
                          label="目标代理 / 策略组"
                          value={rule.proxy}
                          onChange={(event: ChangeEvent<HTMLInputElement>) =>
                            updateRule(index, { proxy: event.target.value })
                          }
                          helperText="例如 DIRECT、REJECT 或已有策略组名称。"
                          fullWidth
                          size="small"
                        />
                      </div>

                      <IconButton
                        size="small"
                        color="error"
                        onClick={() => handleDeleteRule(index)}
                        aria-label={`删除第 ${index + 1} 条规则`}
                      >
                        <Trash2 className="h-4 w-4" />
                      </IconButton>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </div>
        ) : null}
      </DialogContent>

      <DialogActions>
        <Button variant="outlined" onClick={onClose}>
          取消
        </Button>
        <Button variant="contained" onClick={onSave}>
          保存
        </Button>
      </DialogActions>
    </Dialog>
  )
}
