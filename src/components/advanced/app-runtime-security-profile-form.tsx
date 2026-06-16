import { Save } from 'lucide-react'
import type { ChangeEvent, Dispatch, SetStateAction } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Select } from '@/components/tailwind/Select'
import { TextField } from '@/components/tailwind/TextField'
import type { AppPolicyBinding, AppRegistryEntry } from '@/services/app-runtime'

import { enabledOptions } from './app-runtime-planning-utils'

interface SecurityProfileDraft {
  profileId: string
  name: string
  requireNodePool: string
  requireDnsProfile: string
  minRuntimeSupportedNameservers: string
  allowedRoutingIntents: string
  tags: string
}

interface AppRuntimeSecurityProfileFormProps {
  selectedApp: AppRegistryEntry | null
  selectedBinding: AppPolicyBinding | null
  draft: SecurityProfileDraft
  pending: boolean
  setDraft: Dispatch<SetStateAction<SecurityProfileDraft>>
  onSave: () => void
}

export function AppRuntimeSecurityProfileForm({
  selectedApp,
  selectedBinding,
  draft,
  pending,
  setDraft,
  onSave,
}: AppRuntimeSecurityProfileFormProps) {
  if (!selectedApp) {
    return null
  }

  return (
    <div className="space-y-3 rounded-lg border border-border p-3">
      <div>
        <div className="text-sm font-semibold">Security profile 快速表单</div>
        <div className="mt-1 text-xs text-muted-foreground">
          编辑当前 app 绑定的 security profile 约束；仍只影响 diagnostics /
          planning。
        </div>
      </div>

      <div className="grid gap-3 lg:grid-cols-2">
        <TextField
          fullWidth
          size="small"
          label="Profile ID"
          value={draft.profileId}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              profileId: event.target.value,
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
        <Select
          fullWidth
          size="small"
          label="Require node pool"
          value={draft.requireNodePool}
          options={enabledOptions}
          onChange={(value: string | number) => {
            setDraft((current) => ({
              ...current,
              requireNodePool: String(value),
            }))
          }}
        />
        <Select
          fullWidth
          size="small"
          label="Require DNS profile"
          value={draft.requireDnsProfile}
          options={enabledOptions}
          onChange={(value: string | number) => {
            setDraft((current) => ({
              ...current,
              requireDnsProfile: String(value),
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Min runtime-supported nameservers"
          value={draft.minRuntimeSupportedNameservers}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              minRuntimeSupportedNameservers: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Allowed routing intents"
          value={draft.allowedRoutingIntents}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              allowedRoutingIntents: event.target.value,
            }))
          }}
          helperText="逗号分隔，例如 proxy, fallback。"
        />
        <TextField
          fullWidth
          size="small"
          label="Tags"
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
            保存 security profile
          </Button>
        </div>
      </div>

      <div className="text-xs text-muted-foreground">
        当前绑定:{' '}
        {selectedBinding?.securityProfileId || '未绑定 security profile'}
      </div>
    </div>
  )
}
