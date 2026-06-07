import { type ChangeEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Select } from '@/components/tailwind/Select'
import type { SelectPrimitiveValue } from '@/components/tailwind/Select'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import type { AppEgressRule } from '@/services/coordinator'

import { joinList, splitList } from '../../shared'

interface AppRuleCardProps {
  index: number
  rule: AppEgressRule
  profileOptions: Array<{ value: string; label: string }>
  onUpdateAppRule: (index: number, nextRule: AppEgressRule) => void
  onRemoveAppRule: (index: number) => void
}

export function AppRuleCard({
  index,
  rule,
  profileOptions,
  onUpdateAppRule,
  onRemoveAppRule,
}: AppRuleCardProps) {
  return (
    <div className="space-y-4 rounded-lg border border-gray-200 p-4 dark:border-gray-700">
      <div className="flex items-center justify-between gap-4">
        <div>
          <p className="font-medium">规则 {index + 1}</p>
          <p className="mt-1 text-sm text-gray-500">
            优先级数字越小越先匹配
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Switch
            checked={rule.enabled}
            onCheckedChange={(checked) =>
              onUpdateAppRule(index, { ...rule, enabled: checked })
            }
          />
          <Button
            size="small"
            variant="outlined"
            onClick={() => onRemoveAppRule(index)}
          >
            删除规则
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <TextField
          label="进程名"
          value={rule.process_name || ''}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onUpdateAppRule(index, {
              ...rule,
              process_name: event.target.value || null,
            })
          }
          fullWidth
        />
        <TextField
          label="可执行路径"
          value={rule.exe_path || ''}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onUpdateAppRule(index, {
              ...rule,
              exe_path: event.target.value || null,
            })
          }
          fullWidth
        />
        <Select
          value={rule.profile_id}
          onChange={(value: SelectPrimitiveValue) =>
            onUpdateAppRule(index, {
              ...rule,
              profile_id: String(value),
            })
          }
          options={profileOptions}
          label="目标画像"
          fullWidth
        />
        <TextField
          label="优先级"
          type="number"
          value={rule.priority}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onUpdateAppRule(index, {
              ...rule,
              priority: Number.parseInt(event.target.value, 10) || 0,
            })
          }
          fullWidth
        />
      </div>

      <TextField
        label="域名模式"
        value={joinList(rule.domains)}
        onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
          onUpdateAppRule(index, {
            ...rule,
            domains: splitList(event.target.value),
          })
        }
        helperText="用逗号或换行分隔，例如 *.openai.com"
        multiline
        rows={3}
        fullWidth
      />
    </div>
  )
}
