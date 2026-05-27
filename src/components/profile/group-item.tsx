import { useSortable } from '@dnd-kit/sortable'
import { CSS } from '@dnd-kit/utilities'
import { DeleteForeverRounded, UndoRounded } from '@mui/icons-material'

import { IconButton } from '@/components/tailwind/IconButton'
import { ListItem, ListItemText } from '@/components/tailwind/List'
import { useIconCache } from '@/hooks/system'

interface Props {
  type: 'prepend' | 'original' | 'delete' | 'append'
  group: IProxyGroupConfig
  onDelete: () => void
}

export const GroupItem = (props: Props) => {
  const { type, group, onDelete } = props
  const sortable = type === 'prepend' || type === 'append'

  const {
    attributes: sortableAttributes,
    listeners: sortableListeners,
    setNodeRef: sortableSetNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({
    id: group.name,
    disabled: !sortable,
  })
  const dragAttributes = sortable ? sortableAttributes : undefined
  const dragListeners = sortable ? sortableListeners : undefined
  const dragNodeRef = sortable ? sortableSetNodeRef : undefined

  const iconCachePath = useIconCache({
    icon: group.icon,
    cacheKey: group.name.replaceAll(' ', ''),
  })

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
      {group.icon && group.icon?.trim().startsWith('http') && (
        <img
          src={iconCachePath === '' ? group.icon : iconCachePath}
          width="32px"
          className="mr-3 rounded-md"
        />
      )}
      {group.icon && group.icon?.trim().startsWith('data') && (
        <img src={group.icon} width="32px" className="mr-3 rounded-md" />
      )}
      {group.icon && group.icon?.trim().startsWith('<svg') && (
        <img
          src={`data:image/svg+xml;base64,${btoa(group.icon ?? '')}`}
          width="32px"
          className="mr-3 rounded-md"
        />
      )}
      <ListItemText
        {...(dragAttributes ?? {})}
        {...(dragListeners ?? {})}
        ref={dragNodeRef}
        className={sortable ? 'cursor-move' : ''}
        primary={
          <div
            className={`text-[15px] font-bold leading-6 overflow-hidden text-ellipsis whitespace-nowrap ${
              type === 'delete' ? 'line-through' : ''
            }`}
          >
            {group.name}
          </div>
        }
        secondary={
          <div className="overflow-hidden flex items-center pt-0.5">
            <div className="mt-0.5">
              <span className="inline-block border border-primary/50 text-primary/80 rounded text-[10px] px-1 leading-6 mr-2">
                {group.type}
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
