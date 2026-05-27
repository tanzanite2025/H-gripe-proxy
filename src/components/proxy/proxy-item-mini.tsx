import { CheckCircleOutlineRounded } from '@mui/icons-material'
import { useTranslation } from 'react-i18next'

import { BaseLoading } from '@/components/base'
import { ListItemButton } from '@/components/tailwind/ListItemButton'
import { Tooltip } from '@/components/tailwind/Tooltip'
import { useProxyDelayState } from '@/hooks/network'
import delayManager from '@/services/delay'
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
  onClick?: (name: string) => void
}

// 多列布局
export const ProxyItemMini = (props: Props) => {
  const { group, proxy, selected, showType = true, onClick } = props

  const { t } = useTranslation()

  // -1/<=0 为不显示，-2 为 loading
  const { delayValue, isPreset, timeout, onDelay } = useProxyDelayState(
    proxy,
    group.name,
  )

  const showDelay = delayValue > 0

  return (
    <ListItemButton
      selected={selected}
      onClick={() => onClick?.(proxy.name)}
      className={cn(
        'h-14 rounded-xl pl-3 pr-2 justify-between items-center relative',
        'bg-white dark:bg-[#24252f]',
        'group',
        selected && 'w-[calc(100%+3px)] -ml-[3px] border-l-[3px] border-primary bg-primary/15 dark:bg-primary/35'
      )}
    >
      <div
        title={`${proxy.name}\n${proxy.now ?? ''}`}
        className="overflow-hidden"
      >
        <div className="block text-sm text-text-primary overflow-hidden text-ellipsis whitespace-nowrap break-all">
          {proxy.name}
        </div>

        {showType && (
          <div className="flex flex-nowrap flex-none mt-1">
            {proxy.now && (
              <div className="block text-sm text-text-secondary overflow-hidden text-ellipsis whitespace-nowrap break-all mr-2">
                {proxy.now}
              </div>
            )}
            {!!proxy.provider && (
              <span className="inline-block border border-text-secondary text-text-secondary rounded text-[10px] mr-1 px-1 leading-normal">
                {proxy.provider}
              </span>
            )}
            <span className="inline-block border border-text-secondary text-text-secondary rounded text-[10px] mr-1 px-1 leading-normal">
              {proxy.type}
            </span>
            {proxy.udp && (
              <span className="inline-block border border-text-secondary text-text-secondary rounded text-[10px] mr-1 px-1 leading-normal">
                UDP
              </span>
            )}
            {proxy.xudp && (
              <span className="inline-block border border-text-secondary text-text-secondary rounded text-[10px] mr-1 px-1 leading-normal">
                XUDP
              </span>
            )}
            {proxy.tfo && (
              <span className="inline-block border border-text-secondary text-text-secondary rounded text-[10px] mr-1 px-1 leading-normal">
                TFO
              </span>
            )}
            {proxy.mptcp && (
              <span className="inline-block border border-text-secondary text-text-secondary rounded text-[10px] mr-1 px-1 leading-normal">
                MPTCP
              </span>
            )}
            {proxy.smux && (
              <Tooltip title={getSmuxTooltip(proxy)} arrow placement="top">
                <span className="inline-block border border-text-secondary text-text-secondary rounded text-[10px] mr-1 px-1 leading-normal">
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
                  <span className="inline-block border border-text-secondary text-text-secondary rounded text-[10px] mr-1 px-1 leading-normal">
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
                  <span className="inline-block border border-text-secondary text-text-secondary rounded text-[10px] mr-1 px-1 leading-normal">
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
        {delayValue === -2 && (
          <div className="p-0.5 px-1 text-sm rounded">
            <BaseLoading />
          </div>
        )}
        {!proxy.provider && delayValue !== -2 && (
          // provider 的节点不支持检测
          <div
            className="the-check hidden group-hover:block p-0.5 px-1 text-sm rounded hover:bg-primary/15"
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
          // 显示延迟
          <div
            className={cn(
              'the-delay p-0.5 px-1 text-sm rounded',
              !proxy.provider && 'hover:bg-primary/15'
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
            // 展示已选择的 icon
            <CheckCircleOutlineRounded
              className="the-icon block text-base mr-1"
            />
          )}
      </div>
      {group.fixed && group.fixed === proxy.name && (
        // 展示 fixed 状态
        <span
          className={cn(
            'absolute text-xs -top-1 -right-1',
            proxy.name === group.now ? 'the-pin' : 'the-unpin grayscale'
          )}
          title={
            group.type === 'URLTest'
              ? t('proxies.page.labels.delayCheckReset')
              : ''
          }
        >
          📌
        </span>
      )}
    </ListItemButton>
  )
}
