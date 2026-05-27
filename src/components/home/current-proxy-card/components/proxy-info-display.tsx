import { useTranslation } from 'react-i18next'

import { Chip } from '@/components/tailwind/Chip'
import delayManager from '@/services/delay'
import { cn } from '@/utils/cn'

import { convertDelayColor } from '../utils/proxy-helpers'

interface ProxyInfoDisplayProps {
  proxy: any
  delay: number
  isGlobalMode: boolean
  isDirectMode: boolean
}

/**
 * 代理信息展示组件
 * 显示当前代理的详细信息和延迟
 */
export const ProxyInfoDisplay = ({
  proxy,
  delay,
  isGlobalMode,
  isDirectMode,
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
        'mb-1.5 flex items-center justify-between rounded-2xl border border-dashed border-gray-200 p-1.5',
        'bg-gray-50/20 dark:border-gray-700 dark:bg-gray-800/20',
      )}
    >
      <div>
        <div className="text-sm font-medium">
          {proxy.name}
        </div>

        <div className="flex flex-wrap items-center">
          <div className="mr-1 text-xs text-gray-500 dark:text-gray-400">
            {proxy.type}
          </div>

          {/* 模式标签 */}
          {isGlobalMode && (
            <Chip
              size="small"
              label={t('home.components.currentProxy.labels.globalMode')}
              color="primary"
              className="mr-0.5"
            />
          )}
          {isDirectMode && (
            <Chip
              size="small"
              label={t('home.components.currentProxy.labels.directMode')}
              color="success"
              className="mr-0.5"
            />
          )}

          {/* 节点特性 */}
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
      </div>

      {/* 显示延迟 */}
      {!isDirectMode && (
        <Chip
          size="small"
          label={delayManager.formatDelay(delay)}
          color={convertDelayColor(delay)}
        />
      )}
    </div>
  )
}
