import { Menu as HeadlessMenu, Transition } from '@headlessui/react'
import { Fragment, type ReactNode } from 'react'

export interface MenuProps {
  trigger: ReactNode
  children: ReactNode
}

export const Menu = ({ trigger, children }: MenuProps) => {
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
  children: ReactNode
}

export const MenuItem = ({ onClick, disabled = false, children }: MenuItemProps) => {
  return (
    <HeadlessMenu.Item disabled={disabled}>
      {({ active }) => (
        <button
          onClick={onClick}
          className={`${
            active ? 'bg-gray-100 dark:bg-gray-800' : ''
          } ${
            disabled ? 'cursor-not-allowed opacity-50' : ''
          } group flex w-full items-center px-4 py-2 text-sm text-gray-900 dark:text-gray-100 transition-colors`}
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
