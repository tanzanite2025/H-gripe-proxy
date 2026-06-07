import { useMemo, useState } from 'react'

import type {
  AppEgressRule,
  CoordinatorStatus,
  EgressIdentityConfig,
  EgressIdentityProfile,
  ResidentialProxyPool,
  ShortcutEgressRule,
} from '@/services/coordinator'
import { getRecommendedAdvancedConfig } from '@/services/coordinator'
import {
  egressIdentityAssignMatch,
  egressIdentityClearAssignment,
  egressIdentityPreviewMatch,
  type EgressPreviewRequest,
  type ResolvedEgressIdentity,
} from '@/services/egress-identity'
import { showNotice } from '@/services/notice-service'

import { EgressIdentityOverviewCard } from './egress-identity-panel/overview-card'
import { EgressIdentityProfileRulesEditor } from './egress-identity-panel/profile-rules-editor'
import { EgressIdentityRuntimeToolsCard } from './egress-identity-panel/runtime-tools-card'
import {
  emptyEgressIdentityPreviewForm,
  splitList,
  type EgressIdentityPreviewFormState,
  type EgressProfileOption,
} from './egress-identity-panel/shared'

interface Props {
  config: EgressIdentityConfig
  status: CoordinatorStatus
  onRefreshStatus: () => Promise<CoordinatorStatus | null>
  onChange: (config: EgressIdentityConfig) => void
  residentialPool?: ResidentialProxyPool
}

const starterProfile: EgressIdentityProfile = {
  id: 'stable-default',
  name: '稳定默认画像',
  enabled: true,
  preferred_nodes: [],
  preferred_pools: ['通用池'],
  required_ip_type: null,
  max_fraud_score: 70,
  dns_policy: {
    mode: 'Inherit',
    force_remote_dns: false,
  },
  tls_fingerprint: null,
  session_policy: {
    strict_affinity: false,
    ttl_override: null,
  },
  failover_policy: 'Manual',
  allowed_nodes: [],
  strict_node_scope: false,
  use_residential_chain: false,
  residential_proxy_name: null,
  description: '默认的稳定出口身份画像',
}

const starterAppRule: AppEgressRule = {
  process_name: 'Steam.exe',
  exe_path: null,
  domains: [],
  profile_id: 'stable-default',
  priority: 100,
  enabled: true,
}

const starterShortcutRule: ShortcutEgressRule = {
  shortcut_id: 'chatgpt',
  profile_id: 'stable-default',
  enabled: true,
}

const buildProfileId = (existingIds: string[]) => {
  let index = existingIds.length + 1
  let candidate = `profile-${index}`

  while (existingIds.includes(candidate)) {
    index += 1
    candidate = `profile-${index}`
  }

  return candidate
}

