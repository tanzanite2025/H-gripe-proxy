import { useMemo } from 'react'

import type {
  CoordinatorStatus,
  EgressIdentityConfig,
  ResidentialProxyPool,
} from '@/services/coordinator'

import { EgressIdentityOverviewCard } from './overview-card'
import { EgressIdentityProfileRulesEditor } from './profile-rules-editor'
import { EgressIdentityRuntimeToolsCard } from './runtime-tools-card'
import type { EgressProfileOption } from './shared'
import { createEgressConfigEditor } from './use-egress-config-editor'
import { useEgressPreviewActions } from './use-egress-preview-actions'

interface Props {
  config: EgressIdentityConfig
  status: CoordinatorStatus
  onRefreshStatus: () => Promise<CoordinatorStatus | null>
  onChange: (config: EgressIdentityConfig) => void
  residentialPool?: ResidentialProxyPool
}

export function EgressIdentityPanel({
  config,
  status,
  onRefreshStatus,
  onChange,
  residentialPool,
}: Props) {
  const profileOptions = useMemo<EgressProfileOption[]>(
    () => [
      { value: '', label: '不设置默认画像' },
      ...config.profiles.map((profile) => ({
        value: profile.id,
        label: `${profile.name} (${profile.id})${
          profile.enabled ? '' : ' · 已禁用'
        }`,
      })),
    ],
    [config.profiles],
  )

  const profileNameMap = useMemo(
    () =>
      Object.fromEntries(
        config.profiles.map((profile) => [profile.id, profile.name]),
      ) as Record<string, string>,
    [config.profiles],
  )

  const activeAssignments = status.runtimeState.egressIdentityAssignments
  const domainPatternAssignments =
    status.runtimeState.stableEgressBackwrite.domainPatternAssignments
  const regularAssignments = activeAssignments.filter(
    (assignment) => !assignment.assignmentKey?.startsWith('domain-pattern:'),
  )

  const {
    ensureInitialized,
    addProfile,
    updateProfile,
    renameProfileId,
    removeProfile,
    addAppRule,
    updateAppRule,
    removeAppRule,
    addShortcutRule,
    updateShortcutRule,
    removeShortcutRule,
    handleToggleEnabled,
    handleClearRules,
    handleDefaultProfileChange,
  } = createEgressConfigEditor({
    config,
    onChange,
  })

  const {
    previewResult,
    previewLoading,
    assignLoading,
    assignmentsLoading,
    previewForm,
    refreshAssignments,
    handlePreview,
    handleAssign,
    handleClearAssignment,
    handlePreviewFormChange,
  } = useEgressPreviewActions({
    onRefreshStatus,
  })

  return (
    <div className="space-y-4">
      <EgressIdentityOverviewCard
        config={config}
        activeAssignmentCount={activeAssignments.length}
        domainPatternAssignmentCount={domainPatternAssignments.length}
        onToggleEnabled={handleToggleEnabled}
        onInitialize={ensureInitialized}
        onClearRules={handleClearRules}
      />
      <EgressIdentityProfileRulesEditor
        config={config}
        profileOptions={profileOptions}
        residentialPool={residentialPool}
        onDefaultProfileChange={handleDefaultProfileChange}
        onAddProfile={addProfile}
        onUpdateProfile={updateProfile}
        onRenameProfileId={renameProfileId}
        onRemoveProfile={removeProfile}
        onAddAppRule={addAppRule}
        onUpdateAppRule={updateAppRule}
        onRemoveAppRule={removeAppRule}
        onAddShortcutRule={addShortcutRule}
        onUpdateShortcutRule={updateShortcutRule}
        onRemoveShortcutRule={removeShortcutRule}
      />
      <EgressIdentityRuntimeToolsCard
        enabled={config.enabled}
        previewForm={previewForm}
        previewResult={previewResult}
        profileNameMap={profileNameMap}
        domainPatternAssignments={domainPatternAssignments}
        regularAssignments={regularAssignments}
        previewLoading={previewLoading}
        assignLoading={assignLoading}
        assignmentsLoading={assignmentsLoading}
        onPreviewFormChange={handlePreviewFormChange}
        onRefreshAssignments={refreshAssignments}
        onPreview={handlePreview}
        onAssign={handleAssign}
        onClearAssignment={handleClearAssignment}
      />
    </div>
  )
}
