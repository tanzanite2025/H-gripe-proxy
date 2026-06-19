import { ChevronDown, ChevronUp } from 'lucide-react'
import { useTranslation } from 'react-i18next'

import {
  Chip,
  ListItemText,
  Tooltip,
} from '@/components/tailwind'
import type { IProxyGroupItem } from '@/types/proxy'

import type { IRenderItem } from '../render-list/types'
import type { HeadState } from '../use-head-state'

import { ProxyGroupIcon } from './proxy-group-icon'

interface ProxyGroupCardProps {
  group: IProxyGroupItem
  headState?: HeadState
  item: IRenderItem
  onHeadState: (groupName: string, patch: Partial<HeadState>) => void
}

const ITEM_BACKGROUND_COLOR = '#282A36'

export function ProxyGroupCard({
  group,
  headState,
  item,
  onHeadState,
}: ProxyGroupCardProps) {
  const { t } = useTranslation()

  const handleCardActivate = () => {
    onHeadState(group.name, { open: !headState?.open })
  }

  return (
    <div
      role="button"
      tabIndex={0}
      className="mx-2 my-2 flex h-full cursor-pointer items-center rounded-lg px-3 py-1.5 transition-colors hover:bg-action-hover active:bg-action-selected"
      style={{ background: ITEM_BACKGROUND_COLOR }}
      onClick={handleCardActivate}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          handleCardActivate()
        }
      }}
    >
      <ProxyGroupIcon group={group} />

      <ListItemText
        primary={
          <div className="flex items-center gap-2">
            <span className="min-w-0 flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-base font-bold leading-6">
              {group.name}
            </span>
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
