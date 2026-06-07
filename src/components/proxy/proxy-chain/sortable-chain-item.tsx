import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { GripVertical, Trash2 } from 'lucide-react'

import { Chip } from '@/components/tailwind/Chip'
import { IconButton } from '@/components/tailwind/IconButton'

import type { ProxyChainItem } from '../proxy-chain-types'

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

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  }

  const roleLabel = isFirst ? entryLabel : isLast ? exitLabel : undefined
  const borderClass = isFirst
    ? 'border-2 border-green-500'
    : isLast
      ? 'border-2 border-orange-500'
      : 'border border-divider'

  return (
    <div
      ref={setNodeRef}
      style={style}
      className={`mb-0 flex items-center rounded p-2 transition-all duration-200 ${
        isDragging ? 'bg-background shadow-lg' : 'bg-card shadow'
      } ${borderClass}`}
    >
      <div
        {...attributes}
        {...listeners}
        className="mr-2 flex cursor-grab items-center text-text-secondary active:cursor-grabbing"
      >
        <GripVertical className="h-5 w-5" />
      </div>

      {roleLabel ? (
        <Chip
          label={roleLabel}
          size="small"
          className={`mr-2 font-bold text-white ${
            isFirst ? 'bg-green-500' : 'bg-orange-500'
          }`}
        />
      ) : (
        <Chip
          label={`${index + 1}`}
          size="small"
          color="primary"
          className="mr-2 min-w-[32px]"
        />
      )}

      <span className="flex-1 overflow-hidden text-ellipsis whitespace-nowrap text-sm font-medium">
        {proxy.name}
      </span>

      {proxy.type && (
        <Chip
          label={proxy.type}
          size="small"
          variant="outlined"
          className="mr-2"
        />
      )}

      {proxy.delay !== undefined && (
        <Chip
          label={proxy.delay > 0 ? `${proxy.delay}ms` : timeoutLabel}
          size="small"
          color={
            proxy.delay > 0 && proxy.delay < 200
              ? 'success'
              : proxy.delay > 0 && proxy.delay < 800
                ? 'warning'
                : 'error'
          }
          className="mr-2 min-w-[50px] text-xs"
        />
      )}

      <IconButton
        size="small"
        onClick={() => onRemove(proxy.id)}
        className="text-red-500 hover:bg-red-500/10"
      >
        <Trash2 className="h-4 w-4" />
      </IconButton>
    </div>
  )
}
