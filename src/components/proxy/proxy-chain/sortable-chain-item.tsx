import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'

import type { ProxyChainItem } from '../proxy-chain-types'

import {
  getProxyChainBorderClass,
  getProxyChainDelayColor,
  getProxyChainRoleChipClass,
  getProxyChainRoleLabel,
} from './proxy-chain-item-state'
import { ProxyChainItemView } from './proxy-chain-item-view'

interface SortableChainItemProps {
  proxy: ProxyChainItem
  index: number
  isFirst: boolean
  isLast: boolean
  entryLabel: string
  exitLabel: string
  timeoutLabel: string
  onRemove: (id: string) => void
}

export const SortableChainItem = ({
  proxy,
  index,
  isFirst,
  isLast,
  entryLabel,
  exitLabel,
  timeoutLabel,
  onRemove,
}: SortableChainItemProps) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: proxy.id })

  const roleLabel = getProxyChainRoleLabel({
    isFirst,
    isLast,
    entryLabel,
    exitLabel,
  })

  const delayLabel =
    proxy.delay !== undefined
      ? proxy.delay > 0
        ? `${proxy.delay}ms`
        : timeoutLabel
      : undefined

  return (
    <div ref={setNodeRef}>
      <ProxyChainItemView
        proxy={proxy}
        index={index}
        roleLabel={roleLabel}
        roleChipClassName={getProxyChainRoleChipClass({ isFirst })}
        borderClassName={getProxyChainBorderClass({ isFirst, isLast })}
        delayLabel={delayLabel}
        delayColor={getProxyChainDelayColor(proxy.delay)}
        isDragging={isDragging}
        style={{
          transform: CSS.Transform.toString(transform),
          transition,
          opacity: isDragging ? 0.5 : 1,
        }}
        dragHandleProps={{ ...attributes, ...listeners }}
        onRemove={onRemove}
      />
    </div>
  )
}
