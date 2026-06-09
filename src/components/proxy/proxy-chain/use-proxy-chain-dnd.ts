import {
  type DragEndEvent,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import { arrayMove, sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { useCallback } from 'react'

import type { ProxyChainItem } from '../proxy-chain-types'

interface UseProxyChainDndOptions {
  proxyChain: ProxyChainItem[]
  onUpdateChain: (chain: ProxyChainItem[]) => void
  onDirty: () => void
}

export const useProxyChainDnd = ({
  proxyChain,
  onUpdateChain,
  onDirty,
}: UseProxyChainDndOptions) => {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event

      if (active.id !== over?.id) {
        const oldIndex = proxyChain.findIndex((item) => item.id === active.id)
        const newIndex = proxyChain.findIndex((item) => item.id === over?.id)

        onUpdateChain(arrayMove(proxyChain, oldIndex, newIndex))
        onDirty()
      }
    },
    [proxyChain, onUpdateChain, onDirty],
  )

  const handleRemoveProxy = useCallback(
    (id: string) => {
      onUpdateChain(proxyChain.filter((item) => item.id !== id))
      onDirty()
    },
    [proxyChain, onUpdateChain, onDirty],
  )

  return {
    sensors,
    handleDragEnd,
    handleRemoveProxy,
  }
}
