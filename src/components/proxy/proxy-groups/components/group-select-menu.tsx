import { Box, Menu, MenuItem, Typography } from '@mui/material'

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
      slotProps={{
        paper: {
          sx: {
            maxHeight: 300,
            minWidth: 200,
          },
        },
      }}
    >
      {groups.map((group) => (
        <MenuItem
          key={group.name}
          onClick={() => onSelect(group.name)}
          selected={selectedGroup === group.name}
          sx={{ fontSize: '14px', py: 1 }}
        >
          <Box
            sx={{
              display: 'flex',
              flexDirection: 'column',
              alignItems: 'flex-start',
            }}
          >
            <Typography variant="body2" sx={{ fontWeight: 500 }}>
              {group.name}
            </Typography>
            <Typography variant="caption" color="text.secondary">
              {group.type} · {group.all?.length ?? 0} 节点
            </Typography>
          </Box>
        </MenuItem>
      ))}

      {groups.length === 0 && (
        <MenuItem disabled>
          <Typography variant="body2" color="text.secondary">
            {emptyText}
          </Typography>
        </MenuItem>
      )}
    </Menu>
  )
}
