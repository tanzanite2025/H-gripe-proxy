import {
  DragEndEvent,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import { sortableKeyboardCoordinates } from '@dnd-kit/sortable'
import { useCallback } from 'react'

import { reorderArray } from '../utils/group-helpers'

interface UseGroupDragDropProps {
  prependSeq: IProxyGroupConfig[]
  appendSeq: IProxyGroupConfig[]
  setPrependSeq: (seq: IProxyGroupConfig[]) => void
  setAppendSeq: (seq: IProxyGroupConfig[]) => void
}

export const useGroupDragDrop = ({
  prependSeq,
  appendSeq,
  setPrependSeq,
  setAppendSeq,
}: UseGroupDragDropProps) => {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )

  const onPrependDragEnd = useCallback(
    async (event: DragEndEvent) => {
      const { active, over } = event
      if (over && active.id !== over.id) {
        let activeIndex = 0
        let overIndex = 0
        prependSeq.forEach((item, index) => {
          if (item.name === active.id) {
            activeIndex = index
          }
          if (item.name === over.id) {
            overIndex = index
          }
        })

        setPrependSeq(reorderArray(prependSeq, activeIndex, overIndex))
      }
    },
    [prependSeq, setPrependSeq],
  )

  const onAppendDragEnd = useCallback(
    async (event: DragEndEvent) => {
      const { active, over } = event
      if (over && active.id !== over.id) {
        let activeIndex = 0
        let overIndex = 0
        appendSeq.forEach((item, index) => {
          if (item.name === active.id) {
            activeIndex = index
          }
          if (item.name === over.id) {
            overIndex = index
          }
        })
        setAppendSeq(reorderArray(appendSeq, activeIndex, overIndex))
      }
    },
    [appendSeq, setAppendSeq],
  )

  return {
    sensors,
    onPrependDragEnd,
    onAppendDragEnd,
  }
}
