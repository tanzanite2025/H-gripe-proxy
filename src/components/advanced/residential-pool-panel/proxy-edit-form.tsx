import { useState } from 'react'

import { Switch } from '@/components/base'
import {
  Button,
  DialogActions,
  TextField,
} from '@/components/tailwind'
import { Select } from '@/components/tailwind/Select'
import type { SelectPrimitiveValue } from '@/components/tailwind/Select'
import type { ResidentialProxy, ResidentialProxyType } from '@/services/coordinator'

import { PROXY_TYPES, REGION_OPTIONS } from './constants'

interface ResidentialProxyEditFormProps {
  proxy: ResidentialProxy
  existingNames: string[]
  isAdding: boolean
  onSave: (proxy: ResidentialProxy) => void
  onCancel: () => void
}

export function ResidentialProxyEditForm({
  proxy,
  existingNames,
  isAdding,
  onSave,
  onCancel,
}: ResidentialProxyEditFormProps) {
  const [form, setForm] = useState<ResidentialProxy>({ ...proxy })

  const trimmedName = form.name.trim()
  const trimmedServer = form.server.trim()
  const isNameDuplicate = existingNames.includes(trimmedName)
  const isValid =
    trimmedName !== '' &&
    trimmedServer !== '' &&
    form.port > 0 &&
    !isNameDuplicate

  const showAuthFields =
    form.proxyType === 'socks5' || form.proxyType === 'http'
  const showSsFields = form.proxyType === 'ss'
  const showVmessFields = form.proxyType === 'vmess'
  const showTrojanFields = form.proxyType === 'trojan'

  return (
    <div className="mt-3 space-y-3">
      <div className="grid grid-cols-2 gap-3">
        <TextField
          label="名称"
          value={form.name}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setForm({ ...form, name: event.target.value })
          }
          error={isNameDuplicate}
          helperText={isNameDuplicate ? '名称已存在' : ''}
          placeholder={isAdding ? 'US-Residential-1' : undefined}
          size="small"
        />
        <Select
          label="协议类型"
          value={form.proxyType}
          onChange={(value: SelectPrimitiveValue) =>
            setForm({
              ...form,
              proxyType: String(value) as ResidentialProxyType,
            })
          }
          options={PROXY_TYPES.map((item) => ({
            value: item.value,
            label: item.label,
          }))}
          size="small"
        />
      </div>

      <div className="grid grid-cols-3 gap-3">
        <div className="col-span-2">
          <TextField
            label="服务器地址"
            value={form.server}
            onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
              setForm({ ...form, server: event.target.value })
            }
            placeholder="residential-proxy.example.com"
            size="small"
          />
        </div>
        <TextField
          label="端口"
          type="number"
          value={String(form.port)}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setForm({
              ...form,
              port: Number.parseInt(event.target.value, 10) || 0,
            })
          }
          size="small"
        />
      </div>

      {showAuthFields && (
        <div className="grid grid-cols-2 gap-3">
          <TextField
            label="用户名（可选）"
            value={form.username ?? ''}
            onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
              setForm({
                ...form,
                username: event.target.value || undefined,
              })
            }
            size="small"
          />
          <TextField
            label="密码（可选）"
            type="password"
            value={form.password ?? ''}
            onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
              setForm({
                ...form,
                password: event.target.value || undefined,
              })
            }
            size="small"
          />
        </div>
      )}

      {showSsFields && (
        <TextField
          label="加密方式 (Cipher)"
          value={form.cipher ?? ''}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setForm({
              ...form,
              cipher: event.target.value || undefined,
            })
          }
          placeholder="aes-256-gcm"
          size="small"
        />
      )}

      {showVmessFields && (
        <TextField
          label="UUID"
          value={form.uuid ?? ''}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setForm({
              ...form,
              uuid: event.target.value || undefined,
            })
          }
          placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
          size="small"
        />
      )}

      {showTrojanFields && (
        <TextField
          label="Trojan 密码"
          type="password"
          value={form.trojanPassword ?? ''}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setForm({
              ...form,
              trojanPassword: event.target.value || undefined,
            })
          }
          size="small"
        />
      )}

      <div className="grid grid-cols-3 items-center gap-3">
        <div className="flex items-center gap-2">
          <Switch
            checked={form.tls ?? false}
            onCheckedChange={(value: boolean) =>
              setForm({ ...form, tls: value || undefined })
            }
          />
          <span className="text-sm">TLS</span>
        </div>
        <TextField
          label="SNI（可选）"
          value={form.sni ?? ''}
          onChange={(event: React.ChangeEvent<HTMLInputElement>) =>
            setForm({ ...form, sni: event.target.value || undefined })
          }
          size="small"
        />
        <div className="flex items-center gap-2">
          <Switch
            checked={form.skipCertVerify ?? false}
            onCheckedChange={(value: boolean) =>
              setForm({
                ...form,
                skipCertVerify: value || undefined,
              })
            }
          />
          <span className="text-sm">跳过证书校验</span>
        </div>
      </div>

      <Select
        label="地区标签"
        value={form.region ?? ''}
        onChange={(value: SelectPrimitiveValue) =>
          setForm({
            ...form,
            region: (String(value) || undefined) as string | undefined,
          })
        }
        options={REGION_OPTIONS}
        size="small"
      />

      <DialogActions>
        <Button variant="outlined" onClick={onCancel}>
          取消
        </Button>
        <Button variant="contained" onClick={() => onSave(form)} disabled={!isValid}>
          保存
        </Button>
      </DialogActions>
    </div>
  )
}
