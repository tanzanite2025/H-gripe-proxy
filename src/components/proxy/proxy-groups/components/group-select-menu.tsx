import { Menu, MenuItem } from '@/components/tailwind'

interface ProxyGroupOption {
  name: string
  type: string
  all?: unknown[]
}

interface GroupSelectMenuProps {
  anchorEl: HTMLElement | null
  groups: ProxyGroupOption[]
  selectedGroup: string | null
  emptyText: string
  onClose: () => void
  onSelect: (groupName: string) => void
}

/**
 * 代理组选择菜单组件
 */
export function GroupSelectMenu({
  anchorEl,
  groups,
  selectedGroup,
  emptyText,
  onClose,
  onSelect,
}: GroupSelectMenuProps) {
  return (
    <Menu
      anchorEl={anchorEl}
      open={Boolean(anchorEl)}
      onClose={onClose}
      className="max-h-[300px] min-w-[200px]"
    >
      {groups.map((group) => (
        <MenuItem
          key={group.name}
          onClick={() => onSelect(group.name)}
          selected={selectedGroup === group.name}
          className="py-2 text-sm"
        >
          <div className="flex flex-col items-start">
            <span className="font-medium text-sm">{group.name}</span>
            <span className="text-xs text-gray-500">
              {group.type} · {group.all?.length ?? 0} 节点
            </span>
          </div>
        </MenuItem>
      ))}

      {groups.length === 0 && (
        <MenuItem disabled>
          <span className="text-sm text-gray-500">{emptyText}</span>
        </MenuItem>
      )}
    </Menu>
  )
}
