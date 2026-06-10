import type { ProxyChainItem } from '../proxy-chain-types'

export interface ProxyChainProps {
  proxyChain: ProxyChainItem[]
  onUpdateChain: (chain: ProxyChainItem[]) => void
  chainConfigData?: string | null
  onMarkUnsavedChanges?: () => void
  selectedGroup?: string | null
  bare?: boolean
  onClose?: () => void
}
