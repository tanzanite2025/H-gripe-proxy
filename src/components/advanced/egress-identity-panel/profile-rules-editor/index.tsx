import { AppRulesSection } from './components/app-rules-section'
import { ProfileSection } from './components/profile-section'
import { ShortcutRulesSection } from './components/shortcut-rules-section'
import type { EgressIdentityProfileRulesEditorProps } from './shared'

export function EgressIdentityProfileRulesEditor({
  config,
  profileOptions,
  residentialPool,
  onDefaultProfileChange,
  onAddProfile,
  onUpdateProfile,
  onRenameProfileId,
  onRemoveProfile,
  onAddAppRule,
  onUpdateAppRule,
  onRemoveAppRule,
  onAddShortcutRule,
  onUpdateShortcutRule,
  onRemoveShortcutRule,
}: EgressIdentityProfileRulesEditorProps) {
  return (
    <>
      <ProfileSection
        config={config}
        profileOptions={profileOptions}
        residentialPool={residentialPool}
        onDefaultProfileChange={onDefaultProfileChange}
        onAddProfile={onAddProfile}
        onUpdateProfile={onUpdateProfile}
        onRenameProfileId={onRenameProfileId}
        onRemoveProfile={onRemoveProfile}
      />
      <AppRulesSection
        config={config}
        onAddAppRule={onAddAppRule}
        onUpdateAppRule={onUpdateAppRule}
        onRemoveAppRule={onRemoveAppRule}
      />
      <ShortcutRulesSection
        config={config}
        onAddShortcutRule={onAddShortcutRule}
        onUpdateShortcutRule={onUpdateShortcutRule}
        onRemoveShortcutRule={onRemoveShortcutRule}
      />
    </>
  )
}
