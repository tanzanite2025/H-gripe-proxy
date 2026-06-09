import { Box, Button, ButtonGroup } from '@/components/tailwind'

import {
  getProxyModeDescription,
  PROXY_CHAIN_MODE_LABELS,
  PROXY_CHAIN_MODES,
  PROXY_MODE_SECTION_TITLE,
  type ProxyChainMode,
} from './shared'

interface ProxyPageModeCardProps {
  mode: ProxyChainMode
  onChangeMode: (mode: ProxyChainMode) => void
}

export const ProxyPageModeCard = ({
  mode,
  onChangeMode,
}: ProxyPageModeCardProps) => {
  return (
    <Box className="mx-3 mb-2 rounded-2xl border border-white/10 bg-white/5 px-3 py-2">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="text-sm font-semibold text-text-primary">
            {PROXY_MODE_SECTION_TITLE}
          </div>
          <div className="mt-0.5 text-xs text-text-secondary">
            {getProxyModeDescription(mode)}
          </div>
        </div>

        <ButtonGroup className="uds-toolbar shrink-0" size="small">
          {PROXY_CHAIN_MODES.map((item) => (
            <Button
              key={item}
              variant={item === mode ? 'primary' : 'outlined'}
              onClick={() => onChangeMode(item)}
            >
              {PROXY_CHAIN_MODE_LABELS[item]}
            </Button>
          ))}
        </ButtonGroup>
      </div>
    </Box>
  )
}
