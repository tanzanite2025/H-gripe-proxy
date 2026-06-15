import {
  closestCenter,
  DndContext,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import { SortableContext, sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { useMemo } from 'react'

import { ProxyItem } from '../proxy-item'

import type { SortableProxySectionProps } from './types'

export function SortableProxySection({
  kind,
  items,
  onDelete,
  onDragEnd,
}: SortableProxySectionProps) {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )
  const itemIds = useMemo(() => items.map((item) => item.name), [items])

  return (
    <DndContext
      sensors={sensors}
      collisionDetection={closestCenter}
      onDragEnd={onDragEnd}
    >
      <SortableContext items={itemIds}>
        {items.map((item) => (
          <ProxyItem
            key={item.name}
            type={kind}
            proxy={item}
            onDelete={() => onDelete(item.name)}
          />
        ))}
      </SortableContext>
    </DndContext>
  )
}
