/**
 * 住宅代理池管理面板
 * 用户在此添加/编辑/删除住宅/ISP 代理节点
 */

import { Pencil, Plus, Trash2, Shield } from 'lucide-react'
import { useState } from 'react'

import { Switch } from '@/components/base'
import {
  Button,
  TextField,
  Dialog,
  DialogTitle,
  DialogContent,
  DialogActions,
  Alert,
  Chip,
} from '@/components/tailwind'
import { Select } from '@/components/tailwind/Select'
import type { SelectPrimitiveValue } from '@/components/tailwind/Select'
import type { ResidentialProxy, ResidentialProxyPool, ResidentialProxyType } from '@/services/coordinator'

interface Props {
  config: ResidentialProxyPool
  onChange: (config: ResidentialProxyPool) => void
}

const PROXY_TYPES: { value: ResidentialProxyType; label: string }[] = [
  { value: 'socks5', label: 'SOCKS5' },
  { value: 'http', label: 'HTTP' },
  { value: 'ss', label: 'Shadowsocks' },
  { value: 'vmess', label: 'VMess' },
  { value: 'trojan', label: 'Trojan' },
]

const REGION_OPTIONS = [
  { value: '', label: '未指定' },
  { value: 'US', label: '🇺🇸 美国' },
  { value: 'JP', label: '🇯🇵 日本' },
  { value: 'SG', label: '🇸🇬 新加坡' },
  { value: 'DE', label: '🇩🇪 德国' },
  { value: 'GB', label: '🇬🇧 英国' },
  { value: 'KR', label: '🇰🇷 韩国' },
  { value: 'HK', label: '🇭🇰 香港' },
  { value: 'TW', label: '🇹🇼 台湾' },
  { value: 'AU', label: '🇦🇺 澳大利亚' },
]

function emptyProxy(): ResidentialProxy {
  return {
    name: '',
    proxyType: 'socks5',
    server: '',
    port: 1080,
    enabled: true,
  }
}

