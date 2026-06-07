import { Shield } from 'lucide-react'
import { useState } from 'react'

import { Switch } from '@/components/base'
import {
  Alert,
  Button,
  Chip,
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind'
import type { ResidentialProxy, ResidentialProxyPool } from '@/services/coordinator'
import {
  ipReputationVerifyResidentialProxy,
  type ResidentialProxyVerification,
} from '@/services/ip-reputation'

import { emptyProxy } from './constants'
import { ResidentialProxyEditForm } from './proxy-edit-form'
import { ResidentialProxyRow } from './proxy-row'

interface Props {
  config: ResidentialProxyPool
  onChange: (config: ResidentialProxyPool) => void
}

export function ResidentialPoolPanel({ config, onChange }: Props) {
  const [editingProxy, setEditingProxy] = useState<ResidentialProxy | null>(null)
  const [editIndex, setEditIndex] = useState(-1)
  const [isAdding, setIsAdding] = useState(false)
  const [verifyingName, setVerifyingName] = useState<string | null>(null)
  const [verificationByName, setVerificationByName] = useState<
    Record<string, ResidentialProxyVerification>
  >({})

  const enabledCount = config.proxies.filter((proxy) => proxy.enabled).length

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
    const nextProxies = [...config.proxies]
    nextProxies.splice(index, 1)
    onChange({ ...config, proxies: nextProxies })
  }

  const handleToggleProxy = (index: number, enabled: boolean) => {
    const nextProxies = [...config.proxies]
    nextProxies[index] = { ...nextProxies[index], enabled }
    onChange({ ...config, proxies: nextProxies })
  }

  const handleVerifyProxy = async (proxy: ResidentialProxy) => {
    setVerifyingName(proxy.name)
    try {
      const verification = await ipReputationVerifyResidentialProxy(proxy)
      setVerificationByName((previous) => ({
        ...previous,
        [proxy.name]: verification,
      }))
    } catch (error) {
      setVerificationByName((previous) => ({
        ...previous,
        [proxy.name]: {
          proxyName: proxy.name,
          status: 'failed',
          probeMethod: 'directProxy',
          message: String(error),
          checkedAt: Date.now(),
        },
      }))
    } finally {
      setVerifyingName(null)
    }
  }

  const handleSaveProxy = (proxy: ResidentialProxy) => {
    if (!proxy.name.trim() || !proxy.server.trim()) {
      return
    }

    if (isAdding) {
      onChange({ ...config, proxies: [...config.proxies, proxy] })
    } else {
      const nextProxies = [...config.proxies]
      nextProxies[editIndex] = proxy
      onChange({ ...config, proxies: nextProxies })
    }

    setEditingProxy(null)
  }

  const existingNames =
    isAdding || editIndex < 0
      ? config.proxies.map((proxy) => proxy.name)
      : config.proxies
          .filter((_, index) => index !== editIndex)
          .map((proxy) => proxy.name)

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="flex items-center gap-2 text-sm font-medium">
            <Shield className="h-4 w-4" />
            住宅代理池
          </h3>
          <p className="mt-1 text-xs text-gray-500">
            在这里维护住宅 / ISP 出口节点，供高风险域名策略和代理链出口复用。
          </p>
        </div>
        <Switch checked={config.enabled} onCheckedChange={handleToggle} />
      </div>

      {config.enabled && (
        <>
          <div className="flex items-center gap-3">
            <Chip
              label={`${enabledCount}/${config.proxies.length} 可用`}
              color={enabledCount > 0 ? 'success' : 'default'}
              size="small"
            />
            {enabledCount > 0 && (
              <span className="text-xs text-gray-500">
                启用后可在代理链里追加住宅出口，或供稳定出口画像策略使用。
              </span>
            )}
          </div>

          <Button variant="outlined" onClick={handleAdd} className="gap-1">
            添加住宅代理
          </Button>

          {config.proxies.length === 0 ? (
            <Alert severity="info" className="text-xs">
              {'暂无住宅代理节点。添加后可以把住宅出口接到代理链末端，也可以供稳定出口策略构建“前置节点 -> 住宅出口”的链路。'}
            </Alert>
          ) : (
            <div className="space-y-2">
              {config.proxies.map((proxy, index) => (
                <ResidentialProxyRow
                  key={proxy.name}
                  proxy={proxy}
                  verification={verificationByName[proxy.name]}
                  verifying={verifyingName === proxy.name}
                  onToggleEnabled={(enabled) => handleToggleProxy(index, enabled)}
                  onVerify={() => void handleVerifyProxy(proxy)}
                  onEdit={() => handleEdit(proxy, index)}
                  onDelete={() => handleDelete(index)}
                />
              ))}
            </div>
          )}

          <Alert severity="info" className="text-xs">
            <strong>工作原理:</strong>{' '}
            {
              '当出口画像策略需要住宅出口时，系统会把选中的前置节点与住宅出口组合起来，通过 dialer-proxy 形成“用户 -> 前置节点 -> 住宅出口 -> 目标站点”的链路，让最终出口更接近 ISP / Residential ASN 的特征。'
            }
          </Alert>
        </>
      )}

      <Dialog open={editingProxy !== null} onClose={() => setEditingProxy(null)}>
        <DialogTitle>{isAdding ? '添加住宅代理' : '编辑住宅代理'}</DialogTitle>
        <DialogContent>
          {editingProxy && (
            <ResidentialProxyEditForm
              proxy={editingProxy}
              existingNames={existingNames}
              isAdding={isAdding}
              onSave={handleSaveProxy}
              onCancel={() => setEditingProxy(null)}
            />
          )}
        </DialogContent>
      </Dialog>
    </div>
  )
}
