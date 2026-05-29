import { Menu as HeadlessMenu, Transition } from '@headlessui/react'
import { Fragment, type CSSProperties, type MouseEventHandler, type ReactNode } from 'react'

import { cn } from '@/utils/cn'

export interface MenuProps {
  trigger?: ReactNode
  children: ReactNode
  open?: boolean
  onClose?: () => void
  anchorEl?: HTMLElement | null
  anchorPosition?: { top: number; left: number }
  anchorReference?: 'anchorEl' | 'anchorPosition' | string
  onContextMenu?: MouseEventHandler<HTMLDivElement>
  className?: string
}

export const Menu = ({ trigger, children, open, onClose, anchorEl: _anchorEl, anchorPosition, anchorReference, onContextMenu, className }: MenuProps) => {
  // If using controlled open state (MUI-style API)
  if (open !== undefined && onClose) {
    if (!open) return null

    const anchoredToPosition = anchorReference === 'anchorPosition' && anchorPosition
    const menuStyle: CSSProperties | undefined = anchoredToPosition
      ? {
          position: 'fixed',
          top: anchorPosition.top,
          left: anchorPosition.left,
        }
      : undefined
    
    return (
      <div className="relative inline-block text-left" onContextMenu={onContextMenu}>
        <div
          style={menuStyle}
          className={cn(
            anchoredToPosition
              ? 'z-10 w-56 origin-top-left rounded-input bg-card-light dark:bg-card-dark shadow-lg ring-1 ring-black/5 dark:ring-white/5 focus:outline-none'
              : 'absolute right-0 z-10 mt-2 w-56 origin-top-right rounded-input bg-card-light dark:bg-card-dark shadow-lg ring-1 ring-black/5 dark:ring-white/5 focus:outline-none',
            className,
          )}
        >
          <div className="py-1">{children}</div>
        </div>
      </div>
    )
  }

  // Original Headless UI API
  if (!trigger) return null

  return (
    <HeadlessMenu as="div" className="relative inline-block text-left">
      <HeadlessMenu.Button as={Fragment}>{trigger}</HeadlessMenu.Button>

      <Transition
        as={Fragment}
        enter="transition ease-out duration-100"
        enterFrom="transform opacity-0 scale-95"
        enterTo="transform opacity-100 scale-100"
        leave="transition ease-in duration-75"
        leaveFrom="transform opacity-100 scale-100"
        leaveTo="transform opacity-0 scale-95"
      >
        <HeadlessMenu.Items className="absolute right-0 z-10 mt-2 w-56 origin-top-right rounded-input bg-card-light dark:bg-card-dark shadow-lg ring-1 ring-black/5 dark:ring-white/5 focus:outline-none">
          <div className="py-1">{children}</div>
        </HeadlessMenu.Items>
      </Transition>
    </HeadlessMenu>
  )
}

export interface MenuItemProps {
  onClick?: () => void
  disabled?: boolean
  value?: string | number
  children: ReactNode
  className?: string
  selected?: boolean
}

export const MenuItem = ({ onClick, disabled = false, value, children, className, selected = false }: MenuItemProps) => {
  return (
    <HeadlessMenu.Item disabled={disabled}>
      {({ active }) => (
        <button
          type="button"
          onClick={onClick}
          data-value={value}
          className={cn(
            active || selected ? 'bg-gray-100 dark:bg-gray-800' : '',
            disabled ? 'cursor-not-allowed opacity-50' : '',
            'group flex w-full items-center px-4 py-2 text-sm text-gray-900 dark:text-gray-100 transition-colors',
            className,
          )}
          disabled={disabled}
        >
          {children}
        </button>
      )}
    </HeadlessMenu.Item>
  )
}

export const MenuDivider = () => {
  return <div className="my-1 h-px bg-gray-200 dark:bg-gray-700" />
}

Menu.Item = MenuItem
Menu.Divider = MenuDivider
Menu.displayName = 'Menu'
