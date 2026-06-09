import type { ResidentialExitSectionProps } from './residential-exit-section'
import type { ProxyChainDialogsProps } from './proxy-chain-dialogs'
import type { ProxyChainGridProps } from './proxy-chain-grid'
import type { ProxyChainToolbarProps } from './proxy-chain-toolbar'

interface BuildProxyChainViewModelOptions {
  bare: boolean
  copy: ProxyChainToolbarProps['copy']
  enabledResidentialProxies: ResidentialExitSectionProps['enabledResidentialProxies']
  helpDialogOpen: ProxyChainDialogsProps['helpDialogOpen']
  isConnected: ProxyChainToolbarProps['isConnected']
  isConnecting: ProxyChainToolbarProps['isConnecting']
  localResidentialPool: ProxyChainDialogsProps['localResidentialPool']
  mode?: string
  onAddResidentialExit: ResidentialExitSectionProps['onAddResidentialExit']
  onChangeResidentialPool: ProxyChainDialogsProps['onChangeResidentialPool']
  onClearChain: ProxyChainToolbarProps['onClearChain']
  onClose?: ProxyChainToolbarProps['onClose']
  onCloseHelpDialog: ProxyChainDialogsProps['onCloseHelpDialog']
  onCloseResidentialConfig: ProxyChainDialogsProps['onCloseResidentialConfig']
  onConnect: ProxyChainToolbarProps['onConnect']
  onDragEnd: ProxyChainGridProps['onDragEnd']
  onOpenHelp: ProxyChainToolbarProps['onOpenHelp']
  onOpenResidentialConfig: ProxyChainToolbarProps['onOpenResidentialConfig']
  onRemove: ProxyChainGridProps['onRemove']
  onSaveResidentialPool: ProxyChainDialogsProps['onSaveResidentialPool']
  proxyChain: ProxyChainGridProps['proxyChain']
  residentialConfigOpen: ProxyChainDialogsProps['residentialConfigOpen']
  residentialPool?: ResidentialExitSectionProps['residentialPool']
  selectedGroup?: string | null
  sensors: ProxyChainGridProps['sensors']
}

export const buildProxyChainViewModel = ({
  bare,
  copy,
  enabledResidentialProxies,
  helpDialogOpen,
  isConnected,
  isConnecting,
  localResidentialPool,
  mode,
  onAddResidentialExit,
  onChangeResidentialPool,
  onClearChain,
  onClose,
  onCloseHelpDialog,
  onCloseResidentialConfig,
  onConnect,
  onDragEnd,
  onOpenHelp,
  onOpenResidentialConfig,
  onRemove,
  onSaveResidentialPool,
  proxyChain,
  residentialConfigOpen,
  residentialPool,
  selectedGroup,
  sensors,
}: BuildProxyChainViewModelOptions) => {
  const selectedGroupMissing = mode !== 'global' && !selectedGroup
  const isConnectDisabled =
    isConnecting || proxyChain.length < 2 || selectedGroupMissing

  return {
    bare,
    toolbarProps: {
      chainLength: proxyChain.length,
      isConnected,
      isConnecting,
      isConnectDisabled,
      connectButtonTitle:
        proxyChain.length < 2 ? copy.minimumNodesMessage : undefined,
      copy,
      onOpenHelp,
      onClearChain,
      onOpenResidentialConfig,
      onConnect,
      onClose,
    } satisfies ProxyChainToolbarProps,
    gridProps: {
      proxyChain,
      sensors,
      entryLabel: copy.entryLabel,
      exitLabel: copy.exitLabel,
      timeoutLabel: copy.timeoutLabel,
      emptyLabel: copy.emptyLabel,
      onDragEnd,
      onRemove,
    } satisfies ProxyChainGridProps,
    residentialSectionProps: {
      proxyChain,
      residentialPool,
      enabledResidentialProxies,
      onAddResidentialExit,
    } satisfies ResidentialExitSectionProps,
    dialogsProps: {
      helpDialogOpen,
      onCloseHelpDialog,
      residentialConfigOpen,
      localResidentialPool,
      onChangeResidentialPool,
      onCloseResidentialConfig,
      onSaveResidentialPool,
    } satisfies ProxyChainDialogsProps,
  }
}
