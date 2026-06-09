import { Paper } from '@/components/tailwind/Paper'

import { ProxyChainDialogs } from './proxy-chain-dialogs'
import { ProxyChainGrid } from './proxy-chain-grid'
import { ProxyChainToolbar } from './proxy-chain-toolbar'
import { ResidentialExitSection } from './residential-exit-section'
import { useProxyChainController } from './use-proxy-chain-controller'
import type { ProxyChainProps } from './types'

export const ProxyChain = (props: ProxyChainProps) => {
  const controller = useProxyChainController(props)
  const WrapperComponent = controller.bare ? 'div' : Paper

  return (
    <WrapperComponent className="flex h-full flex-col p-4">
      <ProxyChainToolbar {...controller.toolbarProps} />

      <div className="flex-1 overflow-auto">
        <ProxyChainGrid {...controller.gridProps} />
      </div>

      <ResidentialExitSection {...controller.residentialSectionProps} />
      <ProxyChainDialogs {...controller.dialogsProps} />
    </WrapperComponent>
  )
}
