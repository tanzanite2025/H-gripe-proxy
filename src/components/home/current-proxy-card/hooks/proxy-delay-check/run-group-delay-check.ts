import delayManager, { delayRuntimeGroup } from '@/services/delay'
import { debugLog } from '@/utils/misc'

interface RunGroupDelayCheckOptions {
  groupName: string
  proxyNames: string[]
  timeout: number
}

export async function runGroupDelayCheck({
  groupName,
  proxyNames,
  timeout,
}: RunGroupDelayCheckOptions) {
  if (!proxyNames.length) {
    return
  }

  const url = delayManager.getUrl(groupName)
  debugLog(`[CurrentProxyCard] Test URL: ${url}, timeout: ${timeout}ms`)

  try {
    proxyNames.forEach((name) => {
      delayManager.setDelay(name, groupName, -2)
    })

    const result = await delayRuntimeGroup(groupName, url, timeout, false)
    debugLog(
      `[CurrentProxyCard] Group delay result count: ${Object.keys(result || {}).length}`,
    )

    proxyNames.forEach((name) => {
      delayManager.setDelay(name, groupName, result?.[name] ?? 0)
    })

    debugLog(`[CurrentProxyCard] Delay check finished, group: ${groupName}`)
  } catch (error) {
    console.warn(
      `[CurrentProxyCard] Group delay failed, fallback to per-proxy checks, group: ${groupName}`,
      error,
    )

    try {
      await delayManager.checkListDelay(proxyNames, groupName, timeout)
    } catch (fallbackError) {
      console.error(
        `[CurrentProxyCard] Fallback delay check failed, group: ${groupName}`,
        fallbackError,
      )
    }
  }
}
