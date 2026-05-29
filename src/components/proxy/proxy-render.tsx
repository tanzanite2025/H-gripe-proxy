import { ChevronDown, ChevronUp, Inbox } from 'lucide-react'
import {
  Chip,
  ListItemText,
  Tooltip,
} from '@/components/tailwind'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/utils/cn'

import { useIconCache, useVerge } from '@/hooks/system'
import { useThemeMode } from '@/services/states'

import { ProxyHead } from './proxy-head'
import { ProxyItem } from './proxy-item'
import { ProxyItemMini } from './proxy-item-mini'
import { HeadState } from './use-head-state'
import type { IRenderItem } from './use-render-list'

interface RenderProps {
  item: IRenderItem
  indent: boolean
  isChainMode?: boolean
  onLocation: (group: IRenderItem['group']) => void
  onCheckAll: (groupName: string) => void
  onHeadState: (groupName: string, patch: Partial<HeadState>) => void
  onChangeProxy: (
    group: IRenderItem['group'],
    proxy: IRenderItem['proxy'] & { name: string },
  ) => void
}

export const ProxyRender = (props: RenderProps) => {
  const { t } = useTranslation()
  const {
    indent,
    item,
    onLocation,
    onCheckAll,
    onHeadState,
    onChangeProxy,
    isChainMode: _ = false,
  } = props
  const { type, group, headState, proxy, proxyCol } = item
  const { verge } = useVerge()
  const enable_group_icon = verge?.enable_group_icon ?? true
  const mode = useThemeMode()
  const isDark = mode === 'light' ? false : true
  const itembackgroundcolor = isDark ? '#282A36' : '#ffffff'
  const iconCachePath = useIconCache({
    icon: group.icon,
    cacheKey: group.name.replaceAll(' ', ''),
    enabled: enable_group_icon,
  })

  const showType = headState?.showType
  const proxyColItemsMemo = useMemo(() => {
    if (type !== 4 || !proxyCol) {
      return null
    }

    return proxyCol.map((proxyItem) => (
      <ProxyItemMini
        key={`${item.key}-${proxyItem?.name ?? 'unknown'}`}
        group={group}
        proxy={proxyItem!}
        selected={group.now === proxyItem?.name}
        showType={showType}
        onClick={() => onChangeProxy(group, proxyItem!)}
      />
    ))
  }, [type, proxyCol, item.key, group, showType, onChangeProxy])

  if (type === 0) {
    return (
      <div
        role="button"
        tabIndex={0}
        className="mx-2 my-2 flex h-full items-center rounded-lg px-3 py-1.5 cursor-pointer transition-colors hover:bg-action-hover active:bg-action-selected"
        style={{
          background: itembackgroundcolor,
        }}
        onClick={() => onHeadState(group.name, { open: !headState?.open })}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault()
            onHeadState(group.name, { open: !headState?.open })
          }
        }}
      >
        {enable_group_icon &&
          group.icon &&
          group.icon.trim().startsWith('http') && (
            <img
              src={iconCachePath === '' ? group.icon : iconCachePath}
              width="32px"
              style={{ marginRight: '12px', borderRadius: '6px' }}
            />
          )}
        {enable_group_icon &&
          group.icon &&
          group.icon.trim().startsWith('data') && (
            <img
              src={group.icon}
              width="32px"
              style={{ marginRight: '12px', borderRadius: '6px' }}
            />
          )}
        {enable_group_icon &&
          group.icon &&
          group.icon.trim().startsWith('<svg') && (
            <img
              src={`data:image/svg+xml;base64,${btoa(group.icon)}`}
              width="32px"
            />
          )}
        <ListItemText
          primary={
            <span className="overflow-hidden text-ellipsis whitespace-nowrap text-base font-bold leading-6">
              {group.name}
            </span>
          }
          secondary={
            <div className="flex items-center overflow-hidden pt-0.5">
              <span className="mt-0.5">
                <span className="mr-2 inline-block rounded border border-blue-500/50 px-1 text-[10px] leading-6 text-blue-500/80">
                  {group.type}
                </span>
                <span className="overflow-hidden text-ellipsis whitespace-nowrap text-[13px] text-gray-500">
                  {group.now}
                </span>
              </span>
            </div>
          }
        />
        <div className="flex items-center">
          <Tooltip title={t('proxies.page.labels.proxyCount')} arrow>
            <Chip
              size="small"
              label={`${group.all.length}`}
              className="mr-2 bg-blue-500/10 text-blue-500"
            />
          </Tooltip>
          {headState?.open ? (
            <ChevronUp className="h-5 w-5" />
          ) : (
            <ChevronDown className="h-5 w-5" />
          )}
        </div>
      </div>
    )
  }

  if (type === 1) {
    return (
      <ProxyHead
        className={cn('pl-4 pr-6 mb-2', indent ? 'mt-2' : 'mt-1')}
        url={group.testUrl}
        groupName={group.name}
        headState={headState!}
        onLocation={() => onLocation(group)}
        onCheckDelay={() => onCheckAll(group.name)}
        onHeadState={(p) => onHeadState(group.name, p)}
      />
    )
  }

  if (type === 2) {
    return (
      <ProxyItem
        group={group}
        proxy={proxy!}
        selected={group.now === proxy?.name}
        showType={headState?.showType}
        sx={{ py: 0, pl: 2 }}
        onClick={() => onChangeProxy(group, proxy!)}
      />
    )
  }

  if (type === 3) {
    return (
      <div className="flex flex-col items-center justify-center py-4 pl-0">
        <Inbox className="text-2xl" />
        <span>No Proxies</span>
      </div>
    )
  }

  if (type === 4) {
    return (
      <div
        className="grid h-14 gap-2 px-4 pb-2"
        style={{
          gridTemplateColumns: `repeat(${item.col! || 2}, 1fr)`,
        }}
      >
        {proxyColItemsMemo}
      </div>
    )
  }

  return null
}
