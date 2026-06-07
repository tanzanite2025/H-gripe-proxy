import {
  DndContext,
  type DragEndEvent,
  KeyboardSensor,
  PointerSensor,
  closestCenter,
  useSensor,
  useSensors,
} from '@dnd-kit/core'
import {
  SortableContext,
  sortableKeyboardCoordinates,
} from '@dnd-kit/sortable'
import {
  useMemo,
  useState,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from 'react'

import { BaseSearchBox, VirtualList } from '@/components/base'
import { RuleItem } from '@/components/profile/rule-item'
import { List } from '@/components/tailwind/List'

interface RuleSequenceListProps {
  prependSeq: string[]
  ruleList: string[]
  appendSeq: string[]
  deleteSeq: string[]
  setPrependSeq: Dispatch<SetStateAction<string[]>>
  setAppendSeq: Dispatch<SetStateAction<string[]>>
  setDeleteSeq: Dispatch<SetStateAction<string[]>>
}

const reorder = (list: string[], startIndex: number, endIndex: number) => {
  const result = Array.from(list)
  const [removed] = result.splice(startIndex, 1)
  result.splice(endIndex, 0, removed)
  return result
}

export function RuleSequenceList({
  prependSeq,
  ruleList,
  appendSeq,
  deleteSeq,
  setPrependSeq,
  setAppendSeq,
  setDeleteSeq,
}: RuleSequenceListProps) {
  const [match, setMatch] = useState(() => (_: string) => true)

  const filteredPrependSeq = useMemo(
    () => prependSeq.filter((rule) => match(rule)),
    [prependSeq, match],
  )
  const filteredRuleList = useMemo(
    () => ruleList.filter((rule) => match(rule)),
    [ruleList, match],
  )
  const filteredAppendSeq = useMemo(
    () => appendSeq.filter((rule) => match(rule)),
    [appendSeq, match],
  )

  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 8 },
    }),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  )

  const onPrependDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event
    if (over && active.id !== over.id) {
      const activeIndex = prependSeq.indexOf(active.id.toString())
      const overIndex = prependSeq.indexOf(over.id.toString())
      setPrependSeq(reorder(prependSeq, activeIndex, overIndex))
    }
  }

  const onAppendDragEnd = async (event: DragEndEvent) => {
    const { active, over } = event
    if (over && active.id !== over.id) {
      const activeIndex = appendSeq.indexOf(active.id.toString())
      const overIndex = appendSeq.indexOf(over.id.toString())
      setAppendSeq(reorder(appendSeq, activeIndex, overIndex))
    }
  }

  const renderItem = (index: number): ReactNode => {
    const shift = filteredPrependSeq.length > 0 ? 1 : 0

    if (filteredPrependSeq.length > 0 && index === 0) {
      return (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={onPrependDragEnd}
        >
          <SortableContext items={filteredPrependSeq}>
            {filteredPrependSeq.map((item) => (
              <RuleItem
                key={item}
                type="prepend"
                ruleRaw={item}
                onDelete={() => {
                  setPrependSeq((current) => current.filter((value) => value !== item))
                }}
              />
            ))}
          </SortableContext>
        </DndContext>
      )
    }

    if (index < filteredRuleList.length + shift) {
      const currentRule = filteredRuleList[index - shift]
      return (
        <RuleItem
          key={currentRule}
          type={deleteSeq.includes(currentRule) ? 'delete' : 'original'}
          ruleRaw={currentRule}
          onDelete={() => {
            setDeleteSeq((current) =>
              current.includes(currentRule)
                ? current.filter((value) => value !== currentRule)
                : [...current, currentRule],
            )
          }}
        />
      )
    }

    return (
      <DndContext
        sensors={sensors}
        collisionDetection={closestCenter}
        onDragEnd={onAppendDragEnd}
      >
        <SortableContext items={filteredAppendSeq}>
          {filteredAppendSeq.map((item) => (
            <RuleItem
              key={item}
              type="append"
              ruleRaw={item}
              onDelete={() => {
                setAppendSeq((current) => current.filter((value) => value !== item))
              }}
            />
          ))}
        </SortableContext>
      </DndContext>
    )
  }

  return (
    <List className="w-1/2 px-2.5">
      <BaseSearchBox onSearch={(nextMatch) => setMatch(() => nextMatch)} />
      <VirtualList
        count={
          filteredRuleList.length +
          (filteredPrependSeq.length > 0 ? 1 : 0) +
          (filteredAppendSeq.length > 0 ? 1 : 0)
        }
        estimateSize={56}
        renderItem={renderItem}
        style={{ height: 'calc(100% - 24px)', marginTop: '8px' }}
      />
    </List>
  )
}
