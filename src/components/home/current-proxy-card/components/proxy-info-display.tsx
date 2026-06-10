import { useTranslation } from 'react-i18next'

import { Chip } from '@/components/tailwind/Chip'
import delayManager from '@/services/delay'
import { cn } from '@/utils/cn'

import { convertDelayColor } from '../utils/delay-visuals'

interface ProxyInfoDisplayProps {
  proxy: any
  delay: number
  timeout: number
}

export const ProxyInfoDisplay = ({
  proxy,
  delay,
  timeout,
}: ProxyInfoDisplayProps) => {
  const { t } = useTranslation()

  if (!proxy) {
    return (
      <div className="py-4 text-center">
        <div className="text-sm text-gray-500 dark:text-gray-400">
          {t('home.components.currentProxy.labels.noActiveNode')}
        </div>
      </div>
    )
  }

  return (
    <div
      className={cn(
        'flex items-center justify-between rounded-2xl border border-solid border-gray-200 p-1.5',
        'bg-gray-50/20 dark:border-gray-700 dark:bg-gray-800/20',
      )}
    >
      <div className="flex flex-wrap items-center">
        <div className="mr-1 text-xs text-gray-500 dark:text-gray-400">
          {proxy.type}
        </div>

        {proxy.udp && (
          <Chip size="small" label="UDP" variant="outlined" className="mr-0.5" />
        )}
        {proxy.tfo && (
          <Chip size="small" label="TFO" variant="outlined" className="mr-0.5" />
        )}
        {proxy.xudp && (
          <Chip size="small" label="XUDP" variant="outlined" className="mr-0.5" />
        )}
        {proxy.mptcp && (
          <Chip size="small" label="MPTCP" variant="outlined" className="mr-0.5" />
        )}
        {proxy.smux && (
          <Chip size="small" label="SMUX" variant="outlined" className="mr-0.5" />
        )}
      </div>

      <Chip
        size="small"
        label={delayManager.formatDelay(delay, timeout)}
        color={convertDelayColor(delay, timeout)}
      />
    </div>
  )
}
