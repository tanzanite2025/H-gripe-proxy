import {
  pickPreferredProxyNameFromNames,
  resolveProxyPath,
} from '@/services/proxy-display'

import { normalizePolicyName } from '../../utils/proxy-selection'

export type ProxySortType = 0 | 1 | 2

export type ProxyGroupOption = {
  name: string
  now: string
  all: string[]
  type?: string
  displayKind: 'manual' | 'strategy'
}

export interface ProxyOption {
  name: string
  kind: 'manual' | 'strategy'
}

export type CurrentProxyRecords = Record<string, any>

export interface CurrentProxySource {
  global?: any
  groups?: any[]
  records?: CurrentProxyRecords
}

export type ProxyState = {
  proxyData: {
    groups: ProxyGroupOption[]
    groupMap: Record<string, ProxyGroupOption>
    records: CurrentProxyRecords
  }
  selection: {
    group: string
    proxy: string
  }
  displayProxy: any
  resolvedPath: string[]
}

export interface UseCurrentProxyDataProps {
  proxies: CurrentProxySource
  currentProfileId: string | null
  defaultLatencyTimeout: number
  refreshProxy: () => void
}

export const INITIAL_PROXY_STATE: ProxyState = {
  proxyData: {
    groups: [],
    groupMap: {},
    records: {},
  },
  selection: {
    group: '',
    proxy: '',
  },
  displayProxy: null,
  resolvedPath: [],
}

export const KIND_WEIGHT: Record<ProxyOption['kind'], number> = {
  manual: 0,
  strategy: 1,
}

export function extractProxyNames(
  items?: Array<string | { name?: string }> | null,
) {
  return (items || [])
    .map((item) =>
      typeof item === 'string'
        ? normalizePolicyName(item)
        : normalizePolicyName(item?.name),
    )
    .filter((value): value is string => value.length > 0)
}

export function resolveLeafProxy(records: CurrentProxyRecords, name: string) {
  const resolved = resolveProxyPath(records, name)
  const leafName = resolved.leafName || name

  return {
    displayProxy: records?.[leafName] || records?.[name] || null,
    resolvedPath: resolved.path,
  }
}

export function buildResolvedPath(groupName: string | null, path: string[]) {
  if (!groupName) return path

  return [groupName, ...path.filter((name) => name !== groupName)]
}

export function buildSelectionSnapshot(
  records: CurrentProxyRecords,
  groupName: string | null,
  proxyName: string,
) {
  const resolved = resolveLeafProxy(records, proxyName)

  return {
    displayProxy: resolved.displayProxy,
    resolvedPath: buildResolvedPath(groupName, resolved.resolvedPath),
  }
}

export function pickVisibleProxyName(
  names: string[],
  records: CurrentProxyRecords,
  ...candidates: Array<string | null | undefined>
) {
  for (const candidate of candidates) {
    const normalizedCandidate = normalizePolicyName(candidate)
    if (!normalizedCandidate) continue

    const pickedCandidate = pickPreferredProxyNameFromNames({
      names,
      records,
      candidateName: normalizedCandidate,
    })

    if (pickedCandidate === normalizedCandidate) {
      return pickedCandidate
    }
  }

  return pickPreferredProxyNameFromNames({
    names,
    records,
  })
}
