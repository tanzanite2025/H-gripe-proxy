import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { DeleteForeverRounded, UndoRounded } from '@mui/icons-material'

import { IconButton } from '@/components/tailwind/IconButton'
import { ListItem, ListItemText } from '@/components/tailwind/List'

interface Props {
  type: 'prepend' | 'original' | 'delete' | 'append'
  ruleRaw: string
  onDelete: () => void
}

export const RuleItem = (props: Props) => {
  const { type, ruleRaw, onDelete } = props
  const sortable = type === 'prepend' || type === 'append'
  const rule = ruleRaw.replace(',no-resolve', '')

  const ruleType = rule.match(/^[^,]+/)?.[0] ?? ''
  const proxyPolicy = rule.match(/[^,]+$/)?.[0] ?? ''
  const ruleContent = rule.slice(ruleType.length + 1, -proxyPolicy.length - 1)

  const $sortable = useSortable({ id: ruleRaw })

  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = sortable
    ? $sortable
    : {
        attributes: {},
        listeners: {},
        setNodeRef: null,
        transform: null,
        transition: null,
        isDragging: false,
      }

  const getBackgroundClass = () => {
    if (type === 'original') {
      return 'bg-gray-400/30 dark:bg-gray-800/30'
    }
    if (type === 'delete') {
      return 'bg-red-500/30'
    }
    return 'bg-green-500/30'
  }

  return (
    <ListItem
      className={`relative h-full my-2 rounded-lg ${getBackgroundClass()}`}
      style={{
        transform: CSS.Transform.toString(transform),
        transition,
        zIndex: isDragging ? 9999 : undefined,
      }}
    >
      <ListItemText
        {...attributes}
        {...listeners}
        ref={setNodeRef}
        className={sortable ? 'cursor-move' : ''}
        primary={
          <div
            title={ruleContent || '-'}
            className={`text-[15px] font-bold leading-6 overflow-hidden text-ellipsis whitespace-nowrap ${
              type === 'delete' ? 'line-through' : ''
            }`}
          >
            {ruleContent || '-'}
          </div>
        }
        secondary={
          <div className="w-[62%] overflow-hidden flex justify-between pt-0.5">
            <div className="mt-0.5">
              <span className="inline-block border border-primary/50 text-primary/80 rounded text-[10px] px-1 leading-6 mr-2">
                {ruleType}
              </span>
            </div>
            <span className="text-[13px] overflow-hidden text-gray-400 dark:text-gray-500 text-ellipsis whitespace-nowrap">
              {proxyPolicy}
            </span>
          </div>
        }
        secondaryClassName="flex items-center text-gray-400"
      />
      <IconButton onClick={onDelete}>
        {type === 'delete' ? <UndoRounded /> : <DeleteForeverRounded />}
      </IconButton>
    </ListItem>
  )
}
