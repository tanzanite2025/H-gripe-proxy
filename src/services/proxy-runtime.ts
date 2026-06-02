import { getProxies, getProxyProviders } from 'tauri-plugin-mihomo-api'

export interface CalculatedProxies {
  global: IProxyGroupItem
  direct: IProxyItem
  groups: IProxyGroupItem[]
  records: Record<string, IProxyItem>
  proxies: IProxyItem[]
}

export async function calcuProxyProviders() {
  const providers = await getProxyProviders()
  return Object.fromEntries(
    Object.entries(providers.providers)
      .sort()
      .filter(
        ([_, item]) =>
          item?.vehicleType === 'HTTP' || item?.vehicleType === 'File',
      ),
  )
}

export async function calcuProxies(): Promise<CalculatedProxies> {
  const [proxyResponse, providerResponse] = await Promise.all([
    getProxies(),
    calcuProxyProviders(),
  ])

  const proxyRecord = proxyResponse.proxies as unknown as Record<
    string,
    IProxyItem | undefined
  >
  const providerRecord = providerResponse

  const providerMap = Object.fromEntries(
    Object.entries(providerRecord).flatMap(([provider, item]) =>
      item!.proxies.map((proxy) => [proxy.name, { ...proxy, provider }]),
    ),
  )

  const fallbackProxy = (name: string, type = 'unknown'): IProxyItem => ({
    name,
    type,
    udp: false,
    xudp: false,
    tfo: false,
    mptcp: false,
    smux: false,
    history: [],
  })

  const generateItem = (name: string): IProxyItem => {
    if (proxyRecord[name]) return proxyRecord[name]
    if (providerMap[name]) return providerMap[name]
    return fallbackProxy(name)
  }

  const { GLOBAL: global, DIRECT: direct, REJECT: reject } = proxyRecord
  const rawProxies = Object.values(proxyRecord).filter(
    (proxy): proxy is IProxyItem => Boolean(proxy),
  )
  const directItem = direct ?? fallbackProxy('DIRECT', 'Direct')
  const rejectItem = reject ?? fallbackProxy('REJECT', 'Reject')

  let groups: IProxyGroupItem[] = Object.values(proxyRecord).reduce<
    IProxyGroupItem[]
  >((acc, each) => {
    if (each?.name !== 'GLOBAL' && each?.all) {
      acc.push({
        ...each,
        all: each.all!.map((item) => generateItem(item)),
      })
    }

    return acc
  }, [])

  if (global?.all) {
    const globalGroups: IProxyGroupItem[] = global.all.reduce<
      IProxyGroupItem[]
    >((acc, name) => {
      if (proxyRecord[name]?.all) {
        acc.push({
          ...proxyRecord[name],
          all: proxyRecord[name].all!.map((item) => generateItem(item)),
        })
      }
      return acc
    }, [])

    const globalNames = new Set(globalGroups.map((each) => each.name))
    groups = groups
      .filter((group) => !globalNames.has(group.name))
      .concat(globalGroups)
  }

  const proxies = [directItem, rejectItem].concat(
    rawProxies.filter(
      (proxy) =>
        !proxy?.all?.length && proxy?.name !== 'DIRECT' && proxy?.name !== 'REJECT',
    ),
  )

  const fallbackGlobalNames = proxies
    .map((proxy) => proxy.name)
    .filter((name) => name !== 'REJECT')
  const globalAllNames = global?.all?.length ? global.all : fallbackGlobalNames
  const runtimeGlobal = {
    ...fallbackProxy('GLOBAL', 'Selector'),
    ...global,
    now: global?.now || globalAllNames[0] || 'DIRECT',
    all: globalAllNames.map((item) => generateItem(item)),
  }

  const records = {
    ...proxyRecord,
    DIRECT: proxyRecord.DIRECT ?? directItem,
    REJECT: proxyRecord.REJECT ?? rejectItem,
    GLOBAL: proxyRecord.GLOBAL ?? runtimeGlobal,
  }

  return {
    global: runtimeGlobal as IProxyGroupItem,
    direct: directItem as IProxyItem,
    groups,
    records: records as Record<string, IProxyItem>,
    proxies,
  }
}
