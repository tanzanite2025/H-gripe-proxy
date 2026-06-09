import type { CurrentEgressIdentity } from '@/services/cmds/diagnostics'

import {
  isAuxiliaryGroupType,
  isProxyGroupItem,
  isStrategyGroupType,
} from './classification'
import { normalizeName } from './names'
import type { ProxyPathAnalysis } from './types'

export const getIdentityProxyChain = (
  identity?: CurrentEgressIdentity | null,
): string[] => {
  if (!identity || identity.source !== 'mihomoEgressStatus') {
    return []
  }

  const chain = Array.isArray(identity.proxy_chain)
    ? identity.proxy_chain
        .map((item) => normalizeName(item))
        .filter((item) => item.length > 0)
    : []

  if (chain.length > 0) {
    return chain
  }

  const proxyName = normalizeName(identity.proxy_name)
  return proxyName ? [proxyName] : []
}

export const analyzeProxyPath = (
  path: string[],
  records?: Record<string, IProxyItem>,
): ProxyPathAnalysis => {
  const uniquePath = path
    .map((item) => normalizeName(item))
    .filter((item) => item.length > 0)

  return uniquePath.reduce<ProxyPathAnalysis>(
    (analysis, name) => {
      const record = records?.[name]

      if (record) {
        if (isStrategyGroupType(record.type)) {
          analysis.hasStrategyGroup = true
        }

        if (isAuxiliaryGroupType(record.type)) {
          analysis.hasAuxiliaryGroup = true
        }
      }

      if (analysis.path.includes(name)) {
        analysis.cycleDetected = true
        return analysis
      }

      analysis.path.push(name)
      analysis.leafName = name
      return analysis
    },
    {
      path: [],
      leafName: null,
      hasStrategyGroup: false,
      hasAuxiliaryGroup: false,
      cycleDetected: false,
    },
  )
}

export const resolveProxyPath = (
  records: Record<string, IProxyItem> | undefined,
  startName?: string | null,
): ProxyPathAnalysis => {
  const path: string[] = []
  const visited = new Set<string>()
  let currentName = normalizeName(startName)
  let hasStrategyGroup = false
  let hasAuxiliaryGroup = false
  let cycleDetected = false

  while (currentName) {
    if (visited.has(currentName)) {
      cycleDetected = true
      break
    }

    path.push(currentName)
    visited.add(currentName)

    const currentRecord = records?.[currentName]
    if (currentRecord) {
      if (isStrategyGroupType(currentRecord.type)) {
        hasStrategyGroup = true
      }

      if (isAuxiliaryGroupType(currentRecord.type)) {
        hasAuxiliaryGroup = true
      }
    }

    if (!isProxyGroupItem(currentRecord) || !currentRecord.now) {
      break
    }

    const nextName = normalizeName(currentRecord.now)
    if (!nextName || nextName === currentName) {
      break
    }

    currentName = nextName
  }

  return {
    path,
    leafName: path[path.length - 1] || null,
    hasStrategyGroup,
    hasAuxiliaryGroup,
    cycleDetected,
  }
}

