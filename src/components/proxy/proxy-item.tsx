import { CheckCircle } from 'lucide-react'

import { BaseLoading } from '@/components/base'
import {
  ListItem,
  ListItemIcon,
} from '@/components/tailwind/List'
import { ListItemText } from '@/components/tailwind/ListItemText'
import { Tooltip } from '@/components/tailwind/Tooltip'
import { useProxyDelayState } from '@/hooks/network'
import delayManager from '@/services/delay'
import { useThemeMode } from '@/services/states'

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
  onClick?: (name: string) => void
}

export const ProxyItem = (props: Props) => {
  const { group, proxy, selected, showType = true, onClick } = props
  const mode = useThemeMode()
  const isDark = mode === 'dark'

  // -1/<=0 为不显示，-2 为 loading
  const { delayValue, isPreset, timeout, onDelay } = useProxyDelayState(
    proxy,
    group.name,
  )

  const bgcolor = isDark ? '#24252f' : '#ffffff'
  const selectColor = isDark ? '#90caf9' : '#1976d2'
  const showDelay = delayValue > 0

  return (
    <ListItem className="py-0 pl-2">
      <div
        role="button"
        tabIndex={0}
        className={`rounded mb-2 h-10 group ${
          selected
            ? `border-l-[3px] ml-[-3px] w-[calc(100%+3px)]`
            : ''
        }`}
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
        onClick={() => onClick?.(proxy.name)}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault()
            onClick?.(proxy.name)
          }
        }}
      >
        <ListItemText
          title={proxy.name}
          secondary={
            <>
              <div className="inline-block mr-2 text-sm text-current">
                {proxy.name}
                {showType && proxy.now && ` - ${proxy.now}`}
              </div>
              {showType && !!proxy.provider && (
                <span className="inline-block border border-gray-400/40 text-gray-400/50 rounded text-[10px] mr-1 px-0.5 leading-5">
                  {proxy.provider}
                </span>
              )}
              {showType && (
                <span className="inline-block border border-gray-400/40 text-gray-400/50 rounded text-[10px] mr-1 px-0.5 leading-5">
                  {proxy.type}
                </span>
              )}
              {showType && proxy.udp && (
                <span className="inline-block border border-gray-400/40 text-gray-400/50 rounded text-[10px] mr-1 px-0.5 leading-5">
                  UDP
                </span>
              )}
              {showType && proxy.xudp && (
                <span className="inline-block border border-gray-400/40 text-gray-400/50 rounded text-[10px] mr-1 px-0.5 leading-5">
                  XUDP
                </span>
              )}
              {showType && proxy.tfo && (
                <span className="inline-block border border-gray-400/40 text-gray-400/50 rounded text-[10px] mr-1 px-0.5 leading-5">
                  TFO
                </span>
              )}
              {showType && proxy.mptcp && (
                <span className="inline-block border border-gray-400/40 text-gray-400/50 rounded text-[10px] mr-1 px-0.5 leading-5">
                  MPTCP
                </span>
              )}
              {showType && proxy.smux && (
                <Tooltip title={getSmuxTooltip(proxy)} arrow placement="top">
                  <span className="inline-block border border-gray-400/40 text-gray-400/50 rounded text-[10px] mr-1 px-0.5 leading-5">
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
                    <span className="inline-block border border-gray-400/40 text-gray-400/50 rounded text-[10px] mr-1 px-0.5 leading-5">
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
                    <span className="inline-block border border-gray-400/40 text-gray-400/50 rounded text-[10px] mr-1 px-0.5 leading-5">
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
          {delayValue === -2 && (
            <div className="py-0.5 px-1.5 text-sm rounded">
              <BaseLoading />
            </div>
          )}

          {!proxy.provider && delayValue !== -2 && (
            <div
              className="the-check hidden group-hover:block py-0.5 px-1.5 text-sm rounded hover:bg-primary/15 cursor-pointer"
              onClick={(e) => {
                e.preventDefault()
                e.stopPropagation()
                onDelay()
              }}
            >
              Check
            </div>
          )}

          {delayValue > 0 && (
            <div
              className={`the-delay py-0.5 px-1.5 text-sm rounded ${
                !proxy.provider ? 'hover:bg-primary/15 cursor-pointer' : ''
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

          {delayValue !== -2 && delayValue <= 0 && selected && (
            <CheckCircle className="the-icon h-4 w-4" />
          )}
        </ListItemIcon>
      </div>
    </ListItem>
  )
}
