/**
 * DNS 智能分流配置卡片
 */

import { Balance as BalanceIcon, Settings as SettingsIcon, Shield as SecurityIcon, Zap as SpeedIcon } from 'lucide-react'
import { useEffect, useState } from 'react'

import { Alert } from '@/components/tailwind/Alert'
import { Chip } from '@/components/tailwind/Chip'
import { ToggleButton, ToggleButtonGroup } from '@/components/tailwind/ToggleButtonGroup'
import { dnsSmartRoutingService, type DnsRoutingMode } from '@/services/dns-smart-routing'
import { cn } from '@/utils/cn'

export const DnsRoutingCard = () => {
  const [mode, setMode] = useState<DnsRoutingMode>('balanced')
  const [stats, setStats] = useState({
    mode: 'balanced' as DnsRoutingMode,
    domesticDns: '',
    foreignDns: '',
    customRulesCount: 0,
  })

  useEffect(() => {
    // 初始化
    const config = dnsSmartRoutingService.getConfig()
    setMode(config.mode)
    updateStats()

    // 定期更新统计
    const interval = setInterval(updateStats, 5000)
    return () => clearInterval(interval)
  }, [])

  const updateStats = () => {
    const newStats = dnsSmartRoutingService.getStats()
    setStats(newStats)
  }

  const handleModeChange = (_event: React.MouseEvent<HTMLElement>, newMode: DnsRoutingMode) => {
    if (newMode !== null) {
      setMode(newMode)
      dnsSmartRoutingService.setMode(newMode)
      updateStats()
    }
  }

  const getModeDescription = (mode: DnsRoutingMode): string => {
    switch (mode) {
      case 'speed':
        return '全部使用国内 UDP DNS，延迟最低（10-30ms）'
      case 'privacy':
        return '全部使用 Cloudflare DoH，隐私保护最强'
      case 'balanced':
        return '国内域名用 UDP，国外域名用 DoH，平衡速度和隐私'
      case 'custom':
        return '自定义 DNS 配置和规则'
    }
  }

  const getModeColor = (mode: DnsRoutingMode): 'success' | 'info' | 'warning' | 'default' => {
    switch (mode) {
      case 'speed':
        return 'success'
      case 'privacy':
        return 'info'
      case 'balanced':
        return 'warning'
      case 'custom':
        return 'default'
    }
  }

  return (
    <div>
      <h6 className="mb-2 text-lg font-bold">
        DNS 智能分流
      </h6>

      <Alert severity="info" className="mb-2">
        智能分流可根据域名类型自动选择最优 DNS 服务器，提升解析速度并保护隐私
      </Alert>

      <div className="mb-3">
        <div className="mb-1.5 text-sm text-gray-500 dark:text-gray-400">
          分流模式
        </div>
        <ToggleButtonGroup
          value={mode}
          exclusive
          onChange={handleModeChange}
          fullWidth
          className="mb-2"
        >
          <ToggleButton value="speed">
            <SpeedIcon className="mr-1 h-4 w-4" />
            速度优先
          </ToggleButton>
          <ToggleButton value="balanced">
            <BalanceIcon className="mr-1 h-4 w-4" />
            平衡模式
          </ToggleButton>
          <ToggleButton value="privacy">
            <SecurityIcon className="mr-1 h-4 w-4" />
            隐私优先
          </ToggleButton>
          <ToggleButton value="custom">
            <SettingsIcon className="mr-1 h-4 w-4" />
            自定义
          </ToggleButton>
        </ToggleButtonGroup>

        <div className="mb-2 text-sm text-gray-600 dark:text-gray-400">
          {getModeDescription(mode)}
        </div>
      </div>

      <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

      <div>
        <div className="mb-1.5 text-sm text-gray-500 dark:text-gray-400">
          当前配置
        </div>

        <div className="space-y-1.5">
          <div>
            <div className="text-xs text-gray-500 dark:text-gray-400">
              当前模式
            </div>
            <div className="mt-0.5">
              <Chip
                label={
                  mode === 'speed'
                    ? '速度优先'
                    : mode === 'privacy'
                      ? '隐私优先'
                      : mode === 'balanced'
                        ? '平衡模式'
                        : '自定义'
                }
                color={getModeColor(mode)}
                size="small"
              />
            </div>
          </div>

          <div>
            <div className="text-xs text-gray-500 dark:text-gray-400">
              国内域名 DNS
            </div>
            <div className="mt-0.5 text-sm">
              {stats.domesticDns || '未配置'}
            </div>
          </div>

          <div>
            <div className="text-xs text-gray-500 dark:text-gray-400">
              国外域名 DNS
            </div>
            <div className="mt-0.5 text-sm">
              {stats.foreignDns || '未配置'}
            </div>
          </div>

          {stats.customRulesCount > 0 && (
            <div>
              <div className="text-xs text-gray-500 dark:text-gray-400">
                自定义规则
              </div>
              <div className="mt-0.5 text-sm">
                {stats.customRulesCount} 条规则
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="my-2 border-t border-gray-200 dark:border-gray-700" />

      <div>
        <div className="text-xs text-gray-500 dark:text-gray-400">
          性能提示
        </div>
        <div className="mt-1 flex gap-1">
          {mode === 'speed' && (
            <>
              <Chip label="延迟: 10-30ms" size="small" color="success" />
              <Chip label="隐私: 低" size="small" />
            </>
          )}
          {mode === 'privacy' && (
            <>
              <Chip label="延迟: 30-80ms" size="small" />
              <Chip label="隐私: 高" size="small" color="success" />
            </>
          )}
          {mode === 'balanced' && (
            <>
              <Chip label="延迟: 20-40ms" size="small" color="success" />
              <Chip label="隐私: 中" size="small" color="warning" />
            </>
          )}
        </div>
      </div>
    </div>
  )
}
