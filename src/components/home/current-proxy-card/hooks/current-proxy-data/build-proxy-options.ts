import delayManager from '@/services/delay'
import { resolveDelayTimeout } from '@/services/delay-config'
import { buildProxyDisplayOptionsFromNames } from '@/services/proxy-display'

import { categorizeDelay } from '../../utils/proxy-selection'

import {
  KIND_WEIGHT,
  type ProxyOption,
  type ProxySortType,
} from './shared'

interface BuildCurrentProxyOptionsOptions {
  defaultLatencyTimeout: number
  delaySortRefresh: number
  groupMap: Record<string, { all: string[] }>
  records: Record<string, any>
  selectionGroup: string
  sortType: ProxySortType
}

function isVisibleProxyOption(option: { kind: string }): option is ProxyOption {
  return option.kind === 'manual' || option.kind === 'strategy'
}

function sortProxyOptions(
  options: ProxyOption[],
  sortType: ProxySortType,
  selectionGroup: string,
  records: Record<string, any>,
  defaultLatencyTimeout: number,
  delaySortRefresh: number,
) {
  const list = [...options]

  if (sortType === 0) {
    return list.sort((a, b) => KIND_WEIGHT[a.kind] - KIND_WEIGHT[b.kind])
  }

  if (sortType === 1) {
    const effectiveTimeout = resolveDelayTimeout(defaultLatencyTimeout)

    list.sort((a, b) => {
      const kindDiff = KIND_WEIGHT[a.kind] - KIND_WEIGHT[b.kind]
      if (kindDiff !== 0) return kindDiff

      const recordA = records[a.name]
      const recordB = records[b.name]

      const [ar, av] = recordA
        ? categorizeDelay(
            delayManager.getDelayFix(recordA, selectionGroup),
            effectiveTimeout,
          )
        : [6, Number.MAX_SAFE_INTEGER]
      const [br, bv] = recordB
        ? categorizeDelay(
            delayManager.getDelayFix(recordB, selectionGroup),
            effectiveTimeout,
          )
        : [6, Number.MAX_SAFE_INTEGER]

      if (ar !== br) return ar - br
      if (av !== bv) return av - bv
      return delaySortRefresh >= 0 ? a.name.localeCompare(b.name) : 0
    })

    return list
  }

  list.sort((a, b) => {
    const kindDiff = KIND_WEIGHT[a.kind] - KIND_WEIGHT[b.kind]
    if (kindDiff !== 0) return kindDiff
    return a.name.localeCompare(b.name)
  })

  return list
}

export function buildCurrentProxyOptions({
  defaultLatencyTimeout,
  delaySortRefresh,
  groupMap,
  records,
  selectionGroup,
  sortType,
}: BuildCurrentProxyOptionsOptions) {
  const names = groupMap[selectionGroup]?.all || []

  if (!names.length) {
    return []
  }

  const options = buildProxyDisplayOptionsFromNames({
    names,
    records,
  }).filter(isVisibleProxyOption)

  return sortProxyOptions(
    options,
    sortType,
    selectionGroup,
    records,
    defaultLatencyTimeout,
    delaySortRefresh,
  )
}
