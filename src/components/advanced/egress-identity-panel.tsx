import {
  useMemo,
  useState,
  type ChangeEvent,
} from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Select } from '@/components/tailwind/Select'
import type { SelectPrimitiveValue } from '@/components/tailwind/Select'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import type {
  AppEgressRule,
  CoordinatorStatus,
  DnsMode,
  EgressFailoverPolicy,
  EgressIdentityConfig,
  EgressIdentityProfile,
  IpType,
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

interface Props {
  config: EgressIdentityConfig
  status: CoordinatorStatus
  onRefreshStatus: () => Promise<CoordinatorStatus | null>
  onChange: (config: EgressIdentityConfig) => void
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

const dnsModeOptions: { value: DnsMode; label: string }[] = [
  { value: 'Inherit', label: '继承' },
  { value: 'Hijack', label: '强制劫持' },
  { value: 'Remote', label: '强制远端 DNS' },
]

const failoverOptions: { value: EgressFailoverPolicy; label: string }[] = [
  { value: 'Block', label: '阻止' },
  { value: 'Manual', label: '手动确认' },
  { value: 'AutoSwitch', label: '自动切换' },
]

const ipTypeOptions: { value: '' | IpType; label: string }[] = [
  { value: '', label: '不限制' },
  { value: 'Datacenter', label: '机房 IP' },
  { value: 'Residential', label: '住宅 IP' },
  { value: 'Mobile', label: '移动 IP' },
  { value: 'Unknown', label: '未知' },
]

const splitList = (value: string) =>
  value
    .split(/[\n,]/)
    .map((item) => item.trim())
    .filter(Boolean)

const joinList = (values: string[]) => values.join(', ')

const buildProfileId = (existingIds: string[]) => {
  let index = existingIds.length + 1
  let candidate = `profile-${index}`

  while (existingIds.includes(candidate)) {
    index += 1
    candidate = `profile-${index}`
  }

  return candidate
}

export function EgressIdentityPanel({ config, status, onRefreshStatus, onChange }: Props) {
  const [previewResult, setPreviewResult] = useState<ResolvedEgressIdentity | null>(null)
  const [previewLoading, setPreviewLoading] = useState(false)
  const [assignLoading, setAssignLoading] = useState(false)
  const [assignmentsLoading, setAssignmentsLoading] = useState(false)
  const [previewForm, setPreviewForm] = useState({
    process_name: '',
    exe_path: '',
    shortcut_id: '',
    domain: '',
    source_ip: '',
    source_port: '',
    available_nodes: '',
  })

  const profileOptions = useMemo(
    () => [
      { value: '', label: '不设置默认画像' },
      ...config.profiles.map((profile) => ({
        value: profile.id,
        label: `${profile.name} (${profile.id})${profile.enabled ? '' : ' · 已禁用'}`,
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
  const domainPatternAssignments = status.runtimeState.stableEgressBackwrite.domainPatternAssignments
  const regularAssignments = activeAssignments.filter(
    (assignment) => !assignment.assignmentKey?.startsWith('domain-pattern:'),
  )

  const updateConfig = (nextConfig: EgressIdentityConfig) => {
    onChange(nextConfig)
  }

  const ensureInitialized = async () => {
    // 如果当前已经有配置内容，保持现有“填补缺省 + 启用”语义
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
          config.shortcut_rules.length > 0 ? config.shortcut_rules : [starterShortcutRule],
      })
      return
    }

    // 当配置完全为空时，使用后端推荐的 AdvancedConfig.egress_identity 作为初始化模板
    try {
      const recommended = await getRecommendedAdvancedConfig()
      const recommendedEgress = recommended.egress_identity

      updateConfig({
        ...recommendedEgress,
        enabled: true,
      })
    } catch (error: any) {
      showNotice('error', error?.message || error?.toString() || '加载推荐出口身份配置失败')
    }
  }

  const loadAssignments = async () => {
    setAssignmentsLoading(true)
    await onRefreshStatus()
    setAssignmentsLoading(false)
  }

  const applyProfileReferenceUpdate = (
    oldProfileId: string,
    nextProfileId: string,
    nextProfiles: EgressIdentityProfile[],
  ) => {
    updateConfig({
      ...config,
      default_profile:
        config.default_profile === oldProfileId ? nextProfileId || null : config.default_profile,
      profiles: nextProfiles,
      app_rules: config.app_rules.map((rule) =>
        rule.profile_id === oldProfileId ? { ...rule, profile_id: nextProfileId } : rule,
      ),
      shortcut_rules: config.shortcut_rules.map((rule) =>
        rule.profile_id === oldProfileId ? { ...rule, profile_id: nextProfileId } : rule,
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
    const remainingProfiles = config.profiles.filter((profile) => profile.id !== profileId)
    const fallbackProfileId = remainingProfiles[0]?.id || null

    updateConfig({
      ...config,
      default_profile:
        config.default_profile === profileId ? fallbackProfileId : config.default_profile,
      profiles: remainingProfiles,
      app_rules: fallbackProfileId
        ? config.app_rules.map((rule) =>
            rule.profile_id === profileId ? { ...rule, profile_id: fallbackProfileId } : rule,
          )
        : config.app_rules.filter((rule) => rule.profile_id !== profileId),
      shortcut_rules: fallbackProfileId
        ? config.shortcut_rules.map((rule) =>
            rule.profile_id === profileId ? { ...rule, profile_id: fallbackProfileId } : rule,
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
      app_rules: config.app_rules.filter((_, currentIndex) => currentIndex !== index),
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

  const updateShortcutRule = (index: number, nextRule: ShortcutEgressRule) => {
    const nextRules = [...config.shortcut_rules]
    nextRules[index] = nextRule
    updateConfig({ ...config, shortcut_rules: nextRules })
  }

  const removeShortcutRule = (index: number) => {
    updateConfig({
      ...config,
      shortcut_rules: config.shortcut_rules.filter((_, currentIndex) => currentIndex !== index),
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
      showNotice('error', error?.message || error?.toString() || '预览匹配失败')
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
      showNotice('success', '已创建运行态出口身份分配')
    } catch (error: any) {
      showNotice('error', error?.message || error?.toString() || '创建分配失败')
    } finally {
      setAssignLoading(false)
    }
  }

  const handleClearAssignment = async (key: string) => {
    try {
      await egressIdentityClearAssignment(key)
      await loadAssignments()
      showNotice('success', '运行态分配已清除')
    } catch (error: any) {
      showNotice('error', error?.message || error?.toString() || '清除分配失败')
    }
  }

  const renderOverviewAndInit = () => (
    <>
      <div className="p-4 bg-blue-500 text-white rounded-lg">
        <p className="text-sm">
          代理软件会把应用、快捷方式和业务会话统一映射到稳定的出口身份，确保同一主体尽量持续使用同一出口画像和同一节点。
        </p>
      </div>

      <Card variant="outlined">
        <div className="p-4 space-y-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="font-semibold">启用出口身份管理</p>
              <p className="text-sm text-gray-500 mt-1">
                将出口选择提升为“画像 + 规则 + 运行态 assignment”的统一模型。
              </p>
            </div>
            <Switch
              checked={config.enabled}
              onCheckedChange={(checked) => updateConfig({ ...config, enabled: checked })}
            />
          </div>

          <div className="grid grid-cols-1 md:grid-cols-5 gap-3">
            <div className="rounded-lg bg-gray-50 dark:bg-gray-900/40 p-3">
              <div className="text-2xl font-bold">{config.profiles.length}</div>
              <div className="text-sm text-gray-500 mt-1">出口画像</div>
            </div>
            <div className="rounded-lg bg-gray-50 dark:bg-gray-900/40 p-3">
              <div className="text-2xl font-bold">{config.app_rules.length}</div>
              <div className="text-sm text-gray-500 mt-1">应用规则</div>
            </div>
            <div className="rounded-lg bg-gray-50 dark:bg-gray-900/40 p-3">
              <div className="text-2xl font-bold">{config.shortcut_rules.length}</div>
              <div className="text-sm text-gray-500 mt-1">快捷方式规则</div>
            </div>
            <div className="rounded-lg bg-gray-50 dark:bg-gray-900/40 p-3">
              <div className="text-2xl font-bold">{activeAssignments.length}</div>
              <div className="text-sm text-gray-500 mt-1">运行态 assignment</div>
            </div>
            <div className="rounded-lg bg-purple-50 dark:bg-purple-950/30 p-3">
              <div className="text-2xl font-bold">{domainPatternAssignments.length}</div>
              <div className="text-sm text-gray-500 mt-1">domain-pattern 回写</div>
            </div>
          </div>

          <div className="flex flex-wrap gap-2">
            <Button size="small" variant="outlined" onClick={ensureInitialized}>
              初始化模板
            </Button>
            <Button
              size="small"
              variant="outlined"
              onClick={() =>
                updateConfig({
                  ...config,
                  app_rules: [],
                  shortcut_rules: [],
                })
              }
            >
              清空规则
            </Button>
          </div>
        </div>
      </Card>
    </>
  )

  const renderProfileAndRulesEditor = () => (
    <>
      <Card variant="outlined">
        <div className="p-4 space-y-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="font-semibold">默认画像与画像编辑</p>
              <p className="text-sm text-gray-500 mt-1">
                当快捷方式规则和应用规则都不匹配时，回退到默认画像。
              </p>
            </div>
            <Button size="small" variant="outlined" onClick={addProfile}>
              添加画像
            </Button>
          </div>

          <Select
            value={config.default_profile || ''}
            onChange={(value: SelectPrimitiveValue) =>
              updateConfig({
                ...config,
                default_profile: String(value || '') || null,
              })
            }
            options={profileOptions}
            label="默认画像"
            fullWidth
          />

          {config.profiles.length === 0 ? (
            <div className="text-sm text-gray-500 py-8 text-center">暂无出口画像</div>
          ) : (
            <div className="space-y-4">
              {config.profiles.map((profile) => (
                <div
                  key={profile.id}
                  className="rounded-lg border border-gray-200 dark:border-gray-700 p-4 space-y-4"
                >
                  <div className="flex items-center justify-between gap-4">
                    <div>
                      <p className="font-medium">{profile.name || profile.id || '未命名画像'}</p>
                      <p className="text-sm text-gray-500 mt-1">
                        {config.default_profile === profile.id ? '默认画像' : '候选画像'}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Switch
                        checked={profile.enabled}
                        onCheckedChange={(checked) =>
                          updateProfile(profile.id, (current) => ({
                            ...current,
                            enabled: checked,
                          }))
                        }
                      />
                      <Button
                        size="small"
                        variant="outlined"
                        onClick={() => removeProfile(profile.id)}
                      >
                        删除画像
                      </Button>
                    </div>
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <TextField
                      label="画像 ID"
                      value={profile.id}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        renameProfileId(profile.id, event.target.value)
                      }
                      fullWidth
                    />
                    <TextField
                      label="名称"
                      value={profile.name}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          name: event.target.value,
                        }))
                      }
                      fullWidth
                    />
                    <Select
                      value={profile.required_ip_type || ''}
                      onChange={(value: SelectPrimitiveValue) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          required_ip_type: String(value || '')
                            ? (String(value) as IpType)
                            : null,
                        }))
                      }
                      options={ipTypeOptions}
                      label="要求的 IP 类型"
                      fullWidth
                    />
                    <Select
                      value={profile.failover_policy}
                      onChange={(value: SelectPrimitiveValue) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          failover_policy: String(value) as EgressFailoverPolicy,
                        }))
                      }
                      options={failoverOptions}
                      label="故障转移策略"
                      fullWidth
                    />
                    <Select
                      value={profile.dns_policy.mode}
                      onChange={(value: SelectPrimitiveValue) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          dns_policy: {
                            ...current.dns_policy,
                            mode: String(value) as DnsMode,
                          },
                        }))
                      }
                      options={dnsModeOptions}
                      label="DNS 策略"
                      fullWidth
                    />
                    <TextField
                      label="TLS 指纹"
                      value={profile.tls_fingerprint || ''}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          tls_fingerprint: event.target.value || null,
                        }))
                      }
                      helperText="留空表示继承全局 TLS 指纹策略"
                      fullWidth
                    />
                    <TextField
                      label="最大欺诈评分"
                      type="number"
                      value={profile.max_fraud_score ?? ''}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          max_fraud_score: event.target.value
                            ? Number.parseInt(event.target.value, 10)
                            : null,
                        }))
                      }
                      fullWidth
                    />
                    <TextField
                      label="会话 TTL 覆盖（秒）"
                      type="number"
                      value={profile.session_policy.ttl_override ?? ''}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          session_policy: {
                            ...current.session_policy,
                            ttl_override: event.target.value
                              ? Number.parseInt(event.target.value, 10)
                              : null,
                          },
                        }))
                      }
                      fullWidth
                    />
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <TextField
                      label="优先节点"
                      value={joinList(profile.preferred_nodes)}
                      onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          preferred_nodes: splitList(event.target.value),
                        }))
                      }
                      helperText="用逗号或换行分隔节点名称"
                      multiline
                      rows={3}
                      fullWidth
                    />
                    <TextField
                      label="优先节点池"
                      value={joinList(profile.preferred_pools)}
                      onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          preferred_pools: splitList(event.target.value),
                        }))
                      }
                      helperText="用逗号或换行分隔池名称"
                      multiline
                      rows={3}
                      fullWidth
                    />
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <TextField
                      label="严格节点集合（allowed_nodes）"
                      value={joinList(profile.allowed_nodes ?? [])}
                      onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                        updateProfile(profile.id, (current) => ({
                          ...current,
                          allowed_nodes: splitList(event.target.value),
                        }))
                      }
                      helperText="仅当填写且启用严格节点范围时生效；用逗号或换行分隔节点名称"
                      multiline
                      rows={3}
                      fullWidth
                    />
                    <div className="rounded-lg bg-gray-50 dark:bg-gray-900/40 p-3 flex items-center justify-between">
                      <div>
                        <p className="text-sm font-medium">严格节点范围</p>
                        <p className="text-xs text-gray-500 mt-1">
                          只在严格节点集合内选择出口节点，不再使用其他回退节点。
                        </p>
                      </div>
                      <Switch
                        checked={Boolean(profile.strict_node_scope)}
                        onCheckedChange={(checked: boolean) =>
                          updateProfile(profile.id, (current) => ({
                            ...current,
                            strict_node_scope: checked,
                          }))
                        }
                      />
                    </div>
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <div className="rounded-lg bg-gray-50 dark:bg-gray-900/40 p-3 flex items-center justify-between">
                      <div>
                        <p className="text-sm font-medium">强制粘性</p>
                        <p className="text-xs text-gray-500 mt-1">尽量复用已有 assignment</p>
                      </div>
                      <Switch
                        checked={profile.session_policy.strict_affinity}
                        onCheckedChange={(checked: boolean) =>
                          updateProfile(profile.id, (current) => ({
                            ...current,
                            session_policy: {
                              ...current.session_policy,
                              strict_affinity: checked,
                            },
                          }))
                        }
                      />
                    </div>
                    <div className="rounded-lg bg-gray-50 dark:bg-gray-900/40 p-3 flex items-center justify-between">
                      <div>
                        <p className="text-sm font-medium">强制远端 DNS</p>
                        <p className="text-xs text-gray-500 mt-1">尽量避免 DNS 与出口位置不一致</p>
                      </div>
                      <Switch
                        checked={profile.dns_policy.force_remote_dns}
                        onCheckedChange={(checked: boolean) =>
                          updateProfile(profile.id, (current) => ({
                            ...current,
                            dns_policy: {
                              ...current.dns_policy,
                              force_remote_dns: checked,
                            },
                          }))
                        }
                      />
                    </div>
                  </div>

                  <TextField
                    label="描述"
                    value={profile.description}
                    onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                      updateProfile(profile.id, (current) => ({
                        ...current,
                        description: event.target.value,
                      }))
                    }
                    multiline
                    rows={3}
                    fullWidth
                  />
                </div>
              ))}
            </div>
          )}
        </div>
      </Card>

      <Card variant="outlined">
        <div className="p-4 space-y-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="font-semibold">应用规则</p>
              <p className="text-sm text-gray-500 mt-1">
                用进程名、可执行路径和域名模式把应用映射到目标画像。
              </p>
            </div>
            <Button size="small" variant="outlined" onClick={addAppRule}>
              添加应用规则
            </Button>
          </div>

          {config.app_rules.length === 0 ? (
            <div className="text-sm text-gray-500 py-8 text-center">暂无应用规则</div>
          ) : (
            <div className="space-y-4">
              {config.app_rules.map((rule, index) => (
                <div
                  key={`${rule.process_name || ''}-${rule.exe_path || ''}-${rule.profile_id}-${rule.priority}`}
                  className="rounded-lg border border-gray-200 dark:border-gray-700 p-4 space-y-4"
                >
                  <div className="flex items-center justify-between gap-4">
                    <div>
                      <p className="font-medium">规则 {index + 1}</p>
                      <p className="text-sm text-gray-500 mt-1">优先级数字越小越先匹配</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Switch
                        checked={rule.enabled}
                        onCheckedChange={(checked) =>
                          updateAppRule(index, { ...rule, enabled: checked })
                        }
                      />
                      <Button size="small" variant="outlined" onClick={() => removeAppRule(index)}>
                        删除规则
                      </Button>
                    </div>
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <TextField
                      label="进程名"
                      value={rule.process_name || ''}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        updateAppRule(index, {
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
                        updateAppRule(index, {
                          ...rule,
                          exe_path: event.target.value || null,
                        })
                      }
                      fullWidth
                    />
                    <Select
                      value={rule.profile_id}
                      onChange={(value: SelectPrimitiveValue) =>
                        updateAppRule(index, { ...rule, profile_id: String(value) })
                      }
                      options={config.profiles.map((profile) => ({
                        value: profile.id,
                        label: `${profile.name} (${profile.id})`,
                      }))}
                      label="目标画像"
                      fullWidth
                    />
                    <TextField
                      label="优先级"
                      type="number"
                      value={rule.priority}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        updateAppRule(index, {
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
                      updateAppRule(index, {
                        ...rule,
                        domains: splitList(event.target.value),
                      })
                    }
                    helperText="用逗号或换行分隔，如 *.openai.com"
                    multiline
                    rows={3}
                    fullWidth
                  />
                </div>
              ))}
            </div>
          )}
        </div>
      </Card>

      <Card variant="outlined">
        <div className="p-4 space-y-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="font-semibold">快捷方式规则</p>
              <p className="text-sm text-gray-500 mt-1">
                用软件内快捷方式 ID 直接绑定到目标画像。
              </p>
            </div>
            <Button size="small" variant="outlined" onClick={addShortcutRule}>
              添加快捷方式规则
            </Button>
          </div>

          {config.shortcut_rules.length === 0 ? (
            <div className="text-sm text-gray-500 py-8 text-center">暂无快捷方式规则</div>
          ) : (
            <div className="space-y-4">
              {config.shortcut_rules.map((rule, index) => (
                <div
                  key={`${rule.shortcut_id}-${rule.profile_id}`}
                  className="rounded-lg border border-gray-200 dark:border-gray-700 p-4 space-y-4"
                >
                  <div className="flex items-center justify-between gap-4">
                    <div>
                      <p className="font-medium">快捷方式规则 {index + 1}</p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Switch
                        checked={rule.enabled}
                        onCheckedChange={(checked) =>
                          updateShortcutRule(index, { ...rule, enabled: checked })
                        }
                      />
                      <Button
                        size="small"
                        variant="outlined"
                        onClick={() => removeShortcutRule(index)}
                      >
                        删除规则
                      </Button>
                    </div>
                  </div>

                  <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                    <TextField
                      label="快捷方式 ID"
                      value={rule.shortcut_id}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        updateShortcutRule(index, {
                          ...rule,
                          shortcut_id: event.target.value,
                        })
                      }
                      fullWidth
                    />
                    <Select
                      value={rule.profile_id}
                      onChange={(value: SelectPrimitiveValue) =>
                        updateShortcutRule(index, { ...rule, profile_id: String(value) })
                      }
                      options={config.profiles.map((profile) => ({
                        value: profile.id,
                        label: `${profile.name} (${profile.id})`,
                      }))}
                      label="目标画像"
                      fullWidth
                    />
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>
      </Card>
    </>
  )

  const renderRuntimeTools = () => (
    <Card variant="outlined">
      <div className="p-4 space-y-4">
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="font-semibold">运行态诊断</p>
            <p className="text-sm text-gray-500 mt-1">
              这里可以直接预览或创建运行态 assignment，并查看当前活跃的出口身份分配。
            </p>
          </div>
          <Button size="small" variant="outlined" onClick={loadAssignments} loading={assignmentsLoading}>
            刷新 assignment
          </Button>
        </div>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <TextField
            label="域名"
            value={previewForm.domain}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              setPreviewForm((current) => ({ ...current, domain: event.target.value }))
            }
            fullWidth
          />
          <TextField
            label="快捷方式 ID"
            value={previewForm.shortcut_id}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              setPreviewForm((current) => ({ ...current, shortcut_id: event.target.value }))
            }
            fullWidth
          />
          <TextField
            label="进程名"
            value={previewForm.process_name}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              setPreviewForm((current) => ({ ...current, process_name: event.target.value }))
            }
            fullWidth
          />
          <TextField
            label="可执行路径"
            value={previewForm.exe_path}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              setPreviewForm((current) => ({ ...current, exe_path: event.target.value }))
            }
            fullWidth
          />
          <TextField
            label="源 IP"
            value={previewForm.source_ip}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              setPreviewForm((current) => ({ ...current, source_ip: event.target.value }))
            }
            fullWidth
          />
          <TextField
            label="源端口"
            value={previewForm.source_port}
            onChange={(event: ChangeEvent<HTMLInputElement>) =>
              setPreviewForm((current) => ({ ...current, source_port: event.target.value }))
            }
            fullWidth
          />
        </div>

        <TextField
          label="可用节点"
          value={previewForm.available_nodes}
          onChange={(event: ChangeEvent<HTMLInputElement>) =>
            setPreviewForm((current) => ({ ...current, available_nodes: event.target.value }))
          }
          helperText="用逗号或换行分隔，例如 hk-01, us-02"
          multiline
          rows={3}
          fullWidth
        />

        <div className="flex flex-wrap gap-2">
          <Button size="small" variant="outlined" onClick={handlePreview} loading={previewLoading} disabled={!config.enabled}>
            预览匹配
          </Button>
          <Button size="small" variant="primary" onClick={handleAssign} loading={assignLoading} disabled={!config.enabled}>
            创建 assignment
          </Button>
        </div>

        {!config.enabled && (
          <div className="p-3 bg-yellow-500 text-white rounded-lg text-sm">
            先启用出口身份管理，再进行运行态预览或创建 assignment。
          </div>
        )}

        {previewResult && (
          <div className="rounded-lg bg-green-500 text-white p-3 text-sm">
            <div>画像：{profileNameMap[previewResult.profileId] || previewResult.profileId}</div>
            <div>节点：{previewResult.selectedNode}</div>
            <div>DNS：{previewResult.dnsMode}</div>
            <div>匹配来源：{previewResult.matchedBy}</div>
            <div>Assignment Key：{previewResult.assignmentKey || '预览模式'}</div>
          </div>
        )}

        <div className="space-y-4">
          <div className="rounded-lg border border-purple-200 dark:border-purple-800 p-4 space-y-3 bg-purple-50/60 dark:bg-purple-950/20">
            <div>
              <div className="font-medium">稳定出口回写（domain-pattern）</div>
              <div className="text-sm text-gray-500 mt-1">
                这里展示稳定组手动选择回写到 `egress_identity` 后形成的域名模式级运行态状态。
              </div>
            </div>

            {domainPatternAssignments.length === 0 ? (
              <div className="text-sm text-gray-500 py-4 text-center">暂无 domain-pattern 回写 assignment</div>
            ) : (
              domainPatternAssignments.map((assignment) => (
                <div
                  key={`${assignment.assignmentKey || assignment.profileId}-${assignment.selectedNode}`}
                  className="rounded-lg border border-purple-200 dark:border-purple-800 p-3 flex items-center justify-between gap-4 bg-card"
                >
                  <div>
                    <div className="font-medium">
                      {profileNameMap[assignment.profileId] || assignment.profileId}
                    </div>
                    <div className="text-sm text-gray-500 mt-1">
                      {assignment.assignmentKey || '无 assignment key'} · {assignment.matchedBy}
                    </div>
                    {assignment.sourceGroupName && (
                      <div className="text-xs text-purple-600 mt-1">
                        来源稳定组：{assignment.sourceGroupName}
                      </div>
                    )}
                    <div className="grid grid-cols-1 md:grid-cols-2 gap-2 mt-2 text-xs">
                      <div className="rounded border border-purple-200 dark:border-purple-800 px-2 py-1 bg-purple-50/60 dark:bg-purple-950/20">
                        <span className="text-gray-500">来源组当前选中节点：</span>
                        <span className="ml-1 font-medium text-purple-700 dark:text-purple-300">
                          {assignment.sourceGroupSelectedNode || '未知'}
                        </span>
                      </div>
                      <div className="rounded border border-blue-200 dark:border-blue-800 px-2 py-1 bg-blue-50/60 dark:bg-blue-950/20">
                        <span className="text-gray-500">回写节点：</span>
                        <span className="ml-1 font-medium text-blue-700 dark:text-blue-300">
                          {assignment.selectedNode}
                        </span>
                      </div>
                    </div>
                  </div>
                  <Button
                    size="small"
                    variant="outlined"
                    disabled={!assignment.assignmentKey}
                    onClick={() => assignment.assignmentKey && handleClearAssignment(assignment.assignmentKey)}
                  >
                    清除
                  </Button>
                </div>
              ))
            )}
          </div>

          <div className="rounded-lg border border-gray-200 dark:border-gray-700 p-4 space-y-3">
            <div>
              <div className="font-medium">普通运行态 assignment</div>
              <div className="text-sm text-gray-500 mt-1">
                这里展示由应用、快捷方式、连接上下文等直接生成的常规运行态 assignment。
              </div>
            </div>

            {regularAssignments.length === 0 ? (
              <div className="text-sm text-gray-500 py-4 text-center">暂无普通运行态 assignment</div>
            ) : (
              regularAssignments.map((assignment) => (
                <div
                  key={`${assignment.assignmentKey || assignment.profileId}-${assignment.selectedNode}`}
                  className="rounded-lg border border-gray-200 dark:border-gray-700 p-3 flex items-center justify-between gap-4"
                >
                  <div>
                    <div className="font-medium">
                      {profileNameMap[assignment.profileId] || assignment.profileId}
                    </div>
                    <div className="text-sm text-gray-500 mt-1">
                      {assignment.assignmentKey || '无 assignment key'} · {assignment.selectedNode} · {assignment.matchedBy}
                    </div>
                  </div>
                  <Button
                    size="small"
                    variant="outlined"
                    disabled={!assignment.assignmentKey}
                    onClick={() => assignment.assignmentKey && handleClearAssignment(assignment.assignmentKey)}
                  >
                    清除
                  </Button>
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </Card>
  )

  return (
    <div className="space-y-4">
      {renderOverviewAndInit()}
      {renderProfileAndRulesEditor()}
      {renderRuntimeTools()}
    </div>
  )
}
