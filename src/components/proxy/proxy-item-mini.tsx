import { CheckCircle2, Pin, SlidersHorizontal } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import { BaseLoading } from '@/components/base'
import { ListItemButton } from '@/components/tailwind/ListItemButton'
import { Tooltip } from '@/components/tailwind/Tooltip'
import { useProxyDelayState } from '@/hooks/network'
import delayManager from '@/services/delay'
import {
  categorizeProxyGroup,
  isProxyGroupItem,
} from '@/services/proxy-display'
import { cn } from '@/utils/cn'

import {
  getMieruMultiplexShortText,
  getMieruMultiplexTooltip,
  getSmuxShortText,
  getSmuxTooltip,
  getSudokuMultiplexShortText,
  getSudokuMultiplexTooltip,
} from './utils/multiplexing-helpers'

interface Props {
  group: IProxyGroupItem
  proxy: IProxyItem
  selected: boolean
  showType?: boolean
  clickable?: boolean
  onClick?: (name: string) => void
  onConfigure?: (group: IProxyGroupItem) => void
}

export const ProxyItemMini = (props: Props) => {
  const {
    group,
    proxy,
    selected,
    showType = true,
    clickable = true,
    onClick,
    onConfigure,
  } = props

  const { t } = useTranslation()
  const { delayValue, isPreset, timeout, onDelay } = useProxyDelayState(
    proxy,
    group.name,
  )
  const isConfigurableStrategy =
    Boolean(onConfigure) &&
    isProxyGroupItem(proxy) &&
    categorizeProxyGroup(proxy) === 'strategy'

  return (
    <ListItemButton
      selected={selected}
      disabled={!clickable}
      onClick={clickable ? () => onClick?.(proxy.name) : undefined}
      className={cn(
        'relative h-14 items-center justify-between rounded-xl bg-white pl-3 pr-2 group dark:bg-[#24252f]',
        selected &&
          'w-[calc(100%+3px)] -ml-[3px] border-l-[3px] border-primary bg-primary/15 dark:bg-primary/35',
        !clickable &&
          'hover:bg-white active:bg-white dark:hover:bg-[#24252f] dark:active:bg-[#24252f]',
      )}
    >
      <div
        title={`${proxy.name}\n${proxy.now ?? ''}`}
        className="overflow-hidden"
      >
        <div className="block overflow-hidden text-ellipsis whitespace-nowrap break-all text-sm text-text-primary">
          {proxy.name}
        </div>

        {showType && (
          <div className="mt-1 flex flex-none flex-nowrap">
            {proxy.now && (
              <div className="mr-2 block overflow-hidden text-ellipsis whitespace-nowrap break-all text-sm text-text-secondary">
                {proxy.now}
              </div>
            )}
            {!!proxy.provider && (
              <span className="mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary">
                {proxy.provider}
              </span>
            )}
            <span className="mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary">
              {proxy.type}
            </span>
            {proxy.udp && (
              <span className="mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary">
                UDP
              </span>
            )}
            {proxy.xudp && (
              <span className="mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary">
                XUDP
              </span>
            )}
            {proxy.tfo && (
              <span className="mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary">
                TFO
              </span>
            )}
            {proxy.mptcp && (
              <span className="mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary">
                MPTCP
              </span>
            )}
            {proxy.smux && (
              <Tooltip title={getSmuxTooltip(proxy)} arrow placement="top">
                <span className="mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary">
                  {getSmuxShortText(proxy)}
                </span>
              </Tooltip>
            )}
            {proxy.type === 'mieru' &&
              (proxy as any).multiplexing &&
              (proxy as any).multiplexing !== 'MULTIPLEXING_OFF' && (
                <Tooltip
                  title={getMieruMultiplexTooltip((proxy as any).multiplexing)}
                  arrow
                  placement="top"
                >
                  <span className="mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary">
                    {getMieruMultiplexShortText((proxy as any).multiplexing)}
                  </span>
                </Tooltip>
              )}
            {proxy.type === 'sudoku' &&
              (proxy as any).httpmask?.multiplex &&
              (proxy as any).httpmask.multiplex !== 'off' && (
                <Tooltip
                  title={getSudokuMultiplexTooltip(
                    (proxy as any).httpmask.multiplex,
                  )}
                  arrow
                  placement="top"
                >
                  <span className="mr-1 inline-block rounded border border-text-secondary px-1 text-[10px] leading-normal text-text-secondary">
                    {getSudokuMultiplexShortText(
                      (proxy as any).httpmask.multiplex,
                    )}
                  </span>
                </Tooltip>
              )}
          </div>
        )}
      </div>
      <div className={cn('ml-1 text-primary', isPreset && 'hidden')}>
        {isConfigurableStrategy && (
          <Tooltip title="配置策略池成员" arrow placement="top">
            <div
              className="mb-1 ml-auto flex h-6 w-6 cursor-pointer items-center justify-center rounded text-amber-400 hover:bg-amber-500/10"
              onClick={(e) => {
                e.preventDefault()
                e.stopPropagation()
                onConfigure?.(proxy)
              }}
            >
              <SlidersHorizontal className="h-3.5 w-3.5" />
            </div>
          </Tooltip>
        )}
        {delayValue === -2 && (
          <div className="rounded p-0.5 px-1 text-sm">
            <BaseLoading />
          </div>
        )}
        {!proxy.provider && delayValue !== -2 && (
          <div
            className="the-check hidden rounded p-0.5 px-1 text-sm hover:bg-primary/15 group-hover:block"
            onClick={(e) => {
              e.preventDefault()
              e.stopPropagation()
              onDelay()
            }}
          >
            Check
          </div>
        )}

        {delayValue >= 0 && (
          <div
            className={cn(
              'the-delay rounded p-0.5 px-1 text-sm',
              !proxy.provider && 'hover:bg-primary/15',
            )}
            style={{ color: delayManager.formatDelayColor(delayValue, timeout) }}
            onClick={(e) => {
              if (proxy.provider) return
              e.preventDefault()
              e.stopPropagation()
              onDelay()
            }}
          >
            {delayManager.formatDelay(delayValue, timeout)}
          </div>
        )}
        {proxy.type !== 'Direct' &&
          delayValue !== -2 &&
          delayValue < 0 &&
          selected && (
            <CheckCircle2 className="the-icon mr-1 block h-4 w-4" />
          )}
      </div>
      {group.fixed && group.fixed === proxy.name && (
        <span
          className={cn(
            'absolute -right-1 -top-1 text-xs',
            proxy.name === group.now ? 'the-pin' : 'the-unpin grayscale',
          )}
          title={
            group.type === 'URLTest'
              ? t('proxies.page.labels.delayCheckReset')
              : ''
          }
        >
          <Pin className="h-3.5 w-3.5" />
        </span>
      )}
    </ListItemButton>
  )
}
