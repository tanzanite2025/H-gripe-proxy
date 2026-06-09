import { Network } from 'lucide-react'

import { ProviderButton } from '@/components/proxy/provider-button'
import { Box, Button } from '@/components/tailwind'

interface ProxyPageToolbarProps {
  isChainMode: boolean
  toggleLabel: string
  onToggleChainMode: () => void | Promise<void>
}

export const ProxyPageToolbar = ({
  isChainMode,
  toggleLabel,
  onToggleChainMode,
}: ProxyPageToolbarProps) => {
  return (
    <Box className="mb-2 flex items-center gap-1 pl-3">
      <ProviderButton />

      <Button
        size="small"
        variant={isChainMode ? 'primary' : 'outlined'}
        onClick={onToggleChainMode}
        className="ml-1"
        startIcon={<Network className="h-5 w-5" />}
      >
        {toggleLabel}
      </Button>
    </Box>
  )
}
