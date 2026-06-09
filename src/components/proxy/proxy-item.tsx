import { CheckCircle, SlidersHorizontal } from 'lucide-react'

import { BaseLoading } from '@/components/base'
import { ListItem, ListItemIcon } from '@/components/tailwind/List'
import { ListItemText } from '@/components/tailwind/ListItemText'
import { Tooltip } from '@/components/tailwind/Tooltip'
import { useProxyDelayState } from '@/hooks/network'
import delayManager from '@/services/delay'
import {
  categorizeProxyGroup,
  isProxyGroupItem,
} from '@/services/proxy-display'

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
  sx?: any
  clickable?: boolean
  onClick?: (name: string) => void
  onConfigure?: (group: IProxyGroupItem) => void
}

export const ProxyItem = (props: Props) => {
  const {
    group,
    proxy,
    selected,
    showType = true,
    clickable = true,
    onClick,
    onConfigure,
  } = props
  const isDark = true

  const { delayValue, isPreset, timeout, onDelay } = useProxyDelayState(
    proxy,
    group.name,
  )
  const isConfigurableStrategy =
    Boolean(onConfigure) &&
    isProxyGroupItem(proxy) &&
    categorizeProxyGroup(proxy) === 'strategy'

  const bgcolor = isDark ? '#24252f' : '#ffffff'
  const selectColor = isDark ? '#90caf9' : '#1976d2'

  return (
    <ListItem className="py-0 pl-2">
      <div
        role={clickable ? 'button' : undefined}
        tabIndex={clickable ? 0 : -1}
        className={`mb-2 h-10 rounded group ${
          selected
            ? 'ml-[-3px] w-[calc(100%+3px)] border-l-[3px]'
            : ''
        } ${clickable ? 'cursor-pointer' : 'cursor-default'}`}
        style={{
          backgroundColor: bgcolor,
          ...(selected
            ? {
                borderLeftColor: selectColor,
                backgroundColor: isDark
                  ? 'rgba(25, 118, 210, 0.35)'
                  : 'rgba(25, 118, 210, 0.15)',
              }
            : {}),
        }}
        onClick={clickable ? () => onClick?.(proxy.name) : undefined}
        onKeyDown={
          clickable
            ? (event) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault()
                  onClick?.(proxy.name)
                }
              }
            : undefined
        }
      >
        <ListItemText
          title={proxy.name}
          secondary={
            <>
              <div className="mr-2 inline-block text-sm text-current">
                {proxy.name}
                {showType && proxy.now && ` - ${proxy.now}`}
              </div>
              {showType && !!proxy.provider && (
                <span className="mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50">
                  {proxy.provider}
                </span>
              )}
              {showType && (
                <span className="mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50">
                  {proxy.type}
                </span>
              )}
              {showType && proxy.udp && (
                <span className="mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50">
                  UDP
                </span>
              )}
              {showType && proxy.xudp && (
                <span className="mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50">
                  XUDP
                </span>
              )}
              {showType && proxy.tfo && (
                <span className="mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50">
                  TFO
                </span>
              )}
              {showType && proxy.mptcp && (
                <span className="mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50">
                  MPTCP
                </span>
              )}
              {showType && proxy.smux && (
                <Tooltip title={getSmuxTooltip(proxy)} arrow placement="top">
                  <span className="mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50">
                    {getSmuxShortText(proxy)}
                  </span>
                </Tooltip>
              )}
              {showType &&
                proxy.type === 'mieru' &&
                (proxy as any).multiplexing &&
                (proxy as any).multiplexing !== 'MULTIPLEXING_OFF' && (
                  <Tooltip
                    title={getMieruMultiplexTooltip((proxy as any).multiplexing)}
                    arrow
                    placement="top"
                  >
                    <span className="mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50">
                      {getMieruMultiplexShortText((proxy as any).multiplexing)}
                    </span>
                  </Tooltip>
                )}
              {showType &&
                proxy.type === 'sudoku' &&
                (proxy as any).httpmask?.multiplex &&
                (proxy as any).httpmask.multiplex !== 'off' && (
                  <Tooltip
                    title={getSudokuMultiplexTooltip(
                      (proxy as any).httpmask.multiplex,
                    )}
                    arrow
                    placement="top"
                  >
                    <span className="mr-1 inline-block rounded border border-gray-400/40 px-0.5 text-[10px] leading-5 text-gray-400/50">
                      {getSudokuMultiplexShortText(
                        (proxy as any).httpmask.multiplex,
                      )}
                    </span>
                  </Tooltip>
                )}
            </>
          }
        />

        <ListItemIcon
          className={`justify-end text-primary ${isPreset ? 'hidden' : ''}`}
        >
          {isConfigurableStrategy && (
            <Tooltip title="配置策略池成员" arrow placement="top">
              <div
                className="mr-1 flex h-7 w-7 cursor-pointer items-center justify-center rounded text-amber-400 hover:bg-amber-500/10"
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
            <div className="rounded px-1.5 py-0.5 text-sm">
              <BaseLoading />
            </div>
          )}

          {!proxy.provider && delayValue !== -2 && (
            <div
              className="the-check hidden cursor-pointer rounded px-1.5 py-0.5 text-sm hover:bg-primary/15 group-hover:block"
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
              className={`the-delay rounded px-1.5 py-0.5 text-sm ${
                !proxy.provider ? 'cursor-pointer hover:bg-primary/15' : ''
              }`}
              style={{
                color: delayManager.formatDelayColor(delayValue, timeout),
              }}
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

          {delayValue !== -2 && delayValue < 0 && selected && (
            <CheckCircle className="the-icon h-4 w-4" />
          )}
        </ListItemIcon>
      </div>
    </ListItem>
  )
}
