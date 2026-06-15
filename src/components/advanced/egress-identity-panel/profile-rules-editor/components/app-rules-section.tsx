import { Card } from '@/components/tailwind/Card'

import type { EgressIdentityProfileRulesEditorProps } from '../shared'
import { buildProfileTargetOptions } from '../shared'

import { AppRuleCard } from './app-rule-card'
import { EmptyState } from './empty-state'
import { SectionHeader } from './section-header'

interface AppRulesSectionProps {
  config: EgressIdentityProfileRulesEditorProps['config']
  onAddAppRule: EgressIdentityProfileRulesEditorProps['onAddAppRule']
  onUpdateAppRule: EgressIdentityProfileRulesEditorProps['onUpdateAppRule']
  onRemoveAppRule: EgressIdentityProfileRulesEditorProps['onRemoveAppRule']
}

export function AppRulesSection({
  config,
  onAddAppRule,
  onUpdateAppRule,
  onRemoveAppRule,
}: AppRulesSectionProps) {
  const profileOptions = buildProfileTargetOptions(config.profiles)

  return (
    <Card variant="outlined">
      <div className="space-y-4 p-4">
        <SectionHeader
          title="应用规则"
          description="用进程名、可执行路径和域名模式把应用映射到目标画像。"
          actionLabel="添加应用规则"
          onAction={onAddAppRule}
        />

        {config.app_rules.length === 0 ? (
          <EmptyState message="暂无应用规则" />
        ) : (
          <div className="space-y-4">
            {config.app_rules.map((rule, index) => (
              <AppRuleCard
                key={`${rule.process_name || ''}-${rule.exe_path || ''}-${rule.profile_id}-${rule.priority}`}
                index={index}
                rule={rule}
                profileOptions={profileOptions}
                onUpdateAppRule={onUpdateAppRule}
                onRemoveAppRule={onRemoveAppRule}
              />
            ))}
          </div>
        )}
      </div>
    </Card>
  )
}
