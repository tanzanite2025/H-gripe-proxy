import {
  Box,
  Button,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  Switch,
  Tab,
  Tabs,
} from '@mui/material'
import { useEffect, useState } from 'react'

import {
  getObfuscationManager,
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
    manager.updateConfig({
      enabled,
      level,
      autoAdjust,
    })

    if (onApply) {
      const clashConfig = manager.generateClashConfig()
      onApply(clashConfig)
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
        <Box sx={{ mb: 2 }}>
          <FormControlLabel
            control={
              <Switch
                checked={enabled}
                onChange={(e) => setEnabled(e.target.checked)}
              />
            }
            label="启用混淆"
          />

          <FormControlLabel
            control={
              <Switch
                checked={autoAdjust}
                onChange={(e) => setAutoAdjust(e.target.checked)}
                disabled={!enabled}
              />
            }
            label="自动调整（根据网络环境）"
            sx={{ ml: 2 }}
          />
        </Box>

        {enabled && (
          <>
            <Tabs value={activeTab} onChange={(_, v) => setActiveTab(v)}>
              <Tab label="选择级别" />
              <Tab label="策略详情" />
              <Tab label="统计信息" />
            </Tabs>

            <Box sx={{ mt: 2 }}>
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
            </Box>
          </>
        )}
      </DialogContent>

      <DialogActions>
        <Button onClick={onClose}>取消</Button>
        <Button onClick={handleApply} variant="contained">
          应用
        </Button>
      </DialogActions>
    </Dialog>
  )
}
