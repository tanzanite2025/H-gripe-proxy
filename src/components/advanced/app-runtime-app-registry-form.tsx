import { Save } from 'lucide-react'
import type { ChangeEvent, Dispatch, SetStateAction } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Select } from '@/components/tailwind/Select'
import { TextField } from '@/components/tailwind/TextField'
import type {
  AppProcessMatcherKind,
  AppRegistryEntry,
} from '@/services/app-runtime'

import { processMatcherKindOptions } from './app-runtime-planning-utils'

interface AppRegistryDraft {
  name: string
  executablePath: string
  bundleId: string
  workingDirectory: string
  matcherKind: AppProcessMatcherKind
  matcherPattern: string
  tags: string
}

interface AppRuntimeAppRegistryFormProps {
  selectedApp: AppRegistryEntry | null
  draft: AppRegistryDraft
  pending: boolean
  setDraft: Dispatch<SetStateAction<AppRegistryDraft>>
  onSave: () => void
}

export function AppRuntimeAppRegistryForm({
  selectedApp,
  draft,
  pending,
  setDraft,
  onSave,
}: AppRuntimeAppRegistryFormProps) {
  if (!selectedApp) {
    return null
  }

  return (
    <div className="space-y-3 rounded-lg border border-border p-3">
      <div>
        <div className="text-sm font-semibold">App registry 快速表单</div>
        <div className="mt-1 text-xs text-muted-foreground">
          常用应用注册字段可直接通过表单保存；高级字段仍可用 JSON editor。
        </div>
      </div>

      <div className="grid gap-3 lg:grid-cols-2">
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
          label="Executable path"
          value={draft.executablePath}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              executablePath: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Bundle ID"
          value={draft.bundleId}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              bundleId: event.target.value,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Working directory"
          value={draft.workingDirectory}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              workingDirectory: event.target.value,
            }))
          }}
        />
        <Select
          fullWidth
          size="small"
          label="Matcher kind"
          value={draft.matcherKind}
          options={processMatcherKindOptions}
          onChange={(value: string | number) => {
            setDraft((current) => ({
              ...current,
              matcherKind: String(value) as AppProcessMatcherKind,
            }))
          }}
        />
        <TextField
          fullWidth
          size="small"
          label="Matcher pattern"
          value={draft.matcherPattern}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => {
            setDraft((current) => ({
              ...current,
              matcherPattern: event.target.value,
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
        <div className="flex items-end">
          <Button
            size="small"
            startIcon={<Save className="h-4 w-4" />}
            onClick={onSave}
            disabled={pending}
          >
            保存 app
          </Button>
        </div>
      </div>

      <div className="text-xs text-muted-foreground">
        App ID: {selectedApp.appId}
      </div>
    </div>
  )
}
