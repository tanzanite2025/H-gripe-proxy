import {
  Scale as BalanceIcon,
  Settings as SettingsIcon,
  Shield as SecurityIcon,
  Zap as SpeedIcon,
} from 'lucide-react'

import { Alert } from '@/components/tailwind/Alert'
import { Chip } from '@/components/tailwind/Chip'
import {
  ToggleButton,
  ToggleButtonGroup,
} from '@/components/tailwind/ToggleButtonGroup'
import type { DnsRuntimeStatus } from '@/services/cmds'
import type { DnsRoutingMode } from '@/services/coordinator'

import { buildDnsRuntimeViewModel } from './dns-runtime-view-model'

interface Props {
  mode: DnsRoutingMode
  runtimeStatus?: DnsRuntimeStatus
  onChange: (mode: DnsRoutingMode) => void
}

const MODE_DESCRIPTIONS: Record<DnsRoutingMode, string> = {
  speed: '全部优先使用低延迟 DNS，适合更看重解析速度的场景。',
  privacy: '全部优先使用加密 DNS，隐私更强，但延迟通常更高。',
  balanced:
    '国内域名走低延迟 DNS，海外域名走加密 DNS，在速度和隐私之间折中。',
  custom: '保留给手动接管的自定义 DNS 路由策略。',
}

export const DnsRoutingCard = ({ mode, runtimeStatus, onChange }: Props) => {
  const handleModeChange = (
    _event: React.MouseEvent<HTMLElement>,
    value: string | string[],
  ) => {
    if (typeof value === 'string') {
      onChange(value as DnsRoutingMode)
    }
  }

  const runtimeView = runtimeStatus
    ? buildDnsRuntimeViewModel(runtimeStatus)
    : null

  return (
    <div>
      <h6 className="mb-2 text-lg font-bold">DNS 智能分流</h6>

      <Alert severity="info" className="mb-2">
        智能分流会根据域名类别自动选择更合适的 DNS 路径，在保证解析速度的同时尽量减少不必要的绕路。
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
          {MODE_DESCRIPTIONS[mode]}
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
              海外域名 DNS
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
          {mode === 'custom' && (
            <Chip label="按自定义策略执行" size="small" color="info" />
          )}
        </div>
      </div>
    </div>
  )
}
