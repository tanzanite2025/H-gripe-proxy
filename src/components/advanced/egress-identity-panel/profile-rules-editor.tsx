import { type ChangeEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Select } from '@/components/tailwind/Select'
import type { SelectPrimitiveValue } from '@/components/tailwind/Select'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import type {
  AppEgressRule,
  EgressIdentityConfig,
  EgressIdentityProfile,
  ResidentialProxyPool,
  ShortcutEgressRule,
} from '@/services/coordinator'

import {
  dnsModeOptions,
  failoverOptions,
  ipTypeOptions,
  joinList,
  splitList,
  type EgressProfileOption,
} from './shared'

interface Props {
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
}: Props) {
  const enabledResidentialProxies =
    residentialPool?.proxies.filter((proxy) => proxy.enabled) ?? []
  const hasEnabledResidentialProxies =
    Boolean(residentialPool?.enabled) && enabledResidentialProxies.length > 0

  return (
    <>
      <Card variant="outlined">
        <div className="space-y-4 p-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="font-semibold">默认画像与画像编辑</p>
              <p className="mt-1 text-sm text-gray-500">
                当快捷方式规则和应用规则都不匹配时，回退到默认画像。
              </p>
            </div>
            <Button size="small" variant="outlined" onClick={onAddProfile}>
              添加画像
            </Button>
          </div>

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
            <div className="py-8 text-center text-sm text-gray-500">
              暂无出口画像
            </div>
          ) : (
            <div className="space-y-4">
              {config.profiles.map((profile) => (
                <div
                  key={profile.id}
                  className="space-y-4 rounded-lg border border-gray-200 p-4 dark:border-gray-700"
                >
                  <div className="flex items-center justify-between gap-4">
                    <div>
                      <p className="font-medium">
                        {profile.name || profile.id || '未命名画像'}
                      </p>
                      <p className="mt-1 text-sm text-gray-500">
                        {config.default_profile === profile.id
                          ? '默认画像'
                          : '候选画像'}
                      </p>
                    </div>
                    <div className="flex items-center gap-2">
                      <Switch
                        checked={profile.enabled}
                        onCheckedChange={(checked) =>
                          onUpdateProfile(profile.id, (current) => ({
                            ...current,
                            enabled: checked,
                          }))
                        }
                      />
                      <Button
                        size="small"
                        variant="outlined"
                        onClick={() => onRemoveProfile(profile.id)}
                      >
                        删除画像
                      </Button>
                    </div>
                  </div>

                  <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                    <TextField
                      label="画像 ID"
                      value={profile.id}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        onRenameProfileId(profile.id, event.target.value)
                      }
                      fullWidth
                    />
                    <TextField
                      label="名称"
                      value={profile.name}
                      onChange={(event: ChangeEvent<HTMLInputElement>) =>
                        onUpdateProfile(profile.id, (current) => ({
                          ...current,
                          name: event.target.value,
                        }))
                      }
                      fullWidth
                    />
                    <Select
                      value={profile.required_ip_type || ''}
                      onChange={(value: SelectPrimitiveValue) =>
                        onUpdateProfile(profile.id, (current) => ({
                          ...current,
                          required_ip_type: String(value || '')
                            ? (String(value) as EgressIdentityProfile['required_ip_type'])
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
                        onUpdateProfile(profile.id, (current) => ({
                          ...current,
                          failover_policy:
                            String(value) as EgressIdentityProfile['failover_policy'],
                        }))
                      }
                      options={failoverOptions}
                      label="故障切换策略"
                      fullWidth
                    />
                    <Select
                      value={profile.dns_policy.mode}
                      onChange={(value: SelectPrimitiveValue) =>
                        onUpdateProfile(profile.id, (current) => ({
                          ...current,
                          dns_policy: {
                            ...current.dns_policy,
                            mode:
                              String(value) as EgressIdentityProfile['dns_policy']['mode'],
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
                        onUpdateProfile(profile.id, (current) => ({
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
                        onUpdateProfile(profile.id, (current) => ({
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
                        onUpdateProfile(profile.id, (current) => ({
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

                  <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                    <TextField
                      label="优先节点"
                      value={joinList(profile.preferred_nodes)}
                      onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                        onUpdateProfile(profile.id, (current) => ({
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
                        onUpdateProfile(profile.id, (current) => ({
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

                  <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                    <TextField
                      label="严格节点集合（allowed_nodes）"
                      value={joinList(profile.allowed_nodes ?? [])}
                      onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
                        onUpdateProfile(profile.id, (current) => ({
                          ...current,
                          allowed_nodes: splitList(event.target.value),
                        }))
                      }
                      helperText="仅当填写且启用严格节点范围时生效；用逗号或换行分隔节点名称"
                      multiline
                      rows={3}
                      fullWidth
                    />
                    <div className="flex items-center justify-between rounded-lg bg-gray-50 p-3 dark:bg-gray-900/40">
                      <div>
                        <p className="text-sm font-medium">严格节点范围</p>
                        <p className="mt-1 text-xs text-gray-500">
                          只在严格节点集合内选择出口节点，不再使用其他回退节点。
                        </p>
                      </div>
                      <Switch
                        checked={Boolean(profile.strict_node_scope)}
                        onCheckedChange={(checked: boolean) =>
                          onUpdateProfile(profile.id, (current) => ({
                            ...current,
                            strict_node_scope: checked,
                          }))
                        }
                      />
                    </div>
                  </div>

                  <div className="space-y-3 rounded-lg border border-orange-200 bg-orange-50 p-3 dark:border-orange-800 dark:bg-orange-900/20">
                    <div className="flex items-center justify-between">
                      <div>
                        <p className="text-sm font-medium">链式住宅路由</p>
                        <p className="mt-1 text-xs text-gray-500">
                          自动构建 VPS -&gt; 住宅 链式代理，使出口 IP 呈现
                          ISP/Residential ASN 特征。
                        </p>
                      </div>
                      <Switch
                        checked={Boolean(profile.use_residential_chain)}
                        onCheckedChange={(checked: boolean) =>
                          onUpdateProfile(profile.id, (current) => ({
                            ...current,
                            use_residential_chain: checked,
                          }))
                        }
                      />
                    </div>
                    {profile.use_residential_chain &&
                      hasEnabledResidentialProxies && (
                        <Select
                          label="指定住宅代理（留空自动选择）"
                          value={profile.residential_proxy_name || ''}
                          onChange={(value: SelectPrimitiveValue) =>
                            onUpdateProfile(profile.id, (current) => ({
                              ...current,
                              residential_proxy_name: String(value) || null,
                            }))
                          }
                          options={[
                            { value: '', label: '自动选择' },
                            ...enabledResidentialProxies.map((proxy) => ({
                              value: proxy.name,
                              label: `${proxy.name} (${proxy.proxyType.toUpperCase()} ${proxy.server}:${proxy.port}${proxy.region ? ` ${proxy.region}` : ''})`,
                            })),
                          ]}
                          fullWidth
                        />
                      )}
                    {profile.use_residential_chain &&
                      !hasEnabledResidentialProxies && (
                        <p className="text-xs text-orange-600 dark:text-orange-400">
                          住宅代理池未启用或无可用节点，请先在“住宅代理池”标签页添加并启用住宅代理。
                        </p>
                      )}
                  </div>

                  <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
                    <div className="flex items-center justify-between rounded-lg bg-gray-50 p-3 dark:bg-gray-900/40">
                      <div>
                        <p className="text-sm font-medium">强制粘性</p>
                        <p className="mt-1 text-xs text-gray-500">
                          尽量复用已有 assignment
                        </p>
                      </div>
                      <Switch
                        checked={profile.session_policy.strict_affinity}
                        onCheckedChange={(checked: boolean) =>
                          onUpdateProfile(profile.id, (current) => ({
                            ...current,
                            session_policy: {
                              ...current.session_policy,
                              strict_affinity: checked,
                            },
                          }))
                        }
                      />
                    </div>
                    <div className="flex items-center justify-between rounded-lg bg-gray-50 p-3 dark:bg-gray-900/40">
                      <div>
                        <p className="text-sm font-medium">强制远端 DNS</p>
                        <p className="mt-1 text-xs text-gray-500">
                          尽量避免 DNS 与出口位置不一致
                        </p>
                      </div>
                      <Switch
                        checked={profile.dns_policy.force_remote_dns}
                        onCheckedChange={(checked: boolean) =>
                          onUpdateProfile(profile.id, (current) => ({
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
                      onUpdateProfile(profile.id, (current) => ({
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
        <div className="space-y-4 p-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="font-semibold">应用规则</p>
              <p className="mt-1 text-sm text-gray-500">
                用进程名、可执行路径和域名模式把应用映射到目标画像。
              </p>
            </div>
            <Button size="small" variant="outlined" onClick={onAddAppRule}>
              添加应用规则
            </Button>
          </div>

          {config.app_rules.length === 0 ? (
            <div className="py-8 text-center text-sm text-gray-500">
              暂无应用规则
            </div>
          ) : (
            <div className="space-y-4">
              {config.app_rules.map((rule, index) => (
                <div
                  key={`${rule.process_name || ''}-${rule.exe_path || ''}-${rule.profile_id}-${rule.priority}`}
                  className="space-y-4 rounded-lg border border-gray-200 p-4 dark:border-gray-700"
                >
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
                        onUpdateAppRule(index, {
                          ...rule,
                          priority:
                            Number.parseInt(event.target.value, 10) || 0,
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
              ))}
            </div>
          )}
        </div>
      </Card>

      <Card variant="outlined">
        <div className="space-y-4 p-4">
          <div className="flex items-center justify-between gap-4">
            <div>
              <p className="font-semibold">快捷方式规则</p>
              <p className="mt-1 text-sm text-gray-500">
                用软件内快捷方式 ID 直接绑定到目标画像。
              </p>
            </div>
            <Button
              size="small"
              variant="outlined"
              onClick={onAddShortcutRule}
            >
              添加快捷方式规则
            </Button>
          </div>

          {config.shortcut_rules.length === 0 ? (
            <div className="py-8 text-center text-sm text-gray-500">
              暂无快捷方式规则
            </div>
          ) : (
            <div className="space-y-4">
              {config.shortcut_rules.map((rule, index) => (
                <div
                  key={`${rule.shortcut_id}-${rule.profile_id}`}
                  className="space-y-4 rounded-lg border border-gray-200 p-4 dark:border-gray-700"
                >
                  <div className="flex items-center justify-between gap-4">
                    <div>
                      <p className="font-medium">
                        快捷方式规则 {index + 1}
                      </p>
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
}
