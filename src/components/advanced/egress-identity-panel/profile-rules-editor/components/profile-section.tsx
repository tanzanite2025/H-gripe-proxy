import { Card } from '@/components/tailwind/Card'
import { Select } from '@/components/tailwind/Select'
import type { SelectPrimitiveValue } from '@/components/tailwind/Select'
import type { ResidentialProxyPool } from '@/services/coordinator'

import type { EgressProfileOption } from '../../shared'
import type { EgressIdentityProfileRulesEditorProps } from '../shared'

import { EmptyState } from './empty-state'
import { ProfileCard } from './profile-card'
import { SectionHeader } from './section-header'

interface ProfileSectionProps {
  config: EgressIdentityProfileRulesEditorProps['config']
  profileOptions: EgressProfileOption[]
  residentialPool?: ResidentialProxyPool
  onDefaultProfileChange: (profileId: string) => void
  onAddProfile: () => void
  onUpdateProfile: EgressIdentityProfileRulesEditorProps['onUpdateProfile']
  onRenameProfileId: EgressIdentityProfileRulesEditorProps['onRenameProfileId']
  onRemoveProfile: EgressIdentityProfileRulesEditorProps['onRemoveProfile']
}

export function ProfileSection({
  config,
  profileOptions,
  residentialPool,
  onDefaultProfileChange,
  onAddProfile,
  onUpdateProfile,
  onRenameProfileId,
  onRemoveProfile,
}: ProfileSectionProps) {
  const enabledResidentialProxies =
    residentialPool?.proxies.filter((proxy) => proxy.enabled) ?? []
  const hasEnabledResidentialProxies =
    Boolean(residentialPool?.enabled) && enabledResidentialProxies.length > 0

  return (
    <Card variant="outlined">
      <div className="space-y-4 p-4">
        <SectionHeader
          title="默认画像与画像编辑"
          description="当快捷方式规则和应用规则都不匹配时，回退到默认画像。"
          actionLabel="添加画像"
          onAction={onAddProfile}
        />

        <Select
          value={config.default_profile || ''}
          onChange={(value: SelectPrimitiveValue) =>
            onDefaultProfileChange(String(value || ''))
          }
          options={profileOptions}
          label="默认画像"
          fullWidth
        />

        {config.profiles.length === 0 ? (
          <EmptyState message="暂无出口画像" />
        ) : (
          <div className="space-y-4">
            {config.profiles.map((profile) => (
              <ProfileCard
                key={profile.id}
                profile={profile}
                isDefaultProfile={config.default_profile === profile.id}
                enabledResidentialProxies={enabledResidentialProxies}
                hasEnabledResidentialProxies={hasEnabledResidentialProxies}
                onUpdateProfile={onUpdateProfile}
                onRenameProfileId={onRenameProfileId}
                onRemoveProfile={onRemoveProfile}
              />
            ))}
          </div>
        )}
      </div>
    </Card>
  )
}
