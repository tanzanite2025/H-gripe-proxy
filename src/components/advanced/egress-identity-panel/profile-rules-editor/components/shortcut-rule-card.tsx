import { type ChangeEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Select } from '@/components/tailwind/Select'
import type { SelectPrimitiveValue } from '@/components/tailwind/Select'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import type { ShortcutEgressRule } from '@/services/coordinator'

interface ShortcutRuleCardProps {
  index: number
  rule: ShortcutEgressRule
  profileOptions: Array<{ value: string; label: string }>
  onUpdateShortcutRule: (index: number, nextRule: ShortcutEgressRule) => void
  onRemoveShortcutRule: (index: number) => void
}

export function ShortcutRuleCard({
  index,
  rule,
  profileOptions,
  onUpdateShortcutRule,
  onRemoveShortcutRule,
}: ShortcutRuleCardProps) {
  return (
    <div className="space-y-4 rounded-lg border border-gray-200 p-4 dark:border-gray-700">
      <div className="flex items-center justify-between gap-4">
        <div>
          <p className="font-medium">快捷方式规则 {index + 1}</p>
        </div>
        <div className="flex items-center gap-2">
          <Switch
            checked={rule.enabled}
            onCheckedChange={(checked) =>
              onUpdateShortcutRule(index, {
                ...rule,
                enabled: checked,
              })
            }
          />
          <Button
            size="small"
            variant="outlined"
            onClick={() => onRemoveShortcutRule(index)}
          >
            删除规则
          </Button>
        </div>
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <TextField
          label="快捷方式 ID"
          value={rule.shortcut_id}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            onUpdateShortcutRule(index, {
              ...rule,
              shortcut_id: event.target.value,
            })
          }
          fullWidth
        />
        <Select
          value={rule.profile_id}
          onChange={(value: SelectPrimitiveValue) =>
            onUpdateShortcutRule(index, {
              ...rule,
              profile_id: String(value),
            })
          }
          options={profileOptions}
          label="目标画像"
          fullWidth
        />
      </div>
    </div>
  )
}
