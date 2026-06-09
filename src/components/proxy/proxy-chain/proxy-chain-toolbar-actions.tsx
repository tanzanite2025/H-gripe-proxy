import { Link, Link2Off, Settings, Trash2, X } from 'lucide-react'

import { Button } from '@/components/tailwind/Button'
import { IconButton } from '@/components/tailwind/IconButton'

import type { ProxyChainCopy } from './proxy-chain-copy'

interface ProxyChainToolbarActionsProps {
  chainLength: number
  isConnected: boolean
  isConnecting: boolean
  isConnectDisabled: boolean
  connectButtonTitle?: string
  copy: Pick<
    ProxyChainCopy,
    | 'clearChainLabel'
    | 'residentialPoolLabel'
    | 'connectingLabel'
    | 'disconnectLabel'
    | 'connectLabel'
  >
  onClearChain: () => void
  onOpenResidentialConfig: () => void
  onConnect: () => void | Promise<void>
  onClose?: () => void
}

const getConnectButtonLabel = (
  isConnecting: boolean,
  isConnected: boolean,
  copy: Pick<
    ProxyChainCopy,
    'connectingLabel' | 'disconnectLabel' | 'connectLabel'
  >,
) => {
  if (isConnecting) return copy.connectingLabel
  if (isConnected) return copy.disconnectLabel
  return copy.connectLabel
}

export const ProxyChainToolbarActions = ({
  chainLength,
  isConnected,
  isConnecting,
  isConnectDisabled,
  connectButtonTitle,
  copy,
  onClearChain,
  onOpenResidentialConfig,
  onConnect,
  onClose,
}: ProxyChainToolbarActionsProps) => {
  return (
    <div className="flex items-center gap-2">
      {chainLength > 0 && (
        <IconButton
          size="small"
          onClick={onClearChain}
          className="text-red-500 hover:bg-red-500/10"
          title={copy.clearChainLabel}
        >
          <Trash2 className="h-4 w-4" />
        </IconButton>
      )}

      <IconButton
        size="small"
        onClick={onOpenResidentialConfig}
        title={copy.residentialPoolLabel}
      >
        <Settings className="h-4 w-4" />
      </IconButton>

      <Button
        size="small"
        variant={isConnected ? 'outlined' : 'primary'}
        startIcon={
          isConnected ? (
            <Link2Off className="h-4 w-4" />
          ) : (
            <Link className="h-4 w-4" />
          )
        }
        onClick={onConnect}
        disabled={isConnectDisabled}
        className={`min-w-[90px] ${
          isConnected ? 'border-red-500 text-red-500 hover:bg-red-500/10' : ''
        }`}
        title={connectButtonTitle}
      >
        {getConnectButtonLabel(isConnecting, isConnected, copy)}
      </Button>

      {onClose && (
        <IconButton size="small" onClick={onClose}>
          <X className="h-4 w-4" />
        </IconButton>
      )}
    </div>
  )
}
