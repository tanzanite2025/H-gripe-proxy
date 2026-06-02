export interface ProxyChainItem {
  id: string
  name: string
  type?: string
  delay?: number
}

export interface ProxyChainRuntimeIntent {
  targetGroup: string
  exitNode: string
  runtimePayload: string[]
}

export const PROXY_CHAIN_STORAGE_KEYS = {
  group: 'proxy-chain-group',
  exitNode: 'proxy-chain-exit-node',
  items: 'proxy-chain-items',
} as const

export const proxyChainNodeNames = (chain: ProxyChainItem[]): string[] =>
  chain.map((node) => node.name)

export const proxyChainEntryNode = (
  chain: ProxyChainItem[],
): ProxyChainItem | null => chain[0] ?? null

export const proxyChainExitNode = (
  chain: ProxyChainItem[],
): ProxyChainItem | null => chain[chain.length - 1] ?? null

export const proxyChainTargetGroup = (
  mode: string | undefined,
  selectedGroup: string | null | undefined,
  fallbackGroup?: string | null,
): string | null => {
  if (mode === 'global') return 'GLOBAL'
  return selectedGroup || fallbackGroup || null
}

export const loadProxyChainRuntimeGroup = (): string | null =>
  localStorage.getItem(PROXY_CHAIN_STORAGE_KEYS.group)

export const loadProxyChainRuntimeExitNode = (): string | null =>
  localStorage.getItem(PROXY_CHAIN_STORAGE_KEYS.exitNode)

export const buildProxyChainRuntimeIntent = (
  chain: ProxyChainItem[],
  mode: string | undefined,
  selectedGroup: string | null | undefined,
): ProxyChainRuntimeIntent | null => {
  const exitNode = proxyChainExitNode(chain)
  const targetGroup = proxyChainTargetGroup(mode, selectedGroup)

  if (!exitNode || !targetGroup || chain.length < 2) {
    return null
  }

  return {
    targetGroup,
    exitNode: exitNode.name,
    runtimePayload: proxyChainNodeNames(chain),
  }
}

export const isProxyChainConnected = (
  proxies: any,
  chain: ProxyChainItem[],
  mode: string | undefined,
  selectedGroup: string | null | undefined,
): boolean => {
  const exitNode = proxyChainExitNode(chain)
  if (!proxies || !exitNode || chain.length < 2) return false

  if (mode === 'global') {
    return proxies.global?.now === exitNode.name
  }

  if (!selectedGroup || !Array.isArray(proxies.groups)) {
    return false
  }

  const proxyChainGroup = proxies.groups.find(
    (group: { name: string }) => group.name === selectedGroup,
  )

  return proxyChainGroup?.now === exitNode.name
}

export const loadProxyChainStorage = (): ProxyChainItem[] => {
  try {
    const saved = localStorage.getItem(PROXY_CHAIN_STORAGE_KEYS.items)
    if (saved) return JSON.parse(saved)
  } catch {
    // ignore invalid persisted chain data
  }
  return []
}

export const saveProxyChainStorage = (chain: ProxyChainItem[]) => {
  if (chain.length > 0) {
    localStorage.setItem(PROXY_CHAIN_STORAGE_KEYS.items, JSON.stringify(chain))
  } else {
    localStorage.removeItem(PROXY_CHAIN_STORAGE_KEYS.items)
  }
}

export const saveProxyChainRuntimeSelection = (
  targetGroup: string,
  exitNode: string,
) => {
  localStorage.setItem(PROXY_CHAIN_STORAGE_KEYS.group, targetGroup)
  localStorage.setItem(PROXY_CHAIN_STORAGE_KEYS.exitNode, exitNode)
}

export const clearProxyChainStorage = () => {
  localStorage.removeItem(PROXY_CHAIN_STORAGE_KEYS.group)
  localStorage.removeItem(PROXY_CHAIN_STORAGE_KEYS.exitNode)
  localStorage.removeItem(PROXY_CHAIN_STORAGE_KEYS.items)
}
