import { HelpCircle as HelpIcon, X as CloseIcon } from 'lucide-react'
import { useState, type SyntheticEvent } from 'react'

import {
  Dialog,
  DialogContent,
  DialogTitle,
} from '@/components/tailwind/Dialog'
import { IconButton } from '@/components/tailwind/IconButton'
import { Tab, Tabs } from '@/components/tailwind/Tabs'

import { ProxyChainHelpBestPracticesTab } from './proxy-chain-help-dialog/best-practices-tab'
import {
  PROXY_CHAIN_HELP_TABS,
  PROXY_CHAIN_HELP_TITLE,
} from './proxy-chain-help-dialog/data'
import { ProxyChainHelpExamplesTab } from './proxy-chain-help-dialog/examples-tab'
import { ProxyChainHelpFaqTab } from './proxy-chain-help-dialog/faq-tab'
import { ProxyChainHelpOverviewTab } from './proxy-chain-help-dialog/overview-tab'
import { ProxyChainHelpSetupTab } from './proxy-chain-help-dialog/setup-tab'
import { ProxyChainHelpTabPanel } from './proxy-chain-help-dialog/shared'

interface ProxyChainHelpDialogProps {
  open: boolean
  onClose: () => void
}

export const ProxyChainHelpDialog = ({
  open,
  onClose,
}: ProxyChainHelpDialogProps) => {
  const [tabValue, setTabValue] = useState(0)

  const handleTabChange = (
    _event: SyntheticEvent,
    newValue: string | number,
  ) => {
    setTabValue(Number(newValue))
  }

  return (
    <Dialog open={open} onClose={onClose} maxWidth="md" fullWidth>
      <DialogTitle>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <HelpIcon className="text-primary" />
            <h6 className="text-lg font-semibold">{PROXY_CHAIN_HELP_TITLE}</h6>
          </div>
          <IconButton onClick={onClose} size="small">
            <CloseIcon />
          </IconButton>
        </div>
      </DialogTitle>

      <DialogContent>
        <Tabs
          value={tabValue}
          onChange={handleTabChange}
          className="border-b border-divider"
        >
          {PROXY_CHAIN_HELP_TABS.map((label) => (
            <Tab key={label} label={label} />
          ))}
        </Tabs>

        <ProxyChainHelpTabPanel value={tabValue} index={0}>
          <ProxyChainHelpOverviewTab />
        </ProxyChainHelpTabPanel>

        <ProxyChainHelpTabPanel value={tabValue} index={1}>
          <ProxyChainHelpSetupTab />
        </ProxyChainHelpTabPanel>

        <ProxyChainHelpTabPanel value={tabValue} index={2}>
          <ProxyChainHelpExamplesTab />
        </ProxyChainHelpTabPanel>

        <ProxyChainHelpTabPanel value={tabValue} index={3}>
          <ProxyChainHelpBestPracticesTab />
        </ProxyChainHelpTabPanel>

        <ProxyChainHelpTabPanel value={tabValue} index={4}>
          <ProxyChainHelpFaqTab />
        </ProxyChainHelpTabPanel>
      </DialogContent>
    </Dialog>
  )
}
