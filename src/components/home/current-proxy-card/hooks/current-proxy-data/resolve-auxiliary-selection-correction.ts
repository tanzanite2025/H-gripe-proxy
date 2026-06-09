import { isAuxiliarySelectionName } from '@/services/proxy-display'

import { normalizePolicyName } from '../../utils/proxy-selection'
import {
  extractProxyNames,
  pickVisibleProxyName,
  type CurrentProxySource,
  type ProxyState,
} from './shared'

interface ResolveAuxiliarySelectionCorrectionOptions {
  isGlobalMode: boolean
  proxies: CurrentProxySource
  state: ProxyState
}

interface AuxiliarySelectionCorrection {
  currentNow: string
  signature: string
  targetProxy: string
}

export function resolveAuxiliarySelectionCorrection({
  isGlobalMode,
  proxies,
  state,
}: ResolveAuxiliarySelectionCorrectionOptions):
  | AuxiliarySelectionCorrection
  | null {
  if (!proxies.records || !state.selection.group) {
    return null
  }

  const currentGroup = isGlobalMode
    ? proxies.global
    : state.proxyData.groupMap[state.selection.group]

  const currentNow = normalizePolicyName(currentGroup?.now)
  if (!currentNow || !isAuxiliarySelectionName(currentNow, state.proxyData.records)) {
    return null
  }

  const targetProxy = pickVisibleProxyName(
    extractProxyNames(currentGroup?.all),
    state.proxyData.records,
    state.selection.proxy,
    currentNow,
  )

  if (!targetProxy || targetProxy === currentNow) {
    return null
  }

  return {
    currentNow,
    signature: `${state.selection.group}:${currentNow}->${targetProxy}`,
    targetProxy,
  }
}
