import type { ProxyChainCopy } from './proxy-chain-copy'
import { ProxyChainToolbarActions } from './proxy-chain-toolbar-actions'
import { ProxyChainToolbarTitle } from './proxy-chain-toolbar-title'

export interface ProxyChainToolbarProps {
  chainLength: number
  isConnected: boolean
  isConnecting: boolean
  isConnectDisabled: boolean
  connectButtonTitle?: string
  copy: ProxyChainCopy
  onOpenHelp: () => void
  onClearChain: () => void
  onOpenResidentialConfig: () => void
  onConnect: () => void | Promise<void>
  onClose?: () => void
}

export const ProxyChainToolbar = ({
  chainLength,
  isConnected,
  isConnecting,
  isConnectDisabled,
  connectButtonTitle,
  copy,
  onOpenHelp,
  onClearChain,
  onOpenResidentialConfig,
  onConnect,
  onClose,
}: ProxyChainToolbarProps) => {
  return (
    <div className="mb-4 flex items-center justify-between">
      <ProxyChainToolbarTitle copy={copy} onOpenHelp={onOpenHelp} />

      <ProxyChainToolbarActions
        chainLength={chainLength}
        isConnected={isConnected}
        isConnecting={isConnecting}
        isConnectDisabled={isConnectDisabled}
        connectButtonTitle={connectButtonTitle}
        copy={copy}
        onClearChain={onClearChain}
        onOpenResidentialConfig={onOpenResidentialConfig}
        onConnect={onConnect}
        onClose={onClose}
      />
    </div>
  )
}
