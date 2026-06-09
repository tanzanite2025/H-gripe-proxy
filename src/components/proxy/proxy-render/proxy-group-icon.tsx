import { useIconCache } from '@/hooks/system'

interface ProxyGroupIconProps {
  group: IProxyGroupItem
}

export function ProxyGroupIcon({ group }: ProxyGroupIconProps) {
  const iconCachePath = useIconCache({
    icon: group.icon,
    cacheKey: group.name?.replaceAll(' ', '') || 'proxy-group',
    enabled: true,
  })

  if (!group.icon) {
    return null
  }

  if (group.icon.trim().startsWith('http')) {
    return (
      <img
        src={iconCachePath === '' ? group.icon : iconCachePath}
        width="32px"
        style={{ marginRight: '12px', borderRadius: '6px' }}
      />
    )
  }

  if (group.icon.trim().startsWith('data')) {
    return (
      <img
        src={group.icon}
        width="32px"
        style={{ marginRight: '12px', borderRadius: '6px' }}
      />
    )
  }

  if (group.icon.trim().startsWith('<svg')) {
    return (
      <img src={`data:image/svg+xml;base64,${btoa(group.icon)}`} width="32px" />
    )
  }

  return null
}
