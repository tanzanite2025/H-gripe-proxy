import type {
  DraggableAttributes,
  DraggableSyntheticListeners,
} from '@dnd-kit/core'
import { ListItem, ListItemButton } from '@/components/tailwind'
import type { CSSProperties, ReactNode } from 'react'
import { useMatch, useNavigate, useResolvedPath } from 'react-router'

import { useVerge } from '@/hooks/system'

interface SortableProps {
  setNodeRef?: (element: HTMLElement | null) => void
  attributes?: DraggableAttributes
  listeners?: DraggableSyntheticListeners
  style?: CSSProperties
  isDragging?: boolean
  disabled?: boolean
}

interface Props {
  to: string
  children: string
  icon: ReactNode[]
  sortable?: SortableProps
}
export const LayoutItem = (props: Props) => {
  const { to, children, sortable } = props
  const { verge } = useVerge()
  const navCollapsed = verge?.collapse_navbar ?? false
  const resolved = useResolvedPath(to)
  const match = useMatch({ path: resolved.pathname, end: true })
  const navigate = useNavigate()

  const { setNodeRef, attributes, listeners, style, isDragging, disabled } =
    sortable ?? {}

  const draggable = Boolean(sortable) && !disabled
  const dragHandleProps = draggable
    ? { ...(attributes ?? {}), ...(listeners ?? {}) }
    : undefined

  const itemClassName = isDragging ? 'layout-nav-item is-dragging' : 'layout-nav-item'
  const buttonClassName = `layout-nav-item__button${match ? ' is-active' : ''}${draggable ? ' is-draggable' : ''}`

  return (
    <ListItem
      ref={setNodeRef}
      style={style}
      className={itemClassName}
    >
      <ListItemButton
        {...(dragHandleProps ?? {})}
        className={buttonClassName}
        title={navCollapsed ? children : undefined}
        aria-label={navCollapsed ? children : undefined}
        onClick={() => navigate(to)}
      >
        <div className="layout-nav-item__text">
          <span className="layout-nav-item__primary">{children}</span>
        </div>
      </ListItemButton>
    </ListItem>
  )
}
