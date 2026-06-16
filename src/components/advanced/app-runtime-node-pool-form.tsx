import { Save } from 'lucide-react'
import type { ChangeEvent, Dispatch, SetStateAction } from 'react'

import { Button } from '@/components/tailwind/Button'
import { TextField } from '@/components/tailwind/TextField'
import type { AppPolicyBinding, AppRegistryEntry } from '@/services/app-runtime'

interface NodePoolDraft {
  poolId: string
  name: string
  region: string
  protocols: string
  purpose: string
  costTier: string
  candidateNodeName: string
  candidateProxyGroup: string
  candidateTags: string
  tags: string
}

interface AppRuntimeNodePoolFormProps {
  selectedApp: AppRegistryEntry | null
  selectedBinding: AppPolicyBinding | null
  draft: NodePoolDraft
  pending: boolean
  setDraft: Dispatch<SetStateAction<NodePoolDraft>>
  onSave: () => void
}

export function AppRuntimeNodePoolForm({
  selectedApp,
  selectedBinding,
  draft,
  pending,
  setDraft,
  onSave,
}: AppRuntimeNodePoolFormProps) {
  if (!selectedApp) {
    return null
  }

  return (
    <div className="space-y-3 rounded-lg border border-border p-3">
      <div>
        <div className="text-sm font-semibold">Node pool 快速表单</div>
        <div className="mt-1 text-xs text-muted-foreground">
          编辑当前 app 绑定的节点池常用字段；保存后可在 policy binding
          表单中选择该 pool。
        </div>
      </div>

      <div className="grid gap-3 lg:grid-cols-2">
        <TextField
          fullWidth
          size="small"
          label="Pool ID"
          value={draft.poolId}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              poolId: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Name"
          value={draft.name}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              name: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Region"
          value={draft.region}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              region: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Protocols"
          value={draft.protocols}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              protocols: event.target.value,
            }))
          }}
          helperText="逗号分隔。"
        />
        <TextField
          fullWidth
          size="small"
          label="Purpose"
          value={draft.purpose}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              purpose: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Cost tier"
          value={draft.costTier}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              costTier: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Candidate node"
          value={draft.candidateNodeName}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              candidateNodeName: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Candidate proxy group"
          value={draft.candidateProxyGroup}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              candidateProxyGroup: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Candidate tags"
          value={draft.candidateTags}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              candidateTags: event.target.value,
            }))
          }}
          helperText="逗号分隔。"
        />
        <TextField
          fullWidth
          size="small"
          label="Pool tags"
          value={draft.tags}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              tags: event.target.value,
            }))
          }}
          helperText="逗号分隔。"
        />
        <div className="flex items-end">
          <Button
            size="small"
            startIcon={<Save className="h-4 w-4" />}
            onClick={onSave}
            disabled={pending}
          >
            保存 node pool
          </Button>
        </div>
      </div>

      <div className="text-xs text-muted-foreground">
        当前绑定: {selectedBinding?.nodePoolId || '未绑定 node pool'}
      </div>
    </div>
  )
}
