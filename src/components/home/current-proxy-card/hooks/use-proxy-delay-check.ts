import { useLockFn } from 'ahooks'

import { resolveDelayTimeout } from '@/services/delay-config'
import { debugLog } from '@/utils/misc'

import type { CurrentProxySource } from './current-proxy-data/shared'
import { buildDelayCheckTargets } from './proxy-delay-check/build-delay-check-targets'
import { runGroupDelayCheck } from './proxy-delay-check/run-group-delay-check'
import { runProviderHealthChecks } from './proxy-delay-check/run-provider-health-checks'

interface UseProxyDelayCheckProps {
  currentGroup: string
  defaultLatencyTimeout: number
  proxies: CurrentProxySource | undefined
  proxyRecords: Record<string, any>
  refreshProxy: () => void
  onDelayCheckComplete?: () => void
}

export const useProxyDelayCheck = ({
  currentGroup,
  defaultLatencyTimeout,
  proxies,
  proxyRecords,
  refreshProxy,
  onDelayCheckComplete,
}: UseProxyDelayCheckProps) => {
  const handleCheckAllDelay = useLockFn(async (isGlobalMode: boolean) => {
    const groupName = currentGroup
    if (!groupName) return

    debugLog(`[CurrentProxyCard] Start delay check, group: ${groupName}`)

    const timeout = resolveDelayTimeout(defaultLatencyTimeout)
    const { providerNames, proxyNames } = buildDelayCheckTargets({
      isGlobalMode,
      proxies,
      proxyRecords,
    })

    if (!isGlobalMode || !proxies?.global) {
      debugLog(
        '[CurrentProxyCard] Rule mode batch delay check requires group proxy data',
      )
    }

    debugLog(
      `[CurrentProxyCard] Proxy count: ${proxyNames.length}, provider count: ${providerNames.length}`,
    )

    if (providerNames.length > 0) {
      debugLog('[CurrentProxyCard] Start provider health checks')
      await runProviderHealthChecks(providerNames)
    }

    if (proxyNames.length > 0) {
      await runGroupDelayCheck({
        groupName,
        proxyNames,
        timeout,
      })
    }

    refreshProxy()
    onDelayCheckComplete?.()
  })

  return {
    handleCheckAllDelay,
  }
}
