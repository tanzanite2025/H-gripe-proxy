import { getProxies, getProxyProviders } from 'tauri-plugin-mihomo-api'

import { isBuiltinPolicyName, isHiddenProxyName } from './proxy-display'

export interface CalculatedProxies {
  global: IProxyGroupItem
  groups: IProxyGroupItem[]
  records: Record<string, IProxyItem>
  proxies: IProxyItem[]
}

const isDisplayableProxyName = (name?: string | null) =>
  !isHiddenProxyName(name) && !isBuiltinPolicyName(name)

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
  const proxyResponse = await getProxies()
  const providerResponse = await calcuProxyProviders().catch((error) => {
    console.warn(
      '[calcuProxies] proxy providers unavailable, continue without provider metadata:',
      error,
    )
    return {}
  })

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

  const { GLOBAL: global } = proxyRecord
  const rawProxies = Object.values(proxyRecord).filter(
    (proxy): proxy is IProxyItem => Boolean(proxy),
  )

  let groups: IProxyGroupItem[] = Object.values(proxyRecord).reduce<
    IProxyGroupItem[]
  >((acc, each) => {
    if (each?.name !== 'GLOBAL' && each?.all) {
      acc.push({
        ...each,
        all: each.all!
          .map((item) => generateItem(item))
          .filter((item) => isDisplayableProxyName(item.name)),
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
          all: proxyRecord[name].all!
            .map((item) => generateItem(item))
            .filter((item) => isDisplayableProxyName(item.name)),
        })
      }
      return acc
    }, [])

    const globalNames = new Set(globalGroups.map((each) => each.name))
    groups = groups
      .filter((group) => !globalNames.has(group.name))
      .concat(globalGroups)
  }

  const proxies = rawProxies.filter(
    (proxy) =>
      !proxy?.all?.length &&
      isDisplayableProxyName(proxy?.name),
  )

  const fallbackGlobalNames = proxies.map((proxy) => proxy.name)
  const globalAllNames =
    global?.all?.filter((name) => isDisplayableProxyName(name))?.length
      ? global.all.filter((name) => isDisplayableProxyName(name))
      : fallbackGlobalNames
  const runtimeGlobal = {
    ...fallbackProxy('GLOBAL', 'Selector'),
    ...global,
    now:
      (global?.now && isDisplayableProxyName(global.now) ? global.now : '') ||
      globalAllNames[0] ||
      '',
    all: globalAllNames.map((item) => generateItem(item)),
  }

  const records = {
    ...proxyRecord,
    GLOBAL: proxyRecord.GLOBAL ?? runtimeGlobal,
  }

  return {
    global: runtimeGlobal as IProxyGroupItem,
    groups,
    records: records as Record<string, IProxyItem>,
    proxies,
  }
}
