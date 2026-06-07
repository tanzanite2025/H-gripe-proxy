import { AlertCircle as ErrorIcon, CheckCircle as CheckIcon } from 'lucide-react'

import { Alert } from '@/components/tailwind/Alert'
import { Button } from '@/components/tailwind/Button'
import { Chip } from '@/components/tailwind/Chip'
import type { TorRuntimeStatus } from '@/services/cmds'

import type { ReturnTypeBuildTorRuntimeViewModel } from './types'

interface RuntimeStatusPanelProps {
  runtimeView: ReturnTypeBuildTorRuntimeViewModel
  status?: TorRuntimeStatus
  checking: boolean
  disabled: boolean
  onCheckConnection: () => void
}

export function RuntimeStatusPanel({
  runtimeView,
  status,
  checking,
  disabled,
  onCheckConnection,
}: RuntimeStatusPanelProps) {
  return (
    <section className="space-y-2">
      <div className="text-sm text-gray-500 dark:text-gray-400">连接状态</div>

      <div className="flex flex-wrap gap-2">
        {runtimeView.enabled.active ? (
          <Chip
            icon={<CheckIcon className="h-3 w-3" />}
            label={runtimeView.enabled.label}
            color={runtimeView.enabled.color}
            size="small"
          />
        ) : (
          <Chip
            icon={<ErrorIcon className="h-3 w-3" />}
            label={runtimeView.enabled.label}
            size="small"
          />
        )}

        {runtimeView.connection.connected ? (
          <Chip
            icon={<CheckIcon className="h-3 w-3" />}
            label={runtimeView.connection.label}
            color={runtimeView.connection.color}
            size="small"
          />
        ) : runtimeView.connection.color === 'info' ? (
          <Chip
            label={runtimeView.connection.label}
            color={runtimeView.connection.color}
            size="small"
          />
        ) : (
          <Chip
            icon={<ErrorIcon className="h-3 w-3" />}
            label={runtimeView.connection.label}
            color={runtimeView.connection.color}
            size="small"
          />
        )}

        {runtimeView.assessment ? (
          <Chip
            label={runtimeView.assessment.label}
            color={runtimeView.assessment.color}
            size="small"
          />
        ) : null}

        {runtimeView.confidence ? (
          <Chip
            label={runtimeView.confidence.label}
            color={runtimeView.confidence.color}
            size="small"
          />
        ) : null}

        {status ? (
          <Chip
            label={status.circuit_established ? '电路已建立' : '电路未建立'}
            color={status.circuit_established ? 'success' : 'warning'}
            size="small"
          />
        ) : null}
      </div>

      <div className="space-y-1 text-sm">
        <div className="flex items-center gap-1">
          <div>代理地址:</div>
          <span className="uds-mono">
            {status?.configured_proxy_url ?? '未配置'}
          </span>
        </div>

        {status?.current_ip ? (
          <div className="flex items-center gap-1">
            <div>出口 IP:</div>
            <span className="uds-mono">{status.current_ip}</span>
          </div>
        ) : null}

        {status?.exit_node ? (
          <div className="flex items-center gap-1">
            <div>出口节点:</div>
            <span>{status.exit_node}</span>
          </div>
        ) : null}

        {status?.observation_path ? (
          <div className="flex items-center gap-1">
            <div>观测路径:</div>
            <span className="uds-mono">{status.observation_path}</span>
          </div>
        ) : null}

        {status?.observation_source ? (
          <div className="flex items-center gap-1">
            <div>观测来源:</div>
            <span className="uds-mono">{status.observation_source}</span>
          </div>
        ) : null}

        {status?.check_method ? (
          <div className="flex items-center gap-1">
            <div>检测方式:</div>
            <span className="uds-mono">{status.check_method}</span>
          </div>
        ) : null}
      </div>

      {status?.runtime_risk_detected && status.runtime_risk_type.length > 0 ? (
        <Alert severity="warning" className="text-xs">
          {runtimeView.runtimeRiskText}
        </Alert>
      ) : null}

      {status?.observation_incomplete ? (
        <Alert severity="info" className="text-xs">
          当前 Tor 观测并不完整，结果只代表已经完成的 SOCKS5H 出口检测。
        </Alert>
      ) : null}

      {status?.warnings.length ? (
        <Alert severity="warning" className="text-xs">
          {status.warnings.join('；')}
        </Alert>
      ) : null}

      {status?.error ? (
        <Alert severity="error" className="text-xs">
          {status.error}
        </Alert>
      ) : null}

      <Button
        variant="outlined"
        size="small"
        onClick={onCheckConnection}
        fullWidth
        loading={checking}
        disabled={disabled}
      >
        检查连接
      </Button>
    </section>
  )
}
