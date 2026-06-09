import { ProxyChainHelpDialog } from '../proxy-chain-help-dialog'
import { ResidentialPoolDialog } from './residential-pool-dialog'
import type { ProxyChainProps } from './types'

export interface ProxyChainDialogsProps {
  helpDialogOpen: boolean
  onCloseHelpDialog: () => void
  residentialConfigOpen: boolean
  localResidentialPool: Parameters<
    typeof ResidentialPoolDialog
  >[0]['config']
  onChangeResidentialPool: Parameters<
    typeof ResidentialPoolDialog
  >[0]['onChange']
  onCloseResidentialConfig: () => void
  onSaveResidentialPool: () => Promise<void>
}

export const ProxyChainDialogs = ({
  helpDialogOpen,
  onCloseHelpDialog,
  residentialConfigOpen,
  localResidentialPool,
  onChangeResidentialPool,
  onCloseResidentialConfig,
  onSaveResidentialPool,
}: ProxyChainDialogsProps) => {
  return (
    <>
      <ResidentialPoolDialog
        open={residentialConfigOpen}
        config={localResidentialPool}
        onChange={onChangeResidentialPool}
        onClose={onCloseResidentialConfig}
        onSave={onSaveResidentialPool}
      />

      <ProxyChainHelpDialog
        open={helpDialogOpen}
        onClose={onCloseHelpDialog}
      />
    </>
  )
}
