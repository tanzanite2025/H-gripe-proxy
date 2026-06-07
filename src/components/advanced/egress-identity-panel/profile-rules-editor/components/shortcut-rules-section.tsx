import { Card } from '@/components/tailwind/Card'

import type { EgressIdentityProfileRulesEditorProps } from '../shared'
import { buildProfileTargetOptions } from '../shared'
import { EmptyState } from './empty-state'
import { SectionHeader } from './section-header'
import { ShortcutRuleCard } from './shortcut-rule-card'

interface ShortcutRulesSectionProps {
  config: EgressIdentityProfileRulesEditorProps['config']
  onAddShortcutRule: EgressIdentityProfileRulesEditorProps['onAddShortcutRule']
  onUpdateShortcutRule: EgressIdentityProfileRulesEditorProps['onUpdateShortcutRule']
  onRemoveShortcutRule: EgressIdentityProfileRulesEditorProps['onRemoveShortcutRule']
}

export function ShortcutRulesSection({
  config,
  onAddShortcutRule,
  onUpdateShortcutRule,
  onRemoveShortcutRule,
}: ShortcutRulesSectionProps) {
  const profileOptions = buildProfileTargetOptions(config.profiles)

  return (
    <Card variant="outlined">
      <div className="space-y-4 p-4">
        <SectionHeader
          title="快捷方式规则"
          description="用软件内快捷方式 ID 直接绑定到目标画像。"
          actionLabel="添加快捷方式规则"
          onAction={onAddShortcutRule}
        />

        {config.shortcut_rules.length === 0 ? (
          <EmptyState message="暂无快捷方式规则" />
        ) : (
          <div className="space-y-4">
            {config.shortcut_rules.map((rule, index) => (
              <ShortcutRuleCard
                key={`${rule.shortcut_id}-${rule.profile_id}`}
                index={index}
                rule={rule}
                profileOptions={profileOptions}
                onUpdateShortcutRule={onUpdateShortcutRule}
                onRemoveShortcutRule={onRemoveShortcutRule}
              />
            ))}
          </div>
        )}
      </div>
    </Card>
  )
}
