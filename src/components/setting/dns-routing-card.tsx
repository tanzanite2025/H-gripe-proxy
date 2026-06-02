/**
 * DNS 智能分流配置卡片
 */

import { Scale as BalanceIcon, Settings as SettingsIcon, Shield as SecurityIcon, Zap as SpeedIcon } from 'lucide-react'

import { Alert } from '@/components/tailwind/Alert'
import { Chip } from '@/components/tailwind/Chip'
import { ToggleButton, ToggleButtonGroup } from '@/components/tailwind/ToggleButtonGroup'
import type { DnsRuntimeStatus } from '@/services/cmds'
import type { DnsRoutingMode } from '@/services/coordinator'
import { buildDnsRuntimeViewModel } from './dns-runtime-view-model'

interface Props {
  mode: DnsRoutingMode
  runtimeStatus?: DnsRuntimeStatus
  onChange: (mode: DnsRoutingMode) => void
}

export const DnsRoutingCard = ({ mode, runtimeStatus, onChange }: Props) => {
  const handleModeChange = (_event: React.MouseEvent<HTMLElement>, value: string | string[]) => {
    if (typeof value === 'string') {
      const newMode = value as DnsRoutingMode
      onChange(newMode)
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

  const runtimeView = runtimeStatus
    ? buildDnsRuntimeViewModel(runtimeStatus)
    : null

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
          后端确认的当前运行态
        </div>

        <div className="space-y-1.5">
          <div>
            <div className="text-xs text-gray-500 dark:text-gray-400">
              当前模式
            </div>
            <div className="mt-0.5">
              <Chip
                label={runtimeView?.routing.modeLabel ?? 'N/A'}
                color={runtimeView?.routing.modeColor ?? 'default'}
                size="small"
              />
            </div>
          </div>

          <div>
            <div className="text-xs text-gray-500 dark:text-gray-400">
              国内域名 DNS
            </div>
            <div className="mt-0.5 text-sm">
              {runtimeView?.routing.domesticDnsConfig ?? '未配置'}
            </div>
          </div>

          <div>
            <div className="text-xs text-gray-500 dark:text-gray-400">
              国外域名 DNS
            </div>
            <div className="mt-0.5 text-sm">
              {runtimeView?.routing.foreignDnsConfig ?? '未配置'}
            </div>
          </div>

          {runtimeView?.routing.policyCount ? (
            <div>
              <div className="text-xs text-gray-500 dark:text-gray-400">
                策略组数量
              </div>
              <div className="mt-0.5 text-sm">
                {runtimeView.routing.policyCountLabel}
              </div>
            </div>
          ) : null}
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