export function EgressIdentityPanel({
  config,
  status,
  onRefreshStatus,
  onChange,
  residentialPool,
}: Props) {
  const [previewResult, setPreviewResult] =
    useState<ResolvedEgressIdentity | null>(null)
  const [previewLoading, setPreviewLoading] = useState(false)
  const [assignLoading, setAssignLoading] = useState(false)
  const [assignmentsLoading, setAssignmentsLoading] = useState(false)
  const [previewForm, setPreviewForm] =
    useState<EgressIdentityPreviewFormState>(emptyEgressIdentityPreviewForm)

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

  const updateConfig = (nextConfig: EgressIdentityConfig) => {
    onChange(nextConfig)
  }

  const ensureInitialized = async () => {
    // 如果当前已经有部分配置，就补齐缺省模板并顺带启用。
    if (
      config.profiles.length > 0 ||
      config.app_rules.length > 0 ||
      config.shortcut_rules.length > 0
    ) {
      const initializedProfiles =
        config.profiles.length > 0 ? config.profiles : [starterProfile]
      const defaultProfile = config.default_profile || initializedProfiles[0]?.id || null

      updateConfig({
        ...config,
        enabled: true,
        default_profile: defaultProfile,
        profiles: initializedProfiles,
        app_rules: config.app_rules.length > 0 ? config.app_rules : [starterAppRule],
        shortcut_rules:
          config.shortcut_rules.length > 0
            ? config.shortcut_rules
            : [starterShortcutRule],
      })
      return
    }

    // 配置完全为空时，优先使用后端推荐的初始化模板。
    try {
      const recommended = await getRecommendedAdvancedConfig()
      const recommendedEgress = recommended.egress_identity

      updateConfig({
        ...recommendedEgress,
        enabled: true,
      })
    } catch (error: any) {
      showNotice(
        'error',
        error?.message || error?.toString() || '加载推荐出口身份配置失败',
      )
    }
  }

  const loadAssignments = async () => {
    setAssignmentsLoading(true)
    try {
      await onRefreshStatus()
    } finally {
      setAssignmentsLoading(false)
    }
  }

  const applyProfileReferenceUpdate = (
    oldProfileId: string,
    nextProfileId: string,
    nextProfiles: EgressIdentityProfile[],
  ) => {
    updateConfig({
      ...config,
      default_profile:
        config.default_profile === oldProfileId
          ? nextProfileId || null
          : config.default_profile,
      profiles: nextProfiles,
      app_rules: config.app_rules.map((rule) =>
        rule.profile_id === oldProfileId
          ? { ...rule, profile_id: nextProfileId }
          : rule,
      ),
      shortcut_rules: config.shortcut_rules.map((rule) =>
        rule.profile_id === oldProfileId
          ? { ...rule, profile_id: nextProfileId }
          : rule,
      ),
    })
  }

  const addProfile = () => {
    const nextId = buildProfileId(config.profiles.map((profile) => profile.id))
    const nextProfile: EgressIdentityProfile = {
      ...starterProfile,
      id: nextId,
      name: `画像 ${config.profiles.length + 1}`,
    }

    updateConfig({
      ...config,
      profiles: [...config.profiles, nextProfile],
      default_profile: config.default_profile || nextId,
    })
  }

  const updateProfile = (
    profileId: string,
    updater: (profile: EgressIdentityProfile) => EgressIdentityProfile,
  ) => {
    updateConfig({
      ...config,
      profiles: config.profiles.map((profile) =>
        profile.id === profileId ? updater(profile) : profile,
      ),
    })
  }

  const renameProfileId = (profileId: string, nextProfileId: string) => {
    const nextProfiles = config.profiles.map((profile) =>
      profile.id === profileId ? { ...profile, id: nextProfileId } : profile,
    )
    applyProfileReferenceUpdate(profileId, nextProfileId, nextProfiles)
  }

  const removeProfile = (profileId: string) => {
    const remainingProfiles = config.profiles.filter(
      (profile) => profile.id !== profileId,
    )
    const fallbackProfileId = remainingProfiles[0]?.id || null

    updateConfig({
      ...config,
      default_profile:
        config.default_profile === profileId
          ? fallbackProfileId
          : config.default_profile,
      profiles: remainingProfiles,
      app_rules: fallbackProfileId
        ? config.app_rules.map((rule) =>
            rule.profile_id === profileId
              ? { ...rule, profile_id: fallbackProfileId }
              : rule,
          )
        : config.app_rules.filter((rule) => rule.profile_id !== profileId),
      shortcut_rules: fallbackProfileId
        ? config.shortcut_rules.map((rule) =>
            rule.profile_id === profileId
              ? { ...rule, profile_id: fallbackProfileId }
              : rule,
          )
        : config.shortcut_rules.filter((rule) => rule.profile_id !== profileId),
    })
  }

  const addAppRule = () => {
    const profileId = config.default_profile || config.profiles[0]?.id

    if (!profileId) {
      showNotice('error', '请先添加至少一个出口画像')
      return
    }

    updateConfig({
      ...config,
      app_rules: [
        ...config.app_rules,
        {
          ...starterAppRule,
          profile_id: profileId,
        },
      ],
    })
  }

  const updateAppRule = (index: number, nextRule: AppEgressRule) => {
    const nextRules = [...config.app_rules]
    nextRules[index] = nextRule
    updateConfig({ ...config, app_rules: nextRules })
  }

  const removeAppRule = (index: number) => {
    updateConfig({
      ...config,
      app_rules: config.app_rules.filter(
        (_, currentIndex) => currentIndex !== index,
      ),
    })
  }

  const addShortcutRule = () => {
    const profileId = config.default_profile || config.profiles[0]?.id

    if (!profileId) {
      showNotice('error', '请先添加至少一个出口画像')
      return
    }

    updateConfig({
      ...config,
      shortcut_rules: [
        ...config.shortcut_rules,
        {
          ...starterShortcutRule,
          profile_id: profileId,
          shortcut_id: `shortcut-${config.shortcut_rules.length + 1}`,
        },
      ],
    })
  }

  const updateShortcutRule = (
    index: number,
    nextRule: ShortcutEgressRule,
  ) => {
    const nextRules = [...config.shortcut_rules]
    nextRules[index] = nextRule
    updateConfig({ ...config, shortcut_rules: nextRules })
  }

  const removeShortcutRule = (index: number) => {
    updateConfig({
      ...config,
      shortcut_rules: config.shortcut_rules.filter(
        (_, currentIndex) => currentIndex !== index,
      ),
    })
  }

  const buildPreviewRequest = (): EgressPreviewRequest => ({
    process_name: previewForm.process_name.trim() || undefined,
    exe_path: previewForm.exe_path.trim() || undefined,
    shortcut_id: previewForm.shortcut_id.trim() || undefined,
    domain: previewForm.domain.trim() || undefined,
    source_ip: previewForm.source_ip.trim() || undefined,
    source_port: previewForm.source_port.trim()
      ? Number.parseInt(previewForm.source_port, 10) || undefined
      : undefined,
    available_nodes: splitList(previewForm.available_nodes),
  })

  const handlePreview = async () => {
    setPreviewLoading(true)
    try {
      const result = await egressIdentityPreviewMatch(buildPreviewRequest())
      setPreviewResult(result)
    } catch (error: any) {
      showNotice(
        'error',
        error?.message || error?.toString() || '预览匹配失败',
      )
    } finally {
      setPreviewLoading(false)
    }
  }

  const handleAssign = async () => {
    setAssignLoading(true)
    try {
      const result = await egressIdentityAssignMatch(buildPreviewRequest())
      setPreviewResult(result)
      await loadAssignments()
      showNotice('success', '已创建运行时出口身份分配')
    } catch (error: any) {
      showNotice(
        'error',
        error?.message || error?.toString() || '创建分配失败',
      )
    } finally {
      setAssignLoading(false)
    }
  }

  const handleClearAssignment = async (key: string) => {
    try {
      await egressIdentityClearAssignment(key)
      await loadAssignments()
      showNotice('success', '运行时分配已清除')
    } catch (error: any) {
      showNotice(
        'error',
        error?.message || error?.toString() || '清除分配失败',
      )
    }
  }

  const handleToggleEnabled = (enabled: boolean) => {
    updateConfig({ ...config, enabled })
  }

  const handleClearRules = () => {
    updateConfig({
      ...config,
      app_rules: [],
      shortcut_rules: [],
    })
  }

  const handleDefaultProfileChange = (profileId: string) => {
    updateConfig({
      ...config,
      default_profile: profileId || null,
    })
  }

  const handlePreviewFormChange = (
    patch: Partial<EgressIdentityPreviewFormState>,
  ) => {
    setPreviewForm((current) => ({
      ...current,
      ...patch,
    }))
  }

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
        onRefreshAssignments={loadAssignments}
        onPreview={handlePreview}
        onAssign={handleAssign}
        onClearAssignment={handleClearAssignment}
      />
    </div>
  )
}
