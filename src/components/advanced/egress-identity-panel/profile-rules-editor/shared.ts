import type {
  AppEgressRule,
  EgressIdentityConfig,
  EgressIdentityProfile,
  ResidentialProxyPool,
  ShortcutEgressRule,
} from '@/services/coordinator'

import type { EgressProfileOption } from '../shared'

export interface EgressIdentityProfileRulesEditorProps {
  config: EgressIdentityConfig
  profileOptions: EgressProfileOption[]
  residentialPool?: ResidentialProxyPool
  onDefaultProfileChange: (profileId: string) => void
  onAddProfile: () => void
  onUpdateProfile: (
    profileId: string,
    updater: (profile: EgressIdentityProfile) => EgressIdentityProfile,
  ) => void
  onRenameProfileId: (profileId: string, nextProfileId: string) => void
  onRemoveProfile: (profileId: string) => void
  onAddAppRule: () => void
  onUpdateAppRule: (index: number, nextRule: AppEgressRule) => void
  onRemoveAppRule: (index: number) => void
  onAddShortcutRule: () => void
  onUpdateShortcutRule: (index: number, nextRule: ShortcutEgressRule) => void
  onRemoveShortcutRule: (index: number) => void
}

export const getProfileTitle = (profile: EgressIdentityProfile) =>
  profile.name || profile.id || '未命名画像'

export const buildProfileTargetOptions = (
  profiles: EgressIdentityProfile[],
) =>
  profiles.map((profile) => ({
    value: profile.id,
    label: `${profile.name} (${profile.id})`,
  }))
