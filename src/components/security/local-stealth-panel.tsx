/**
 * 本地隐蔽增强面板
 *
 * 功能：
 * 1. 进程隐蔽 - 伪装进程标题
 * 2. 端口隐蔽 - 端口随机化
 * 3. 防本地发现 - 禁用 mDNS/UPnP/LLMNR/NetBIOS/SSDP
 */

import {
  Eye,
  EyeOff,
  Network,
  RefreshCw,
  Shield,
  ShieldCheck,
  ShieldX,
} from 'lucide-react'
import { useState, useEffect, type ChangeEvent } from 'react'

import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import { FormControlLabel } from '@/components/tailwind/FormControlLabel'
import { Switch } from '@/components/tailwind/Switch'
import { TextField } from '@/components/tailwind/TextField'
import {
  getLocalStealthConfig,
  updateLocalStealthConfig,
  applyLocalStealth,
  restoreLocalStealth,
  allocateStealthPort,
  type LocalStealthConfig,
  type ProcessStealthConfig,
  type PortStealthConfig,
  type AntiDiscoveryConfig,
  type StealthApplyResult,
} from '@/services/local-stealth'
import { showNotice } from '@/services/notice-service'

export function LocalStealthPanel() {
  const [config, setConfig] = useState<LocalStealthConfig | null>(null)
  const [result, setResult] = useState<StealthApplyResult | null>(null)
  const [loading, setLoading] = useState(false)

  const loadData = async () => {
    try {
      const cfg = await getLocalStealthConfig()
      setConfig(cfg)
    } catch (error) {
      console.error('Failed to load local stealth config:', error)
      showNotice.error('加载本地隐蔽配置失败')
    }
  }

  useEffect(() => {
    loadData()
  }, [])

  const handleApply = async () => {
    setLoading(true)
    try {
      const r = await applyLocalStealth()
      setResult(r)
      if (r.errors.length === 0) {
        showNotice.success('本地隐蔽策略已全部应用')
      } else {
        showNotice.info(`部分策略应用失败: ${r.errors.join('; ')}`)
      }
    } catch (error) {
      console.error('Failed to apply stealth:', error)
      showNotice.error('应用隐蔽策略失败')
    } finally {
      setLoading(false)
    }
  }

  const handleRestore = async () => {
    setLoading(true)
    try {
      await restoreLocalStealth()
      setResult(null)
      showNotice.success('本地隐蔽策略已恢复')
    } catch (error) {
      console.error('Failed to restore stealth:', error)
      showNotice.error('恢复隐蔽策略失败')
    } finally {
      setLoading(false)
    }
  }

  const handleAllocatePort = async () => {
    setLoading(true)
    try {
      const port = await allocateStealthPort()
      showNotice.success(`已分配隐蔽端口: ${port}`)
    } catch (error) {
      console.error('Failed to allocate port:', error)
      showNotice.error('分配隐蔽端口失败')
    } finally {
      setLoading(false)
    }
  }

  const updateProcessStealth = async (updates: Partial<ProcessStealthConfig>) => {
    if (!config) return
    const newConfig: LocalStealthConfig = {
      ...config,
      process_stealth: { ...config.process_stealth, ...updates },
    }
    try {
      await updateLocalStealthConfig(newConfig)
      setConfig(newConfig)
    } catch (error) {
      console.error('Failed to update config:', error)
      showNotice.error('更新配置失败')
    }
  }

  const updatePortStealth = async (updates: Partial<PortStealthConfig>) => {
    if (!config) return
    const newConfig: LocalStealthConfig = {
      ...config,
      port_stealth: { ...config.port_stealth, ...updates },
    }
    try {
      await updateLocalStealthConfig(newConfig)
      setConfig(newConfig)
    } catch (error) {
      console.error('Failed to update config:', error)
      showNotice.error('更新配置失败')
    }
  }

  const updateAntiDiscovery = async (updates: Partial<AntiDiscoveryConfig>) => {
    if (!config) return
    const newConfig: LocalStealthConfig = {
      ...config,
      anti_discovery: { ...config.anti_discovery, ...updates },
    }
    try {
      await updateLocalStealthConfig(newConfig)
      setConfig(newConfig)
    } catch (error) {
      console.error('Failed to update config:', error)
      showNotice.error('更新配置失败')
    }
  }

  if (!config) {
    return (
      <Card className="p-6">
        <div className="text-sm text-text-secondary">加载中...</div>
      </Card>
    )
  }

  const anyEnabled =
    config.process_stealth.enabled ||
    config.port_stealth.enabled ||
    config.anti_discovery.enabled

  return (
    <Card className="p-6">
      <div className="flex items-start gap-3">
        <div className="rounded-full bg-purple-500/10 p-2 text-purple-500">
          <EyeOff className="h-5 w-5" />
        </div>
        <div className="flex-1">
          <h3 className="text-lg font-semibold text-text-primary">本地隐蔽增强</h3>
          <p className="text-sm text-text-secondary">
            进程伪装 · 端口随机化 · 防服务发现
          </p>
        </div>
        <div className="flex gap-2">
          {anyEnabled && (
            <Button
              variant="outlined"
              color="error"
              size="small"
              onClick={handleRestore}
              disabled={loading}
            >
              恢复
            </Button>
          )}
          <Button
            variant="contained"
            color="primary"
            size="small"
            startIcon={<Shield className="h-4 w-4" />}
            onClick={handleApply}
            disabled={loading || !anyEnabled}
          >
            应用策略
          </Button>
        </div>
      </div>

      <div className="mt-6 space-y-6">
        {/* 应用结果 */}
        {result && (
          <div className="rounded-xl bg-paper p-3 space-y-2">
            <div className="flex flex-wrap gap-2">
              <Chip
                icon={result.process_stealth_applied ? <ShieldCheck className="h-3.5 w-3.5" /> : <ShieldX className="h-3.5 w-3.5" />}
                label="进程隐蔽"
                color={result.process_stealth_applied ? 'success' : 'error'}
                size="small"
              />
              <Chip
                icon={result.port_stealth_applied ? <ShieldCheck className="h-3.5 w-3.5" /> : <ShieldX className="h-3.5 w-3.5" />}
                label={result.allocated_port ? `端口 ${result.allocated_port}` : '端口隐蔽'}
                color={result.port_stealth_applied ? 'success' : 'error'}
                size="small"
              />
              <Chip
                icon={result.anti_discovery_applied ? <ShieldCheck className="h-3.5 w-3.5" /> : <ShieldX className="h-3.5 w-3.5" />}
                label="防发现"
                color={result.anti_discovery_applied ? 'success' : 'error'}
                size="small"
              />
            </div>
            {result.discovery_messages.length > 0 && (
              <div className="text-xs text-text-secondary">
                {result.discovery_messages.join(' · ')}
              </div>
            )}
            {result.errors.length > 0 && (
              <div className="text-xs text-red-500">
                {result.errors.join('; ')}
              </div>
            )}
          </div>
        )}

        {/* ── 进程隐蔽 ── */}
        <div>
          <div className="flex items-center gap-2 mb-3">
            <Eye className="h-4 w-4 text-purple-500" />
            <span className="text-sm font-semibold text-text-primary">进程隐蔽</span>
          </div>
          <div className="space-y-2 pl-6">
            <FormControlLabel
              control={
                <Switch
                  checked={config.process_stealth.enabled}
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    updateProcessStealth({ enabled: e.target.checked })
                  }
                />
              }
              label="启用进程隐蔽"
            />
            {config.process_stealth.enabled && (
              <TextField
                label="伪装标题"
                value={config.process_stealth.disguise_title}
                onChange={(e: ChangeEvent<HTMLInputElement>) =>
                  updateProcessStealth({ disguise_title: e.target.value })
                }
                size="small"
                className="w-full"
                helperText="设置控制台窗口标题，降低进程特征识别"
              />
            )}
          </div>
        </div>

        {/* ── 端口隐蔽 ── */}
        <div>
          <div className="flex items-center gap-2 mb-3">
            <Network className="h-4 w-4 text-cyan-500" />
            <span className="text-sm font-semibold text-text-primary">端口隐蔽</span>
          </div>
          <div className="space-y-2 pl-6">
            <FormControlLabel
              control={
                <Switch
                  checked={config.port_stealth.enabled}
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    updatePortStealth({ enabled: e.target.checked })
                  }
                />
              }
              label="启用端口随机化"
            />
            {config.port_stealth.enabled && (
              <>
                <div className="flex items-center gap-3">
                  <TextField
                    label="起始端口"
                    type="number"
                    value={config.port_stealth.port_range[0]}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      updatePortStealth({
                        port_range: [Number(e.target.value), config.port_stealth.port_range[1]],
                      })
                    }
                    size="small"
                    className="w-[120px]"
                  />
                  <span className="text-text-secondary">—</span>
                  <TextField
                    label="结束端口"
                    type="number"
                    value={config.port_stealth.port_range[1]}
                    onChange={(e: ChangeEvent<HTMLInputElement>) =>
                      updatePortStealth({
                        port_range: [config.port_stealth.port_range[0], Number(e.target.value)],
                      })
                    }
                    size="small"
                    className="w-[120px]"
                  />
                  <Button
                    variant="outlined"
                    size="small"
                    startIcon={<RefreshCw className="h-3.5 w-3.5" />}
                    onClick={handleAllocatePort}
                    disabled={loading}
                  >
                    分配端口
                  </Button>
                </div>
                <div className="text-xs text-text-secondary">
                  避免使用常见代理端口 (7890, 1080, 9090 等)，在指定范围内随机分配
                </div>
              </>
            )}
          </div>
        </div>

        {/* ── 防本地发现 ── */}
        <div>
          <div className="flex items-center gap-2 mb-3">
            <Shield className="h-4 w-4 text-amber-500" />
            <span className="text-sm font-semibold text-text-primary">防本地发现</span>
          </div>
          <div className="space-y-2 pl-6">
            <FormControlLabel
              control={
                <Switch
                  checked={config.anti_discovery.enabled}
                  onChange={(e: ChangeEvent<HTMLInputElement>) =>
                    updateAntiDiscovery({ enabled: e.target.checked })
                  }
                />
              }
              label="启用防本地发现"
            />
            {config.anti_discovery.enabled && (
              <div className="space-y-1">
                <FormControlLabel
                  control={
                    <Switch
                      checked={config.anti_discovery.disable_mdns}
                      onChange={(e: ChangeEvent<HTMLInputElement>) =>
                        updateAntiDiscovery({ disable_mdns: e.target.checked })
                      }
                      size="small"
                    />
                  }
                  label="禁用 mDNS (端口 5353)"
                />
                <FormControlLabel
                  control={
                    <Switch
                      checked={config.anti_discovery.disable_upnp}
                      onChange={(e: ChangeEvent<HTMLInputElement>) =>
                        updateAntiDiscovery({ disable_upnp: e.target.checked })
                      }
                      size="small"
                    />
                  }
                  label="禁用 UPnP (端口 1900)"
                />
                <FormControlLabel
                  control={
                    <Switch
                      checked={config.anti_discovery.disable_llmnr}
                      onChange={(e: ChangeEvent<HTMLInputElement>) =>
                        updateAntiDiscovery({ disable_llmnr: e.target.checked })
                      }
                      size="small"
                    />
                  }
                  label="禁用 LLMNR (端口 5355)"
                />
                <FormControlLabel
                  control={
                    <Switch
                      checked={config.anti_discovery.disable_netbios}
                      onChange={(e: ChangeEvent<HTMLInputElement>) =>
                        updateAntiDiscovery({ disable_netbios: e.target.checked })
                      }
                      size="small"
                    />
                  }
                  label="禁用 NetBIOS (端口 137-139)"
                />
                <FormControlLabel
                  control={
                    <Switch
                      checked={config.anti_discovery.disable_ssdp}
                      onChange={(e: ChangeEvent<HTMLInputElement>) =>
                        updateAntiDiscovery({ disable_ssdp: e.target.checked })
                      }
                      size="small"
                    />
                  }
                  label="禁用 SSDP (UPnP 发现)"
                />
              </div>
            )}
          </div>
        </div>
      </div>
    </Card>
  )
}
