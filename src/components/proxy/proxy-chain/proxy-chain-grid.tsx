import {
  closestCenter,
  DndContext,
  type DragEndEvent,
  type SensorDescriptor,
  type SensorOptions,
} from '@dnd-kit/core'
import { rectSortingStrategy, SortableContext } from '@dnd-kit/sortable'

import type { ProxyChainItem } from '../proxy-chain-types'
import { SortableChainItem } from './sortable-chain-item'

export interface ProxyChainGridProps {
  proxyChain: ProxyChainItem[]
  sensors: Array<SensorDescriptor<SensorOptions>>
  entryLabel: string
  exitLabel: string
  timeoutLabel: string
  emptyLabel: string
  onDragEnd: (event: DragEndEvent) => void
  onRemove: (id: string) => void
}

export const ProxyChainGrid = ({
  proxyChain,
  sensors,
  entryLabel,
  exitLabel,
  timeoutLabel,
  emptyLabel,
  onDragEnd,
  onRemove,
}: ProxyChainGridProps) => {
  if (proxyChain.length === 0) {
    return (
      <div className="flex h-full items-center justify-center text-text-secondary">
        <span>{emptyLabel}</span>
      </div>
    )
  }

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={onDragEnd}
    >
      <SortableContext
        items={proxyChain.map((proxy) => proxy.id)}
        strategy={rectSortingStrategy}
      >
        <div className="grid grid-cols-4 gap-2 p-2">
          {proxyChain.map((proxy, index) => (
            <SortableChainItem
              key={proxy.id}
              proxy={proxy}
              index={index}
              isFirst={index === 0}
              isLast={index === proxyChain.length - 1 && proxyChain.length > 1}
              entryLabel={entryLabel}
              exitLabel={exitLabel}
              timeoutLabel={timeoutLabel}
              onRemove={onRemove}
            />
          ))}
        </div>
      </SortableContext>
    </DndContext>
  )
}
