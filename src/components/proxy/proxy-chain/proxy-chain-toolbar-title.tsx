import { AlertTriangle, HelpCircle } from 'lucide-react'

import { TooltipIcon } from '@/components/base'
import { IconButton } from '@/components/tailwind/IconButton'

import type { ProxyChainCopy } from './proxy-chain-copy'

interface ProxyChainToolbarTitleProps {
  copy: Pick<ProxyChainCopy, 'header' | 'instruction' | 'warning' | 'helpLabel'>
  onOpenHelp: () => void
}

export const ProxyChainToolbarTitle = ({
  copy,
  onOpenHelp,
}: ProxyChainToolbarTitleProps) => {
  return (
    <div className="flex items-center gap-2">
      <h3 className="text-lg font-semibold">{copy.header}</h3>
      <span className="text-sm text-text-secondary">{copy.instruction}</span>
      <TooltipIcon
        title={copy.warning}
        icon={AlertTriangle}
        color="warning"
        className="p-1"
      />
      <IconButton
        size="small"
        onClick={onOpenHelp}
        className="ml-1"
        title={copy.helpLabel}
      >
        <HelpCircle className="h-4 w-4" />
      </IconButton>
    </div>
  )
}
