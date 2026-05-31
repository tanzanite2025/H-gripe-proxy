import { useEffect, useState } from 'react'

import { Button } from '@/components/tailwind/Button'
import {
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind/Dialog'
import { Switch } from '@/components/tailwind/Switch'
import { Tab, Tabs } from '@/components/tailwind/Tabs'
import {
  getObfuscationManager,
  syncObfuscationFromSecurityConfig,
  getObfuscationStrategy,
  type ObfuscationLevel,
} from '@/services/obfuscation'

import { ObfuscationLevelSelector } from './obfuscation-level-selector'
import { ObfuscationStats } from './obfuscation-stats'
import { ObfuscationStrategyConfig } from './obfuscation-strategy-config'

interface ObfuscationConfigProps {
  open: boolean
  onClose: () => void
  onApply?: (config: any) => void
}

export function ObfuscationConfig({
  open,
  onClose,
  onApply,
}: ObfuscationConfigProps) {
  const [activeTab, setActiveTab] = useState(0)
  const [enabled, setEnabled] = useState(false)
  const [level, setLevel] = useState<ObfuscationLevel>('medium')
  const [autoAdjust, setAutoAdjust] = useState(false)

  // 加载配置
  useEffect(() => {
    const manager = getObfuscationManager()
    const config = manager.getConfig()
    setEnabled(config.enabled)
    setLevel(config.level)
    setAutoAdjust(config.autoAdjust)
  }, [open])

  const handleApply = () => {
    const manager = getObfuscationManager()
    // Sync to in-memory manager for stats display
    syncObfuscationFromSecurityConfig({
      enabled,
      level,
      autoAdjust,
    })

    if (onApply) {
      // Config is now managed by Rust backend via SecurityConfig,
      // no longer generating Clash config in frontend
      onApply({ enabled, level, autoAdjust })
    }

    onClose()
  }

  const strategy = getObfuscationStrategy(level)
  const manager = getObfuscationManager()
  const stats = manager.getStats()

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>混沌动态混淆配置</DialogTitle>

      <DialogContent>
        <div className="mb-4">
          <label className="flex items-center gap-2">
            <Switch
              checked={enabled}
              onCheckedChange={setEnabled}
            />
            <span>启用混淆</span>
          </label>

          <label className="flex items-center gap-2 ml-4 mt-2">
            <Switch
              checked={autoAdjust}
              onCheckedChange={setAutoAdjust}
              disabled={!enabled}
            />
            <span>自动调整（根据网络环境）</span>
          </label>
        </div>

        {enabled && (
          <>
            <Tabs value={activeTab} onChange={(_, value) => setActiveTab(Number(value))}>
              <Tab label="选择级别" value={0} />
              <Tab label="策略详情" value={1} />
              <Tab label="统计信息" value={2} />
            </Tabs>

            <div className="mt-4">
              {activeTab === 0 && (
                <ObfuscationLevelSelector
                  currentLevel={level}
                  onChange={setLevel}
                />
              )}

              {activeTab === 1 && (
                <ObfuscationStrategyConfig strategy={strategy} />
              )}

              {activeTab === 2 && (
                <ObfuscationStats
                  enabled={enabled}
                  stats={{
                    level: stats.strategy.level,
                    avgPaddingSize: stats.traffic.avgPaddingSize,
                    avgTimingJitter: stats.traffic.avgTimingJitter,
                    packetSizeVariation: stats.traffic.packetSizeVariation,
                    httpHeaderObfuscation:
                      stats.protocol.httpHeaderObfuscation,
                    tlsFingerprintRandomization:
                      stats.protocol.tlsFingerprintRandomization,
                  }}
                />
              )}
            </div>
          </>
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose} variant="outlined">
          取消
        </Button>
        <Button onClick={handleApply} variant="primary">
          应用
        </Button>
      </DialogActions>
    </Dialog>
  )
}
