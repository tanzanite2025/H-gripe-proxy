import { Network, Radar, ShieldCheck } from 'lucide-react'

import { ProviderButton } from '@/components/proxy/provider-button'
import { Box, Button } from '@/components/tailwind'

interface ProxyPageToolbarProps {
  isChainMode: boolean
  toggleLabel: string
  proxyDetectionLabel: string
  dnsLeakLabel: string
  onToggleChainMode: () => void | Promise<void>
  onOpenProxyDetection: () => void
  onOpenDnsLeak: () => void
}

export const ProxyPageToolbar = ({
  isChainMode,
  toggleLabel,
  proxyDetectionLabel,
  dnsLeakLabel,
  onToggleChainMode,
  onOpenProxyDetection,
  onOpenDnsLeak,
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

      <Button
        size="small"
        variant="outlined"
        onClick={onOpenProxyDetection}
        className="ml-1"
        startIcon={<Radar className="h-5 w-5" />}
      >
        {proxyDetectionLabel}
      </Button>

      <Button
        size="small"
        variant="outlined"
        onClick={onOpenDnsLeak}
        className="ml-1"
        startIcon={<ShieldCheck className="h-5 w-5" />}
      >
        {dnsLeakLabel}
      </Button>
    </Box>
  )
}
