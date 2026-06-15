import { type ChangeEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Select } from '@/components/tailwind/Select'
import type { SelectPrimitiveValue } from '@/components/tailwind/Select'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import type {
  EgressIdentityProfile,
  ResidentialProxyPool,
} from '@/services/coordinator'

import {
  dnsModeOptions,
  failoverOptions,
  ipTypeOptions,
  joinList,
  splitList,
} from '../../shared'
import { getProfileTitle } from '../shared'

import { SettingSwitchCard } from './setting-switch-card'

interface ProfileCardProps {
  profile: EgressIdentityProfile
  isDefaultProfile: boolean
  enabledResidentialProxies: ResidentialProxyPool['proxies']
  hasEnabledResidentialProxies: boolean
  onUpdateProfile: (
    profileId: string,
    updater: (profile: EgressIdentityProfile) => EgressIdentityProfile,
  ) => void
  onRenameProfileId: (profileId: string, nextProfileId: string) => void
  onRemoveProfile: (profileId: string) => void
}

export function ProfileCard({
  profile,
  isDefaultProfile,
  enabledResidentialProxies,
  hasEnabledResidentialProxies,
  onUpdateProfile,
  onRenameProfileId,
  onRemoveProfile,
}: ProfileCardProps) {
  const updateCurrent = (
    updater: (current: EgressIdentityProfile) => EgressIdentityProfile,
  ) => onUpdateProfile(profile.id, updater)

  return (
    <div className="space-y-4 rounded-lg border border-gray-200 p-4 dark:border-gray-700">
      <div className="flex items-center justify-between gap-4">
        <div>
          <p className="font-medium">{getProfileTitle(profile)}</p>
          <p className="mt-1 text-sm text-gray-500">
            {isDefaultProfile ? '默认画像' : '候选画像'}
          </p>
        </div>
        <div className="flex items-center gap-2">
          <Switch
            checked={profile.enabled}
            onCheckedChange={(checked) =>
              updateCurrent((current) => ({
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
            updateCurrent((current) => ({
              ...current,
              name: event.target.value,
            }))
          }
          fullWidth
        />
        <Select
          value={profile.required_ip_type || ''}
          onChange={(value: SelectPrimitiveValue) =>
            updateCurrent((current) => ({
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
            updateCurrent((current) => ({
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
            updateCurrent((current) => ({
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
            updateCurrent((current) => ({
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
            updateCurrent((current) => ({
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
            updateCurrent((current) => ({
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
            updateCurrent((current) => ({
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
            updateCurrent((current) => ({
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
            updateCurrent((current) => ({
              ...current,
              allowed_nodes: splitList(event.target.value),
            }))
          }
          helperText="仅当填写且启用严格节点范围时生效；用逗号或换行分隔节点名称"
          multiline
          rows={3}
          fullWidth
        />
        <SettingSwitchCard
          title="严格节点范围"
          description="只在严格节点集合内选择出口节点，不再使用其他回退节点。"
          checked={Boolean(profile.strict_node_scope)}
          onCheckedChange={(checked) =>
            updateCurrent((current) => ({
              ...current,
              strict_node_scope: checked,
            }))
          }
        />
      </div>

      <div className="space-y-3 rounded-lg border border-orange-200 bg-orange-50 p-3 dark:border-orange-800 dark:bg-orange-900/20">
        <div className="flex items-center justify-between">
          <div>
            <p className="text-sm font-medium">链式住宅路由</p>
            <p className="mt-1 text-xs text-gray-500">
              自动构建 VPS -&gt; 住宅 链式代理，使出口 IP 呈现 ISP/Residential ASN 特征。
            </p>
          </div>
          <Switch
            checked={Boolean(profile.use_residential_chain)}
            onCheckedChange={(checked) =>
              updateCurrent((current) => ({
                ...current,
                use_residential_chain: checked,
              }))
            }
          />
        </div>

        {profile.use_residential_chain && hasEnabledResidentialProxies && (
          <Select
            label="指定住宅代理（留空自动选择）"
            value={profile.residential_proxy_name || ''}
            onChange={(value: SelectPrimitiveValue) =>
              updateCurrent((current) => ({
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

        {profile.use_residential_chain && !hasEnabledResidentialProxies && (
          <p className="text-xs text-orange-600 dark:text-orange-400">
            住宅代理池未启用或无可用节点，请先在“住宅代理池”标签页添加并启用住宅代理。
          </p>
        )}
      </div>

      <div className="grid grid-cols-1 gap-4 md:grid-cols-2">
        <SettingSwitchCard
          title="强制粘性"
          description="尽量复用已有 assignment。"
          checked={profile.session_policy.strict_affinity}
          onCheckedChange={(checked) =>
            updateCurrent((current) => ({
              ...current,
              session_policy: {
                ...current.session_policy,
                strict_affinity: checked,
              },
            }))
          }
        />
        <SettingSwitchCard
          title="强制远端 DNS"
          description="尽量避免 DNS 与出口位置不一致。"
          checked={profile.dns_policy.force_remote_dns}
          onCheckedChange={(checked) =>
            updateCurrent((current) => ({
              ...current,
              dns_policy: {
                ...current.dns_policy,
                force_remote_dns: checked,
              },
            }))
          }
        />
      </div>

      <TextField
        label="描述"
        value={profile.description}
        onChange={(event: ChangeEvent<HTMLTextAreaElement>) =>
          updateCurrent((current) => ({
            ...current,
            description: event.target.value,
          }))
        }
        multiline
        rows={3}
        fullWidth
      />
    </div>
  )
}
