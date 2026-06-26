import type { Proxy, ProxyProvider } from 'clash-dtos'

type ProxyViewBase = Partial<
  Omit<Proxy, 'all' | 'history' | 'name' | 'type'>
>

export interface IProxyItem extends ProxyViewBase {
  name: string
  type: Proxy['type'] | string
  udp: boolean
  xudp: boolean
  tfo: boolean
  mptcp: boolean
  smux: boolean
  history: Proxy['history']
  testUrl?: string
  all?: string[]
  now?: string
  hidden?: boolean
  icon?: string
  provider?: string
  fixed?: string
}

export type IProxyGroupItem = Omit<IProxyItem, 'all'> & {
  all: IProxyItem[]
}

export interface IProxyProviderItem
  extends Omit<
    ProxyProvider,
    'proxies' | 'subscriptionInfo' | 'type' | 'updatedAt' | 'vehicleType'
  > {
  name: string
  type: ProxyProvider['type'] | string
  proxies: IProxyItem[]
  updatedAt: string | null
  vehicleType: ProxyProvider['vehicleType'] | string
  subscriptionInfo?: ProxyProvider['subscriptionInfo']
}
