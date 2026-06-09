import { ChevronDown, ChevronUp, Inbox, SlidersHorizontal } from 'lucide-react'
import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { Chip, IconButton, ListItemText, Tooltip } from '@/components/tailwind'
import { useIconCache } from '@/hooks/system'
import { categorizeProxyGroup } from '@/services/proxy-display'
import { cn } from '@/utils/cn'

import { ProxyHead } from './proxy-head'
import { ProxyItem } from './proxy-item'
import { ProxyItemMini } from './proxy-item-mini'
import { HeadState } from './use-head-state'
import type { IRenderItem } from './use-render-list'

interface RenderProps {
  item: IRenderItem
  indent: boolean
  onLocation: (group: NonNullable<IRenderItem['group']>) => void
  onCheckAll: (groupName: string) => void
  onHeadState: (groupName: string, patch: Partial<HeadState>) => void
  onChangeProxy: (
    group: NonNullable<IRenderItem['group']>,
    proxy: NonNullable<IRenderItem['proxy']> & { name: string },
  ) => void
  onConfigureStrategyGroup: (
    group: NonNullable<IRenderItem['group']>,
  ) => void
}

const SECTION_TONE_CLASS: Record<string, string> = {
  runtime: 'border-teal-500/20 bg-teal-500/5 text-teal-500',
  manual: 'border-sky-500/20 bg-sky-500/5 text-sky-400',
  strategy: 'border-amber-500/20 bg-amber-500/5 text-amber-400',
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
    onConfigureStrategyGroup,
  } = props

  const { type, group, headState, proxy, proxyCol } = item
  const enableGroupIcon = true
  const isDark = true
  const itemBackgroundColor = isDark ? '#282A36' : '#ffffff'
  const iconCachePath = useIconCache({
    icon: group?.icon,
    cacheKey: group?.name?.replaceAll(' ', '') || 'proxy-group',
    enabled: enableGroupIcon,
  })

  const showType = headState?.showType
  const isStrategyGroup = group
    ? categorizeProxyGroup(group) === 'strategy'
    : false
  const allowMemberSelection = !isStrategyGroup

  const proxyColItemsMemo = useMemo(() => {
    if (type !== 4 || !proxyCol || !group) {
      return null
    }

    return proxyCol.map((proxyItem) => (
      <ProxyItemMini
        key={`${item.key}-${proxyItem?.name ?? 'unknown'}`}
        group={group}
        proxy={proxyItem!}
        selected={group.now === proxyItem?.name}
        showType={showType}
        clickable={allowMemberSelection}
        onClick={
          allowMemberSelection
            ? () => onChangeProxy(group, proxyItem!)
            : undefined
        }
      />
    ))
  }, [
    allowMemberSelection,
    type,
    proxyCol,
    item.key,
    group,
    showType,
    onChangeProxy,
  ])

  if (type === 5) {
    const toneClass =
      SECTION_TONE_CLASS[item.sectionKind || 'manual'] ??
      SECTION_TONE_CLASS.manual
    const isRuntime = item.sectionKind === 'runtime'

    return (
      <div
        className={cn(
          'mx-2 mt-2 rounded-xl border px-3 py-2',
          isRuntime ? 'mb-3' : 'mb-1',
          toneClass,
        )}
      >
        <div className="flex flex-wrap items-center gap-2">
          <span className="text-[11px] font-semibold tracking-[0.18em]">
            {item.sectionTitle}
          </span>
          {item.runtimeObserved === false && (
            <span className="rounded-full border border-gray-500/30 px-2 py-0.5 text-[10px] text-gray-400">
              未观测
            </span>
          )}
        </div>
        {item.runtimePath?.length ? (
          <div className="mt-1 break-all text-sm font-semibold text-white/90">
            {item.runtimePath.join(' -> ')}
          </div>
        ) : null}
        {item.sectionDescription && (
          <div
            className={cn(
              'mt-1 text-xs',
              isRuntime ? 'text-gray-300' : 'text-gray-400',
            )}
          >
            {item.sectionDescription}
          </div>
        )}
        {item.runtimeDescription && (
          <div className="mt-1 text-xs text-gray-400">
            {item.runtimeDescription}
          </div>
        )}
      </div>
    )
  }

  if (type === 0 && group) {
    const canConfigureStrategyGroup = isStrategyGroup

    return (
      <div
        role="button"
        tabIndex={0}
        className="mx-2 my-2 flex h-full cursor-pointer items-center rounded-lg px-3 py-1.5 transition-colors hover:bg-action-hover active:bg-action-selected"
        style={{
          background: itemBackgroundColor,
        }}
        onClick={() => onHeadState(group.name, { open: !headState?.open })}
        onKeyDown={(event) => {
          if (event.key === 'Enter' || event.key === ' ') {
            event.preventDefault()
            onHeadState(group.name, { open: !headState?.open })
          }
        }}
      >
        {enableGroupIcon &&
          group.icon &&
          group.icon.trim().startsWith('http') && (
            <img
              src={iconCachePath === '' ? group.icon : iconCachePath}
              width="32px"
              style={{ marginRight: '12px', borderRadius: '6px' }}
            />
          )}
        {enableGroupIcon &&
          group.icon &&
          group.icon.trim().startsWith('data') && (
            <img
              src={group.icon}
              width="32px"
              style={{ marginRight: '12px', borderRadius: '6px' }}
            />
          )}
        {enableGroupIcon &&
          group.icon &&
          group.icon.trim().startsWith('<svg') && (
            <img
              src={`data:image/svg+xml;base64,${btoa(group.icon)}`}
              width="32px"
            />
          )}
        <ListItemText
          primary={
            <div className="flex items-center gap-2">
              <span className="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-base font-bold leading-6">
                {group.name}
              </span>
              {canConfigureStrategyGroup && (
                <Tooltip title="配置策略池成员" arrow>
                  <span>
                    <IconButton
                      size="small"
                      color="primary"
                      className="h-7 w-7"
                      onClick={(event) => {
                        event.preventDefault()
                        event.stopPropagation()
                        onConfigureStrategyGroup(group)
                      }}
                      onKeyDown={(event) => {
                        event.stopPropagation()
                      }}
                    >
                      <SlidersHorizontal className="h-4 w-4" />
                    </IconButton>
                  </span>
                </Tooltip>
              )}
            </div>
          }
          secondary={
            <div className="flex items-center overflow-hidden pt-0.5">
              <span className="mt-0.5">
                <span className="mr-2 inline-block rounded border border-teal-500/50 px-1 text-[10px] leading-6 text-teal-500/80">
                  {group.type}
                </span>
                <span className="overflow-hidden text-ellipsis whitespace-nowrap text-[13px] text-gray-500">
                  {item.pathText || group.now}
                </span>
              </span>
            </div>
          }
        />
        <div className="flex items-center">
          <Tooltip title={t('proxies.page.labels.proxyCount')} arrow>
            <Chip
              size="small"
              label={`${item.memberCount ?? group.all.length}`}
              className="mr-2 bg-teal-500/10 text-teal-500"
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

  if (type === 1 && group) {
    return (
      <ProxyHead
        className={cn('mb-2 pl-4 pr-6', indent ? 'mt-2' : 'mt-1')}
        url={group.testUrl}
        groupName={group.name}
        headState={headState!}
        onLocation={() => onLocation(group)}
        onCheckDelay={() => onCheckAll(group.name)}
        onHeadState={(patch) => onHeadState(group.name, patch)}
      />
    )
  }

  if (type === 2 && group && proxy) {
    return (
      <ProxyItem
        group={group}
        proxy={proxy}
        selected={group.now === proxy.name}
        showType={headState?.showType}
        sx={{ py: 0, pl: 2 }}
        clickable={allowMemberSelection}
        onClick={
          allowMemberSelection ? () => onChangeProxy(group, proxy) : undefined
        }
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
        className="grid h-16 gap-2 px-4 py-1"
        style={{
          gridTemplateColumns: `repeat(${item.col || 2}, 1fr)`,
        }}
      >
        {proxyColItemsMemo}
      </div>
    )
  }

  return null
}
