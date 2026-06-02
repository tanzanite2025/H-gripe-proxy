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

  const proxyRecord = proxyResponse.proxies
  const providerRecord = providerResponse

  const providerMap = Object.fromEntries(
    Object.entries(providerRecord).flatMap(([provider, item]) =>
      item!.proxies.map((proxy) => [proxy.name, { ...proxy, provider }]),
    ),
  )

  const generateItem = (name: string) => {
    if (proxyRecord[name]) return proxyRecord[name]
    if (providerMap[name]) return providerMap[name]
    return {
      name,
      type: 'unknown',
      udp: false,
      xudp: false,
      tfo: false,
      mptcp: false,
      smux: false,
      history: [],
    }
  }

  const { GLOBAL: global, DIRECT: direct, REJECT: reject } = proxyRecord

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

  const proxies = [direct, reject].concat(
    Object.values(proxyRecord).filter(
      (proxy) =>
        !proxy?.all?.length && proxy?.name !== 'DIRECT' && proxy?.name !== 'REJECT',
    ),
  )

  const runtimeGlobal = {
    ...global,
    all: global?.all?.map((item) => generateItem(item)) || [],
  }

  return {
    global: runtimeGlobal as IProxyGroupItem,
    direct: direct as IProxyItem,
    groups,
    records: proxyRecord as Record<string, IProxyItem>,
    proxies: (proxies as IProxyItem[]) ?? [],
  }
}
