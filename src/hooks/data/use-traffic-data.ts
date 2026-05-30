import { useEffect } from 'react'
import { MihomoWebSocket, Traffic } from 'tauri-plugin-mihomo-api'

import { useMihomoWsSubscription } from '@/hooks/network'
import { useTrafficMonitorEnhanced } from '@/hooks/network'

const FALLBACK_TRAFFIC: Traffic = { up: 0, down: 0 }
const DUPLICATE_TRAFFIC_WINDOW_MS = 50

let lastTrafficSignature = ''
let lastTrafficTimestamp = 0

const shouldSkipDuplicateTraffic = (traffic: Traffic) => {
  const now = Date.now()
  const signature = `${traffic.up}:${traffic.down}`

  if (
    signature === lastTrafficSignature &&
    now - lastTrafficTimestamp <= DUPLICATE_TRAFFIC_WINDOW_MS
  ) {
    return true
  }

  lastTrafficSignature = signature
  lastTrafficTimestamp = now
  return false
}

export const useTrafficData = (options?: { enabled?: boolean }) => {
  const enabled = options?.enabled ?? true

  const {
    graphData: { appendData },
  } = useTrafficMonitorEnhanced({ subscribe: false, enabled })
  const { response, refresh } = useMihomoWsSubscription<ITrafficItem>({
    storageKey: 'mihomo_traffic_date',
    buildSubscriptKey: (date) => `getClashTraffic-${date}`,
    fallbackData: FALLBACK_TRAFFIC,
    connect: () => MihomoWebSocket.connect_traffic(),
    throttleMs: 200,
    setupHandlers: ({ next, scheduleReconnect }) => ({
      handleMessage: (data) => {
        if (data.startsWith('Websocket error')) {
          next(data, FALLBACK_TRAFFIC)
          void scheduleReconnect()
          return
        }

        try {
          const parsed = JSON.parse(data) as Traffic
          if (shouldSkipDuplicateTraffic(parsed)) {
            return
          }
          next(null, parsed)
        } catch (error) {
          next(error, FALLBACK_TRAFFIC)
        }
      },
    }),
  })

  const trafficData = response.data
  useEffect(() => {
    if (enabled && trafficData) {
      appendData(trafficData)
    }
  }, [enabled, trafficData, appendData])

  return { response, refreshGetClashTraffic: refresh }
}
