import { useTranslation } from 'react-i18next'

import { Menu } from '@/components/tailwind/Menu'
import { MenuItem } from '@/components/tailwind/MenuItem'

import type { ContextMenuItem } from './shared'

interface ProfileItemMenuProps {
  open: boolean
  position: {
    left: number
    top: number
  }
  items: ContextMenuItem[]
  onClose: () => void
}

export function ProfileItemMenu({
  open,
  position,
  items,
  onClose,
}: ProfileItemMenuProps) {
  const { t } = useTranslation()

  return (
    <Menu
      open={open}
      onClose={onClose}
      anchorPosition={position}
      anchorReference="anchorPosition"
      onContextMenu={(event) => {
        onClose()
        event.preventDefault()
      }}
    >
      {items.map((item) => (
        <MenuItem
          key={item.label}
          onClick={item.handler}
          disabled={item.disabled}
          className={item.destructive ? 'text-error' : undefined}
        >
          {t(item.label)}
        </MenuItem>
      ))}
    </Menu>
  )
}
