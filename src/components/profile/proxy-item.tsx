import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { DeleteForeverRounded, UndoRounded } from '@mui/icons-material'

import { IconButton } from '@/components/tailwind/IconButton'
import { ListItem, ListItemText } from '@/components/tailwind/List'

interface Props {
  type: 'prepend' | 'original' | 'delete' | 'append'
  proxy: IProxyConfig
  onDelete: () => void
}

export const ProxyItem = (props: Props) => {
  const { type, proxy, onDelete } = props
  const sortable = type === 'prepend' || type === 'append'

  const {
    attributes: sortableAttributes,
    listeners: sortableListeners,
    setNodeRef: sortableSetNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id: proxy.name,
    disabled: !sortable,
  })
  const dragAttributes = sortable ? sortableAttributes : undefined
  const dragListeners = sortable ? sortableListeners : undefined
  const dragNodeRef = sortable ? sortableSetNodeRef : undefined

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
        {...(dragAttributes ?? {})}
        {...(dragListeners ?? {})}
        ref={dragNodeRef}
        className={sortable ? 'cursor-move' : ''}
        primary={
          <div
            title={proxy.name}
            className={`text-[15px] font-bold leading-6 overflow-hidden text-ellipsis whitespace-nowrap ${
              type === 'delete' ? 'line-through' : ''
            }`}
          >
            {proxy.name}
          </div>
        }
        secondary={
          <div className="overflow-hidden flex items-center pt-0.5">
            <div className="mt-0.5">
              <span className="inline-block border border-primary/50 text-primary/80 rounded text-[10px] px-1 leading-6 mr-2">
                {proxy.type}
              </span>
            </div>
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
