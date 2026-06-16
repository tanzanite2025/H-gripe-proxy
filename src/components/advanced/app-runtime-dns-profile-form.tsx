import { Save } from 'lucide-react'
import type { ChangeEvent, Dispatch, SetStateAction } from 'react'

import { Button } from '@/components/tailwind/Button'
import { TextField } from '@/components/tailwind/TextField'
import type { AppPolicyBinding, AppRegistryEntry } from '@/services/app-runtime'

interface DnsProfileDraft {
  profileId: string
  name: string
  testDomain: string
  tags: string
  configYaml: string
}

interface AppRuntimeDnsProfileFormProps {
  selectedApp: AppRegistryEntry | null
  selectedBinding: AppPolicyBinding | null
  draft: DnsProfileDraft
  pending: boolean
  setDraft: Dispatch<SetStateAction<DnsProfileDraft>>
  onSave: () => void
}

export function AppRuntimeDnsProfileForm({
  selectedApp,
  selectedBinding,
  draft,
  pending,
  setDraft,
  onSave,
}: AppRuntimeDnsProfileFormProps) {
  if (!selectedApp) {
    return null
  }

  return (
    <div className="space-y-3 rounded-lg border border-border p-3">
      <div>
        <div className="text-sm font-semibold">DNS profile 快速表单</div>
        <div className="mt-1 text-xs text-muted-foreground">
          编辑当前 app 绑定的 DNS profile；保存后可直接运行绑定 DNS controlled
          probe。
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
        <TextField
          fullWidth
          size="small"
          label="Test domain"
          value={draft.testDomain}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              testDomain: event.target.value,
            }))
          }}
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
        <div className="lg:col-span-2">
          <TextField
            fullWidth
            multiline
            rows={6}
            size="small"
            label="DNS YAML"
            value={draft.configYaml}
            onChange={(
              event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
            ) => {
              setDraft((current) => ({
                ...current,
                configYaml: event.target.value,
              }))
            }}
            helperText="用于 Rust DnsResolverPlan / controlled probe；不会切默认 DNS runtime。"
          />
        </div>
        <div className="flex items-end">
          <Button
            size="small"
            startIcon={<Save className="h-4 w-4" />}
            onClick={onSave}
            disabled={pending}
          >
            保存 DNS profile
          </Button>
        </div>
      </div>

      <div className="text-xs text-muted-foreground">
        当前绑定: {selectedBinding?.dnsProfileId || '未绑定 DNS profile'}
      </div>
    </div>
  )
}
