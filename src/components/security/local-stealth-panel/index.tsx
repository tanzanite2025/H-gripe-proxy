import { EyeOff, Shield } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Card } from '@/components/tailwind/Card'
import { Chip } from '@/components/tailwind/Chip'
import {
  allocateStealthPort,
  applyLocalStealth,
  getCurrentStealthPort,
  getLocalStealthConfig,
  restoreLocalStealth,
  updateLocalStealthConfig,
  type AntiDiscoveryConfig,
  type LocalStealthConfig,
  type PortStealthConfig,
  type ProcessStealthConfig,
  type StealthApplyResult,
} from '@/services/local-stealth'
import { showNotice } from '@/services/notice-service'

import {
  AntiDiscoverySection,
  PortStealthSection,
  ProcessStealthSection,
} from './config-sections'
import { ResultSummary } from './result-summary'

function getEnabledFeatureCount(config: LocalStealthConfig) {
  return [
    config.process_stealth.enabled,
    config.port_stealth.enabled,
    config.anti_discovery.enabled,
  ].filter(Boolean).length
}

export function LocalStealthPanel() {
  const [config, setConfig] = useState<LocalStealthConfig | null>(null)
  const [result, setResult] = useState<StealthApplyResult | null>(null)
  const [currentPort, setCurrentPort] = useState<number | null>(null)
  const [loading, setLoading] = useState(false)

  const loadData = async () => {
    try {
      const [nextConfig, nextPort] = await Promise.all([
        getLocalStealthConfig(),
        getCurrentStealthPort(),
      ])

      setConfig(nextConfig)
      setCurrentPort(nextPort)
    } catch (error) {
      console.error('Failed to load local stealth config:', error)
      showNotice.error('加载本地隐匿配置失败。', error)
    }
  }

  useEffect(() => {
    void loadData()
  }, [])

  const persistConfig = async (nextConfig: LocalStealthConfig) => {
    try {
      await updateLocalStealthConfig(nextConfig)
      setConfig(nextConfig)
    } catch (error) {
      console.error('Failed to update local stealth config:', error)
      showNotice.error('更新本地隐匿配置失败。', error)
    }
  }

  const updateProcessStealth = async (
    updates: Partial<ProcessStealthConfig>,
  ) => {
    if (!config) return

    await persistConfig({
      ...config,
      process_stealth: { ...config.process_stealth, ...updates },
    })
  }

  const updatePortStealth = async (updates: Partial<PortStealthConfig>) => {
    if (!config) return

    await persistConfig({
      ...config,
      port_stealth: { ...config.port_stealth, ...updates },
    })
  }

  const updateAntiDiscovery = async (
    updates: Partial<AntiDiscoveryConfig>,
  ) => {
    if (!config) return

    await persistConfig({
      ...config,
      anti_discovery: { ...config.anti_discovery, ...updates },
    })
  }

  const handleApply = async () => {
    setLoading(true)

    try {
      const nextResult = await applyLocalStealth()
      setResult(nextResult)
      setCurrentPort(nextResult.allocated_port ?? currentPort)

      if (nextResult.errors.length === 0) {
        showNotice.success('本地隐匿策略已全部应用。')
      } else {
        showNotice.info('部分本地隐匿策略应用失败。', nextResult.errors.join('; '))
      }
    } catch (error) {
      console.error('Failed to apply local stealth:', error)
      showNotice.error('应用本地隐匿策略失败。', error)
    } finally {
      setLoading(false)
    }
  }

  const handleRestore = async () => {
    setLoading(true)

    try {
      await restoreLocalStealth()
      setResult(null)
      setCurrentPort(null)
      showNotice.success('本地隐匿策略已恢复。')
    } catch (error) {
      console.error('Failed to restore local stealth:', error)
      showNotice.error('恢复本地隐匿策略失败。', error)
    } finally {
      setLoading(false)
    }
  }

  const handleAllocatePort = async () => {
    setLoading(true)

    try {
      const port = await allocateStealthPort()
      setCurrentPort(port)
      showNotice.success(`已分配隐匿端口 ${port}。`)
    } catch (error) {
      console.error('Failed to allocate stealth port:', error)
      showNotice.error('分配隐匿端口失败。', error)
    } finally {
      setLoading(false)
    }
  }

  if (!config) {
    return (
      <Card className="p-6">
        <div className="text-sm text-text-secondary">加载中...</div>
      </Card>
    )
  }

  const enabledFeatureCount = getEnabledFeatureCount(config)
  const anyEnabled = enabledFeatureCount > 0

  return (
    <Card className="p-6">
      <div className="flex items-start gap-3">
        <div className="rounded-full bg-primary/10 p-2 text-primary">
          <EyeOff className="h-5 w-5" />
        </div>

        <div className="flex-1">
          <h3 className="text-lg font-semibold text-text-primary">
            本地隐匿增强
          </h3>
          <p className="mt-1 text-sm text-text-secondary">
            围绕进程展示、监听端口和局域网发现信号做最小暴露控制，减少本机环境里的特征泄露。
          </p>
          <div className="mt-3 flex flex-wrap gap-2">
            <Chip
              size="small"
              color={anyEnabled ? 'success' : 'default'}
              label={`已启用 ${enabledFeatureCount}/3`}
            />
            <Chip
              size="small"
              color={currentPort ? 'info' : 'default'}
              label={currentPort ? `当前隐匿端口 ${currentPort}` : '未分配隐匿端口'}
            />
          </div>
        </div>

        <div className="flex flex-wrap gap-2">
          {anyEnabled ? (
            <Button
              variant="outlined"
              color="error"
              size="small"
              onClick={handleRestore}
              disabled={loading}
            >
              恢复
            </Button>
          ) : null}

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

      <div className="mt-6 space-y-4">
        <Alert severity="info" className="text-sm">
          这些设置主要影响本机暴露面，不会替代出口侧的节点隐匿能力。更稳的做法是把本地隐匿和出口 TLS 指纹、反主动探测一起使用。
        </Alert>

        {result ? <ResultSummary result={result} /> : null}

        <div className="grid grid-cols-1 gap-4 xl:grid-cols-3">
          <ProcessStealthSection
            config={config.process_stealth}
            loading={loading}
            onChange={(updates) => void updateProcessStealth(updates)}
          />

          <PortStealthSection
            config={config.port_stealth}
            currentPort={currentPort}
            loading={loading}
            onChange={(updates) => void updatePortStealth(updates)}
            onAllocatePort={() => void handleAllocatePort()}
          />

          <AntiDiscoverySection
            config={config.anti_discovery}
            loading={loading}
            onChange={(updates) => void updateAntiDiscovery(updates)}
          />
        </div>
      </div>
    </Card>
  )
}
