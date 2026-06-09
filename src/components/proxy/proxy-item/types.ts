export interface ProxyCardProps {
  group: IProxyGroupItem
  proxy: IProxyItem
  selected: boolean
  showType?: boolean
  clickable?: boolean
  onClick?: (name: string) => void
  onConfigure?: (group: IProxyGroupItem) => void
}
