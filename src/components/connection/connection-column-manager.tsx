import {
  closestCenter,
  DndContext,
  PointerSensor,
  useSensor,
  useSensors,
  type DragEndEvent,
} from '@dnd-kit/core'
import { arrayMove, SortableContext, useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import type { Column } from '@tanstack/react-table'
import { GripVertical } from 'lucide-react'
import { useCallback, useMemo } from 'react'
import { useTranslation } from 'react-i18next'

import { Button } from '@/components/tailwind/Button'
import { Checkbox } from '@/components/tailwind/Checkbox'
import { Dialog, DialogActions, DialogContent, DialogTitle } from '@/components/tailwind/Dialog'
import { IconButton } from '@/components/tailwind/IconButton'
import { List, ListItem, ListItemText } from '@/components/tailwind/List'
import { cn } from '@/utils/cn'

interface Props {
  open: boolean
  columns: Column<IConnectionsItem, unknown>[]
  onClose: () => void
  onOrderChange: (order: string[]) => void
  onReset: () => void
}

export const ConnectionColumnManager = ({
  open,
  columns,
  onClose,
  onOrderChange,
  onReset,
}: Props) => {
  const sensors = useSensors(
    useSensor(PointerSensor, {
      activationConstraint: { distance: 6 },
    }),
  )
  const { t } = useTranslation()

  const items = useMemo(() => columns.map((column) => column.id), [columns])
  const visibleCount = useMemo(
    () => columns.filter((column) => column.getIsVisible()).length,
    [columns],
  )

  const handleDragEnd = useCallback(
    (event: DragEndEvent) => {
      const { active, over } = event
      if (!over || active.id === over.id) return

      const order = columns.map((column) => column.id)
      const oldIndex = order.indexOf(active.id as string)
      const newIndex = order.indexOf(over.id as string)
      if (oldIndex === -1 || newIndex === -1) return

      onOrderChange(arrayMove(order, oldIndex, newIndex))
    },
    [columns, onOrderChange],
  )

  return (
    <Dialog open={open} onClose={onClose} maxWidth="xs" fullWidth>
      <DialogTitle>
        {t('connections.components.columnManager.title')}
      </DialogTitle>
      <DialogContent className="pt-2">
        <DndContext
          sensors={sensors}
          collisionDetection={closestCenter}
          onDragEnd={handleDragEnd}
        >
          <SortableContext items={items}>
            <List disablePadding className="flex flex-col gap-2">
              {columns.map((column) => (
                <SortableColumnItem
                  key={column.id}
                  column={column}
                  label={getColumnLabel(column)}
                  dragHandleLabel={t(
                    'connections.components.columnManager.dragHandle',
                  )}
                  disableToggle={
                    !column.getCanHide() ||
                    (column.getIsVisible() && visibleCount <= 1)
                  }
                />
              ))}
            </List>
          </SortableContext>
        </DndContext>
      </DialogContent>
      <DialogActions className="px-6 pb-4">
        <Button variant="text" onClick={onReset}>
          {t('shared.actions.resetToDefault')}
        </Button>
        <Button variant="contained" onClick={onClose}>
          {t('shared.actions.close')}
        </Button>
      </DialogActions>
    </Dialog>
  )
}

interface SortableColumnItemProps {
  column: Column<IConnectionsItem, unknown>
  label: string
  dragHandleLabel: string
  disableToggle?: boolean
}

const SortableColumnItem = ({
  column,
  label,
  dragHandleLabel,
  disableToggle = false,
}: SortableColumnItemProps) => {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: column.id })

  const style = useMemo(
    () => ({
      transform: CSS.Transform.toString(transform),
      transition,
    }),
    [transform, transition],
  )

  return (
    <ListItem
      ref={setNodeRef}
      className={cn(
        'px-2 py-1 rounded border border-divider flex items-center gap-2',
        isDragging ? 'bg-action-hover' : 'bg-transparent'
      )}
      style={style}
    >
      <Checkbox
        checked={column.getIsVisible()}
        disabled={disableToggle}
        onChange={(event) => column.toggleVisibility(event.target.checked)}
      />
      <ListItemText
        primary={label}
        className="mr-2 text-sm"
      />
      <IconButton
        size="small"
        className={cn(isDragging ? 'cursor-grabbing' : 'cursor-grab')}
        aria-label={dragHandleLabel}
        {...attributes}
        {...listeners}
      >
        <GripVertical className="h-4 w-4" />
      </IconButton>
    </ListItem>
  )
}

const getColumnLabel = (column: Column<IConnectionsItem, unknown>) => {
  const meta = column.columnDef.meta as { label?: string } | undefined
  if (meta?.label) return meta.label

  const header = column.columnDef.header
  return typeof header === 'string' ? header : column.id
}
