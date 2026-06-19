import { useProxyDelayState } from '@/hooks/network'
import {
  categorizeProxyGroup,
  isProxyGroupItem,
} from '@/services/proxy-display'
import type { IProxyGroupItem, IProxyItem } from '@/types/proxy'
interface UseProxyCardStateOptions {
  group: IProxyGroupItem
  proxy: IProxyItem
  onConfigure?: (group: IProxyGroupItem) => void
}

export function useProxyCardState(options: UseProxyCardStateOptions) {
  const { group, proxy, onConfigure } = options
  const { delayValue, isPreset, timeout, onDelay } = useProxyDelayState(
    proxy,
    group.name,
  )

  const configurableStrategyGroup =
    onConfigure && isProxyGroupItem(proxy) && categorizeProxyGroup(proxy) === 'strategy'
      ? proxy
      : null

  return {
    configurableStrategyGroup,
    delayValue,
    isPreset,
    onDelay,
    timeout,
  }
}
