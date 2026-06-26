import { Download, Save, Sparkles, Trash2, Upload } from 'lucide-react'
import type { ChangeEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import { Select } from '@/components/tailwind/Select'
import { TextField } from '@/components/tailwind/TextField'
import type { AppRuntimeStateDocument } from '@/services/app-runtime'

import {
  newResourceValue,
  resourceKindOptions,
  stateCountLabel,
  type RuntimeResourceKind,
} from './app-runtime-planning-utils'

interface ResourceOption {
  value: string
  label: string
}

interface AppRuntimeResourceManagerPanelProps {
  state: AppRuntimeStateDocument
  resourceKind: RuntimeResourceKind
  selectedResourceId: string
  resourceOptions: ResourceOption[]
  resourceJson: string
  bulkJson: string
  pending: boolean
  onResourceKindChange: (kind: RuntimeResourceKind) => void
  onSelectedResourceIdChange: (resourceId: string) => void
  onResourceJsonChange: (value: string) => void
  onBulkJsonChange: (value: string) => void
  onSaveResource: () => void
  onDeleteResource: () => void
  onExportConfig: () => void
  onLoadDemoSeed: () => void
  onImportConfig: () => void
}

export function AppRuntimeResourceManagerPanel({
  state,
  resourceKind,
  selectedResourceId,
  resourceOptions,
  resourceJson,
  bulkJson,
  pending,
  onResourceKindChange,
  onSelectedResourceIdChange,
  onResourceJsonChange,
  onBulkJsonChange,
  onSaveResource,
  onDeleteResource,
  onExportConfig,
  onLoadDemoSeed,
  onImportConfig,
}: AppRuntimeResourceManagerPanelProps) {
  return (
    <>
      <div className="flex flex-wrap gap-2">
        <Chip size="small" label={stateCountLabel('Apps', state.apps.length)} />
        <Chip
          size="small"
          label={stateCountLabel('Node pools', state.nodePools.length)}
        />
        <Chip
          size="small"
          label={stateCountLabel('DNS profiles', state.dnsProfiles.length)}
        />
        <Chip
          size="small"
          label={stateCountLabel(
            'Security profiles',
            state.securityProfiles.length,
          )}
        />
        <Chip
          size="small"
          label={stateCountLabel('Bindings', state.policyBindings.length)}
        />
      </div>

      <div className="space-y-3 rounded-lg border border-border p-3">
        <div>
          <div className="text-sm font-semibold">Rust state 管理</div>
          <div className="mt-1 text-xs text-muted-foreground">
            基于现有 app-runtime upsert/delete commands 管理 Rust
            state；保存后仍只生成 planning / projection，不直接修改内核
            runtime。
          </div>
        </div>

        <div className="grid gap-3 lg:grid-cols-[220px_minmax(0,1fr)]">
          <Select
            fullWidth
            size="small"
            label="资源类型"
            value={resourceKind}
            options={resourceKindOptions}
            onChange={(value: string | number) => {
              onResourceKindChange(String(value) as RuntimeResourceKind)
              onSelectedResourceIdChange(newResourceValue)
            }}
          />
          <Select
            fullWidth
            size="small"
            label="资源"
            value={selectedResourceId}
            options={resourceOptions}
            onChange={(value: string | number) => {
              onSelectedResourceIdChange(String(value))
            }}
          />
        </div>

        <TextField
          fullWidth
          multiline
          rows={10}
          size="small"
          label="资源 JSON"
          value={resourceJson}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => onResourceJsonChange(event.target.value)}
          helperText="字段与 AppRuntimeStateDocument 中对应资源类型保持一致。"
        />

        <div className="flex flex-wrap gap-2">
          <Button
            size="small"
            startIcon={<Save className="h-4 w-4" />}
            onClick={onSaveResource}
            disabled={pending}
          >
            保存资源
          </Button>
          <Button
            size="small"
            variant="outlined"
            color="error"
            startIcon={<Trash2 className="h-4 w-4" />}
            onClick={onDeleteResource}
            disabled={pending || selectedResourceId === newResourceValue}
          >
            删除资源
          </Button>
          <Button
            size="small"
            variant="outlined"
            startIcon={<Download className="h-4 w-4" />}
            onClick={onExportConfig}
            disabled={pending}
          >
            导出配置 JSON
          </Button>
          <Button
            size="small"
            variant="outlined"
            startIcon={<Sparkles className="h-4 w-4" />}
            onClick={onLoadDemoSeed}
            disabled={pending}
          >
            加载 Demo seed
          </Button>
        </div>

        <TextField
          fullWidth
          multiline
          rows={6}
          size="small"
          label="批量导入 JSON"
          value={bulkJson}
          onChange={(
            event: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>,
          ) => onBulkJsonChange(event.target.value)}
          helperText="支持 apps / nodePools / dnsProfiles / securityProfiles / policyBindings，导入为合并 upsert。"
        />

        <Button
          size="small"
          variant="outlined"
          startIcon={<Upload className="h-4 w-4" />}
          onClick={onImportConfig}
          disabled={pending || !bulkJson.trim()}
        >
          导入/合并配置
        </Button>
      </div>
    </>
  )
}
