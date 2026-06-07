import { useLockFn } from 'ahooks'
import { Plus, RefreshCw } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Alert, Button, Chip } from '@/components/tailwind'
import {
  securityPolicyApply,
  securityPolicyApplyAll,
  securityPolicyGetStates,
  securityPolicyRevoke,
  securityPolicyRevokeAll,
} from '@/services/cmds'
import { showNotice } from '@/services/notice-service'

import { createEmptyPolicy } from './constants'
import {
  clonePolicy,
  findAppliedPolicyState,
  validatePolicy,
} from './helpers'
import { PolicyCard } from './policy-card'
import { PolicyEditorDialog } from './policy-editor-dialog'

interface Props {
  policies: ISecurityPolicy[]
  hasUnsavedChanges?: boolean
  onChange: (policies: ISecurityPolicy[]) => void
}

function getErrorMessage(error: unknown) {
  if (error instanceof Error) {
    return error.message
  }

  return String(error)
}

export function SecurityPolicyPanel({
  policies,
  hasUnsavedChanges = false,
  onChange,
}: Props) {
  const [editingPolicy, setEditingPolicy] = useState<ISecurityPolicy | null>(null)
  const [editingIndex, setEditingIndex] = useState(-1)
  const [dialogOpen, setDialogOpen] = useState(false)
  const [appliedStates, setAppliedStates] = useState<IAppliedPolicyState[]>([])
  const [loading, setLoading] = useState(false)
  const [refreshingStates, setRefreshingStates] = useState(false)

  const refreshStates = useLockFn(async () => {
    setRefreshingStates(true)

    try {
      const states = await securityPolicyGetStates()
      setAppliedStates(states)
    } catch (error) {
      showNotice('error', '获取策略运行状态失败。', getErrorMessage(error))
    } finally {
      setRefreshingStates(false)
    }
  })

  useEffect(() => {
    void refreshStates()
  }, [refreshStates])

  const closeDialog = () => {
    setDialogOpen(false)
    setEditingPolicy(null)
    setEditingIndex(-1)
  }

  const ensureSavedBeforeRuntimeAction = () => {
    if (hasUnsavedChanges) {
      showNotice('info', '请先保存高级配置，再执行应用或撤销。')
      return false
    }

    return true
  }

  const handleCreate = () => {
    setEditingPolicy(createEmptyPolicy())
    setEditingIndex(-1)
    setDialogOpen(true)
  }

  const handleEdit = (index: number) => {
    setEditingPolicy(clonePolicy(policies[index]))
    setEditingIndex(index)
    setDialogOpen(true)
  }

  const handleSaveDialog = () => {
    if (!editingPolicy) return

    const validationError = validatePolicy(
      editingPolicy,
      policies,
      editingIndex,
    )
    if (validationError) {
      showNotice('error', validationError)
      return
    }

    const nextPolicies = [...policies]

    if (editingIndex === -1) {
      nextPolicies.push(editingPolicy)
    } else {
      nextPolicies[editingIndex] = editingPolicy
    }

    onChange(nextPolicies)
    closeDialog()
  }

  const handleDelete = (index: number) => {
    const targetPolicy = policies[index]
    onChange(policies.filter((_, policyIndex) => policyIndex !== index))
    showNotice(
      'info',
      `策略“${targetPolicy.name}”已从配置中移除，保存后生效。`,
    )
  }

  const handleToggleEnabled = (index: number, enabled: boolean) => {
    const nextPolicies = [...policies]
    nextPolicies[index] = { ...nextPolicies[index], enabled }
    onChange(nextPolicies)
  }

  const runRuntimeAction = async (
    action: () => Promise<void>,
    successMessage: string,
    errorPrefix: string,
  ) => {
    if (!ensureSavedBeforeRuntimeAction()) {
      return
    }

    setLoading(true)

    try {
      await action()
      showNotice('success', successMessage)
      await refreshStates()
    } catch (error) {
      showNotice('error', errorPrefix, getErrorMessage(error))
    } finally {
      setLoading(false)
    }
  }

  const handleApply = useLockFn(async (name: string) => {
    await runRuntimeAction(
      async () => {
        await securityPolicyApply(name)
      },
      `策略“${name}”已应用到 Mihomo。`,
      '应用策略失败。',
    )
  })

  const handleRevoke = useLockFn(async (name: string) => {
    await runRuntimeAction(
      async () => {
        await securityPolicyRevoke(name)
      },
      `策略“${name}”已从 Mihomo 撤销。`,
      '撤销策略失败。',
    )
  })

  const handleApplyAll = useLockFn(async () => {
    if (!ensureSavedBeforeRuntimeAction()) {
      return
    }

    setLoading(true)

    try {
      const applied = await securityPolicyApplyAll()
      showNotice('success', `已应用 ${applied.length} 个策略。`)
      await refreshStates()
    } catch (error) {
      showNotice('error', '批量应用策略失败。', getErrorMessage(error))
    } finally {
      setLoading(false)
    }
  })

  const handleRevokeAll = useLockFn(async () => {
    if (!ensureSavedBeforeRuntimeAction()) {
      return
    }

    setLoading(true)

    try {
      const revoked = await securityPolicyRevokeAll()
      showNotice('success', `已撤销 ${revoked.length} 个策略。`)
      await refreshStates()
    } catch (error) {
      showNotice('error', '批量撤销策略失败。', getErrorMessage(error))
    } finally {
      setLoading(false)
    }
  })

  const appliedCount = appliedStates.filter((state) => state.applied).length
  const enabledCount = policies.filter((policy) => policy.enabled).length

  return (
    <div className="space-y-4">
      <Alert severity="info" className="text-sm">
        安全策略通过 Mihomo 规则引擎控制域名、进程和入口访问。策略定义会保存在高级配置里，保存后还需要手动应用到运行态才会真正生效。
      </Alert>

      {hasUnsavedChanges ? (
        <Alert severity="warning" className="text-sm">
          你当前修改了策略定义，但还没有保存。保存配置之前，应用 / 撤销操作会被阻止，避免运行态和磁盘配置不一致。
        </Alert>
      ) : null}

      <div className="rounded-lg border border-border bg-card p-4">
        <div className="flex flex-wrap items-center justify-between gap-3">
          <div className="flex flex-wrap items-center gap-2">
            <Chip size="small" color="info" label={`共 ${policies.length} 个策略`} />
            <Chip
              size="small"
              color={enabledCount > 0 ? 'success' : 'default'}
              label={`启用 ${enabledCount} 个`}
            />
            <Chip
              size="small"
              color={appliedCount > 0 ? 'success' : 'default'}
              label={`运行态已应用 ${appliedCount} 个`}
            />
            {refreshingStates ? (
              <Chip size="small" color="info" label="正在同步运行态" />
            ) : null}
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outlined"
              size="small"
              onClick={handleCreate}
              startIcon={<Plus className="h-4 w-4" />}
            >
              新建策略
            </Button>
            <Button
              variant="outlined"
              size="small"
              onClick={handleApplyAll}
              disabled={loading || hasUnsavedChanges}
            >
              应用全部
            </Button>
            <Button
              variant="outlined"
              size="small"
              onClick={handleRevokeAll}
              disabled={loading || hasUnsavedChanges}
            >
              撤销全部
            </Button>
            <Button
              variant="outlined"
              size="small"
              onClick={() => void refreshStates()}
              disabled={refreshingStates}
              startIcon={<RefreshCw className="h-4 w-4" />}
            >
              刷新状态
            </Button>
          </div>
        </div>
      </div>

      {policies.length === 0 ? (
        <div className="rounded-lg border border-border bg-card p-6 text-center text-sm text-muted-foreground">
          还没有安全策略。先新建一条策略，再按需要保存并应用到运行态。
        </div>
      ) : (
        <div className="space-y-3">
          {policies.map((policy, index) => (
            <PolicyCard
              key={policy.name}
              policy={policy}
              state={findAppliedPolicyState(appliedStates, policy.name)}
              busy={loading}
              hasUnsavedChanges={hasUnsavedChanges}
              onToggleEnabled={(enabled) => handleToggleEnabled(index, enabled)}
              onApply={() => void handleApply(policy.name)}
              onRevoke={() => void handleRevoke(policy.name)}
              onEdit={() => handleEdit(index)}
              onDelete={() => handleDelete(index)}
            />
          ))}
        </div>
      )}

      <PolicyEditorDialog
        open={dialogOpen}
        policy={editingPolicy}
        editingIndex={editingIndex}
        onClose={closeDialog}
        onChange={setEditingPolicy}
        onSave={handleSaveDialog}
      />
    </div>
  )
}
