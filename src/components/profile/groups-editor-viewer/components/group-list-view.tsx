import { DndContext, DragEndEvent, closestCenter } from '@dnd-kit/core'
import { SortableContext } from '@dnd-kit/sortable'
import { List } from '@mui/material'
import { useMemo } from 'react'

import { VirtualList } from '@/components/base'
import { GroupItem } from '@/components/profile/group-item'

interface GroupListViewProps {
  prependSeq: IProxyGroupConfig[]
  groupList: IProxyGroupConfig[]
  appendSeq: IProxyGroupConfig[]
  deleteSeq: string[]
  match: (name: string) => boolean
  sensors: any
  onPrependDragEnd: (event: DragEndEvent) => void
  onAppendDragEnd: (event: DragEndEvent) => void
  onPrependDelete: (name: string) => void
  onAppendDelete: (name: string) => void
  onGroupToggleDelete: (name: string) => void
}

export const GroupListView = ({
  prependSeq,
  groupList,
  appendSeq,
  deleteSeq,
  match,
  sensors,
  onPrependDragEnd,
  onAppendDragEnd,
  onPrependDelete,
  onAppendDelete,
  onGroupToggleDelete,
}: GroupListViewProps) => {
  const filteredPrependSeq = useMemo(
    () => prependSeq.filter((group) => match(group.name)),
    [prependSeq, match],
  )
  const filteredGroupList = useMemo(
    () => groupList.filter((group) => match(group.name)),
    [groupList, match],
  )
  const filteredAppendSeq = useMemo(
    () => appendSeq.filter((group) => match(group.name)),
    [appendSeq, match],
  )

  const renderItem = (index: number): React.ReactNode => {
    const shift = filteredPrependSeq.length > 0 ? 1 : 0
    if (filteredPrependSeq.length > 0 && index === 0) {
      return (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={onPrependDragEnd}
        >
          <SortableContext
            items={filteredPrependSeq.map((x) => {
              return x.name
            })}
          >
            {filteredPrependSeq.map((item) => {
              return (
                <GroupItem
                  key={item.name}
                  type="prepend"
                  group={item}
                  onDelete={() => onPrependDelete(item.name)}
                />
              )
            })}
          </SortableContext>
        </DndContext>
      )
    } else if (index < filteredGroupList.length + shift) {
      const newIndex = index - shift
      return (
        <GroupItem
          key={filteredGroupList[newIndex].name}
          type={
            deleteSeq.includes(filteredGroupList[newIndex].name)
              ? 'delete'
              : 'original'
          }
          group={filteredGroupList[newIndex]}
          onDelete={() => onGroupToggleDelete(filteredGroupList[newIndex].name)}
        />
      )
    } else {
      return (
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={onAppendDragEnd}
        >
          <SortableContext
            items={filteredAppendSeq.map((x) => {
              return x.name
            })}
          >
            {filteredAppendSeq.map((item) => {
              return (
                <GroupItem
                  key={item.name}
                  type="append"
                  group={item}
                  onDelete={() => onAppendDelete(item.name)}
                />
              )
            })}
          </SortableContext>
        </DndContext>
      )
    }
  }

  return (
    <List
      sx={{
        width: '50%',
        padding: '0 10px',
      }}
    >
      <VirtualList
        count={
          filteredGroupList.length +
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