export function ResidentialPoolPanel({ config, onChange }: Props) {
  const [editingProxy, setEditingProxy] = useState<ResidentialProxy | null>(null)
  const [editIndex, setEditIndex] = useState<number>(-1)
  const [isAdding, setIsAdding] = useState(false)

  const handleToggle = (checked: boolean) => {
    onChange({ ...config, enabled: checked })
  }

  const handleAdd = () => {
    setEditingProxy(emptyProxy())
    setEditIndex(-1)
    setIsAdding(true)
  }

  const handleEdit = (proxy: ResidentialProxy, index: number) => {
    setEditingProxy({ ...proxy })
    setEditIndex(index)
    setIsAdding(false)
  }

  const handleDelete = (index: number) => {
    const newProxies = [...config.proxies]
    newProxies.splice(index, 1)
    onChange({ ...config, proxies: newProxies })
  }

  const handleToggleProxy = (index: number, enabled: boolean) => {
    const newProxies = [...config.proxies]
    newProxies[index] = { ...newProxies[index], enabled }
    onChange({ ...config, proxies: newProxies })
  }

  const handleSaveProxy = (proxy: ResidentialProxy) => {
    if (!proxy.name || !proxy.server) return

    if (isAdding) {
      if (config.proxies.some((p) => p.name === proxy.name)) return
      onChange({ ...config, proxies: [...config.proxies, proxy] })
    } else {
      const newProxies = [...config.proxies]
      newProxies[editIndex] = proxy
      onChange({ ...config, proxies: newProxies })
    }
    setEditingProxy(null)
  }

  const enabledCount = config.proxies.filter((p) => p.enabled).length

  return (
    <div className="space-y-4">
      {/* 启用开关 */}
      <div className="flex items-center justify-between">
        <div>
          <h3 className="text-sm font-medium flex items-center gap-2">
            <Shield className="w-4 h-4" />
            住宅代理池
          </h3>
          <p className="text-xs text-gray-500 mt-1">
            添加住宅/ISP 代理节点，高风控域名自动构建链式路由
          </p>
        </div>
        <Switch checked={config.enabled} onCheckedChange={handleToggle} />
      </div>

      {config.enabled && (
        <>
          {/* 统计 */}
          <div className="flex items-center gap-3">
            <Chip
              label={`${enabledCount}/${config.proxies.length} 可用`}
              color={enabledCount > 0 ? 'success' : 'default'}
              size="small"
            />
            {enabledCount > 0 && (
              <span className="text-xs text-gray-500">
                链式路由已就绪：VPS → 住宅出口
              </span>
            )}
          </div>

          {/* 添加按钮 */}
          <Button variant="outlined" onClick={handleAdd} className="gap-1">
            <Plus className="w-4 h-4" />
            添加住宅代理
          </Button>

          {/* 代理列表 */}
          {config.proxies.length === 0 ? (
            <Alert severity="info" className="text-xs">
              暂无住宅代理节点。添加住宅/ISP 代理后，出口身份画像中启用"链式住宅路由"即可自动构建
              VPS→住宅 链式代理。
            </Alert>
          ) : (
            <div className="space-y-2">
              {config.proxies.map((proxy, index) => (
                <div
                  key={proxy.name}
                  className={`flex items-center justify-between p-3 rounded-lg border ${
                    proxy.enabled
                      ? 'border-green-200 dark:border-green-800 bg-green-50/50 dark:bg-green-900/10'
                      : 'border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/50'
                  }`}
                >
                  <div className="flex items-center gap-3 min-w-0">
                    <Switch
                      checked={proxy.enabled}
                      onCheckedChange={(v: boolean) => handleToggleProxy(index, v)}
                    />
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm font-medium truncate">{proxy.name}</span>
                        <Chip label={proxy.proxyType.toUpperCase()} color="default" size="small" />
                        {proxy.region && (
                          <Chip label={proxy.region} color="info" size="small" />
                        )}
                      </div>
                      <span className="text-xs text-gray-500">
                        {proxy.server}:{proxy.port}
                      </span>
                    </div>
                  </div>
                  <div className="flex items-center gap-1 shrink-0">
                    <button
                      onClick={() => handleEdit(proxy, index)}
                      className="p-1.5 rounded hover:bg-gray-200 dark:hover:bg-gray-700"
                    >
                      <Pencil className="w-3.5 h-3.5" />
                    </button>
                    <button
                      onClick={() => handleDelete(index)}
                      className="p-1.5 rounded hover:bg-red-100 dark:hover:bg-red-900/30 text-red-500"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* 说明 */}
          <Alert severity="info" className="text-xs">
            <strong>工作原理：</strong>当出口身份画像启用"链式住宅路由"时，enhance 引擎会自动为高风控域名的
            VERGE-STABLE-* 组中的前置节点注入 <code>dialer-proxy</code>，构建
            「用户 → VPS 前置节点 → 住宅出口 → 目标」链路，使出口 IP 呈现 ISP/Residential ASN 特征。
          </Alert>
        </>
      )}

      {/* 编辑对话框 */}
      <Dialog open={editingProxy !== null} onClose={() => setEditingProxy(null)}>
        <DialogTitle>{isAdding ? '添加住宅代理' : '编辑住宅代理'}</DialogTitle>
        <DialogContent>
          {editingProxy && (
            <ProxyEditForm
              proxy={editingProxy}
              onSave={handleSaveProxy}
              onCancel={() => setEditingProxy(null)}
              existingNames={config.proxies.map((p) => p.name)}
              isAdding={isAdding}
            />
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}

function ProxyEditForm({
  proxy,
  onSave,
  onCancel,
  existingNames,
  isAdding,
}: {
  proxy: ResidentialProxy
  onSave: (proxy: ResidentialProxy) => void
  onCancel: () => void
  existingNames: string[]
  isAdding: boolean
}) {
  const [form, setForm] = useState<ResidentialProxy>({ ...proxy })

  const isNameDuplicate = isAdding && existingNames.includes(form.name)
  const isValid = form.name.trim() !== '' && form.server.trim() !== '' && !isNameDuplicate

  const showAuthFields = form.proxyType === 'socks5' || form.proxyType === 'http'
  const showSsFields = form.proxyType === 'ss'
  const showVmessFields = form.proxyType === 'vmess'
  const showTrojanFields = form.proxyType === 'trojan'

  return (
    <div className="space-y-3 mt-3">
      {/* 基本信息 */}
      <div className="grid grid-cols-2 gap-3">
        <TextField
          label="名称"
          value={form.name}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) => setForm({ ...form, name: e.target.value })}
          error={isNameDuplicate}
          helperText={isNameDuplicate ? '名称已存在' : ''}
          placeholder="US-Residential-1"
          size="small"
        />
        <Select
          label="协议类型"
          value={form.proxyType}
          onChange={(value: SelectPrimitiveValue) =>
            setForm({ ...form, proxyType: String(value) as ResidentialProxyType })
          }
          options={PROXY_TYPES.map((t) => ({ value: t.value, label: t.label }))}
          size="small"
        />
      </div>

      <div className="grid grid-cols-3 gap-3">
        <div className="col-span-2">
          <TextField
            label="服务器地址"
            value={form.server}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) => setForm({ ...form, server: e.target.value })}
            placeholder="residential-proxy.example.com"
            size="small"
          />
        </div>
        <TextField
          label="端口"
          type="number"
          value={String(form.port)}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
            setForm({ ...form, port: parseInt(e.target.value) || 0 })
          }
          size="small"
        />
      </div>

      {/* SOCKS5/HTTP 认证 */}
      {showAuthFields && (
        <div className="grid grid-cols-2 gap-3">
          <TextField
            label="用户名（可选）"
            value={form.username ?? ''}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              setForm({ ...form, username: e.target.value || undefined })
            }
            size="small"
          />
          <TextField
            label="密码（可选）"
            type="password"
            value={form.password ?? ''}
            onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
              setForm({ ...form, password: e.target.value || undefined })
            }
            size="small"
          />
        </div>
      )}

      {/* SS 字段 */}
      {showSsFields && (
        <TextField
          label="加密方式 (Cipher)"
          value={form.cipher ?? ''}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
            setForm({ ...form, cipher: e.target.value || undefined })
          }
          placeholder="aes-256-gcm"
          size="small"
        />
      )}

      {/* VMess 字段 */}
      {showVmessFields && (
        <TextField
          label="UUID"
          value={form.uuid ?? ''}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
            setForm({ ...form, uuid: e.target.value || undefined })
          }
          placeholder="xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
          size="small"
        />
      )}

      {/* Trojan 字段 */}
      {showTrojanFields && (
        <TextField
          label="Trojan 密码"
          type="password"
          value={form.trojanPassword ?? ''}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
            setForm({ ...form, trojanPassword: e.target.value || undefined })
          }
          size="small"
        />
      )}

      {/* TLS / SNI */}
      <div className="grid grid-cols-3 gap-3 items-center">
        <div className="flex items-center gap-2">
          <Switch
            checked={form.tls ?? false}
            onCheckedChange={(v: boolean) => setForm({ ...form, tls: v || undefined })}
          />
          <span className="text-sm">TLS</span>
        </div>
        <TextField
          label="SNI（可选）"
          value={form.sni ?? ''}
          onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
            setForm({ ...form, sni: e.target.value || undefined })
          }
          size="small"
        />
        <div className="flex items-center gap-2">
          <Switch
            checked={form.skipCertVerify ?? false}
            onCheckedChange={(v: boolean) => setForm({ ...form, skipCertVerify: v || undefined })}
          />
          <span className="text-sm">跳过验证</span>
        </div>
      </div>

      {/* 地区 */}
      <Select
        label="地区标签"
        value={form.region ?? ''}
        onChange={(value: SelectPrimitiveValue) =>
          setForm({ ...form, region: (String(value) || undefined) as string | undefined })
        }
        options={REGION_OPTIONS}
        size="small"
      />

      {/* 按钮 */}
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
