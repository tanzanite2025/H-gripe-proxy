import { Save } from 'lucide-react'
import type { Dispatch, SetStateAction } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Select } from '@/components/tailwind/Select'
import type {
  AppPolicyBinding,
  AppRegistryEntry,
  AppRoutingIntent,
} from '@/services/app-runtime'

import {
  enabledOptions,
  routingIntentOptions,
} from './app-runtime-planning-utils'

interface SelectOption {
  value: string
  label: string
}

interface BindingDraft {
  nodePoolId: string
  dnsProfileId: string
  securityProfileId: string
  routingIntent: AppRoutingIntent
  enabled: string
}

interface AppRuntimePolicyBindingFormProps {
  selectedApp: AppRegistryEntry | null
  selectedAppId: string
  selectedBinding: AppPolicyBinding | null
  draft: BindingDraft
  pending: boolean
  nodePoolOptions: SelectOption[]
  dnsProfileOptions: SelectOption[]
  securityProfileOptions: SelectOption[]
  setDraft: Dispatch<SetStateAction<BindingDraft>>
  onSave: () => void
}

export function AppRuntimePolicyBindingForm({
  selectedApp,
  selectedAppId,
  selectedBinding,
  draft,
  pending,
  nodePoolOptions,
  dnsProfileOptions,
  securityProfileOptions,
  setDraft,
  onSave,
}: AppRuntimePolicyBindingFormProps) {
  if (!selectedApp) {
    return null
  }

  return (
    <div className="space-y-3 rounded-lg border border-border p-3">
      <div>
        <div className="text-sm font-semibold">Policy binding 快速表单</div>
        <div className="mt-1 text-xs text-muted-foreground">
          常用绑定字段可直接通过表单保存；底层仍写入 Rust
          AppRuntimeStateDocument。
        </div>
      </div>

      <div className="grid gap-3 lg:grid-cols-2">
        <Select
          fullWidth
          size="small"
          label="Node pool"
          value={draft.nodePoolId}
          options={nodePoolOptions}
          onChange={(value: string | number) => {
            setDraft((current) => ({
              ...current,
              nodePoolId: String(value),
            }))
          }}
        />
        <Select
          fullWidth
          size="small"
          label="DNS profile"
          value={draft.dnsProfileId}
          options={dnsProfileOptions}
          onChange={(value: string | number) => {
            setDraft((current) => ({
              ...current,
              dnsProfileId: String(value),
            }))
          }}
        />
        <Select
          fullWidth
          size="small"
          label="Security profile"
          value={draft.securityProfileId}
          options={securityProfileOptions}
          onChange={(value: string | number) => {
            setDraft((current) => ({
              ...current,
              securityProfileId: String(value),
            }))
          }}
        />
        <Select
          fullWidth
          size="small"
          label="Routing intent"
          value={draft.routingIntent}
          options={routingIntentOptions}
          onChange={(value: string | number) => {
            setDraft((current) => ({
              ...current,
              routingIntent: String(value) as AppRoutingIntent,
            }))
          }}
        />
        <Select
          fullWidth
          size="small"
          label="Binding status"
          value={draft.enabled}
          options={enabledOptions}
          onChange={(value: string | number) => {
            setDraft((current) => ({
              ...current,
              enabled: String(value),
            }))
          }}
        />
        <div className="flex items-end">
          <Button
            size="small"
            startIcon={<Save className="h-4 w-4" />}
            onClick={onSave}
            disabled={pending}
          >
            保存 binding
          </Button>
        </div>
      </div>

      <div className="text-xs text-muted-foreground">
        Binding ID: {selectedBinding?.bindingId || `binding-${selectedAppId}`}
      </div>
    </div>
  )
}
